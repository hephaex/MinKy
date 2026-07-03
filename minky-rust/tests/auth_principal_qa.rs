//! Adversarial QA (red-team) for `validate_api_key` + `AuthPrincipal` (G002 WP1).
//!
//! Purpose: try to BREAK the API-key auth contract with a real PostgreSQL DB,
//! not to confirm the happy path.
//!
//! Contract under attack (pending-approval.md WP1 / §2.5):
//!   - `validate_api_key`: SHA256 hash lookup, gated by
//!     `is_active = true AND (expires_at IS NULL OR expires_at > NOW())`;
//!     refreshes `last_used_at` non-blockingly on a match; returns `Ok(None)`
//!     for any non-match (never an error / never a panic).
//!   - `AuthPrincipal` extraction order: BYPASS short-circuit -> Bearer JWT ->
//!     `X-API-Key` -> 401.
//!
//! Safety / CI: every test connects via `G002_QA_DATABASE_URL` (preferred) or
//! `DATABASE_URL`. When neither is set the test is a no-op that PASSES, so CI
//! without a database stays green. Schema is provisioned idempotently
//! (`users`, `api_keys` only — no pgvector) so the suite is self-contained.
//!
//! The BYPASS_AUTH=true case is exercised in a *child process* (see
//! `bypass_auth_allows_unauthenticated_via_subprocess`) because
//! `is_bypass_auth()` caches the env var in a process-global `OnceLock`;
//! toggling it inside the shared test process would poison sibling tests.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use minky::services::SecurityService;

// ---------------------------------------------------------------------------
// Environment / connection helpers
// ---------------------------------------------------------------------------

fn db_url() -> Option<String> {
    std::env::var("G002_QA_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .filter(|s| !s.trim().is_empty())
}

/// Connect to the QA database, or return `None` to signal "skip this test".
async fn connect() -> Option<PgPool> {
    let url = db_url()?;
    match PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await
    {
        Ok(pool) => {
            ensure_schema(&pool).await;
            Some(pool)
        }
        Err(e) => {
            eprintln!("SKIP: cannot connect to QA database: {e}");
            None
        }
    }
}

/// Idempotently provision the minimal schema the auth path touches.
///
/// Concurrent `CREATE TABLE IF NOT EXISTS` on a virgin database races inside
/// PostgreSQL (duplicate pg_type key, SQLSTATE 23505), so provisioning is
/// serialized behind a per-process advisory lock: the winner creates, the
/// rest wait and observe the committed schema.
async fn ensure_schema(pool: &PgPool) {
    // Session-scoped advisory lock keyed on an arbitrary constant.
    let guard = pool.acquire().await.expect("acquire provisioning conn");
    let mut guard = guard;
    sqlx::query("SELECT pg_advisory_lock(730001)")
        .execute(&mut *guard)
        .await
        .expect("take provisioning advisory lock");

    provision(pool).await;

    sqlx::query("SELECT pg_advisory_unlock(730001)")
        .execute(&mut *guard)
        .await
        .expect("release provisioning advisory lock");
}

async fn provision(pool: &PgPool) {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) NOT NULL UNIQUE,
            username VARCHAR(50) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL DEFAULT '',
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .expect("create users table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS api_keys (
            id BIGSERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            key_prefix VARCHAR(16) NOT NULL,
            key_hash VARCHAR(128) NOT NULL,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            permissions JSONB NOT NULL DEFAULT '[]'::jsonb,
            last_used_at TIMESTAMP WITH TIME ZONE,
            expires_at TIMESTAMP WITH TIME ZONE,
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
        )"#,
    )
    .execute(pool)
    .await
    .expect("create api_keys table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash)")
        .execute(pool)
        .await
        .expect("create key_hash index");
}

// ---------------------------------------------------------------------------
// Seed / hash helpers
// ---------------------------------------------------------------------------

static SEQ: AtomicU64 = AtomicU64::new(0);

/// Per-invocation unique token so parallel tests never collide on unique cols.
fn unique() -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}_{n}")
}

/// Mirror of the private `hash_api_key` in `security_service.rs`
/// (SHA256 hex of the raw key bytes).
fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn make_user(pool: &PgPool) -> i32 {
    let u = unique();
    let (id,): (i32,) = sqlx::query_as(
        "INSERT INTO users (email, username) VALUES ($1, $2) RETURNING id",
    )
    .bind(format!("qa+{u}@g002.local"))
    .bind(format!("qa_{u}"))
    .fetch_one(pool)
    .await
    .expect("insert qa user");
    id
}

/// Insert an api_keys row keyed on `raw_key` (its SHA256 is what the service
/// looks up). Returns the row id.
async fn insert_key(
    pool: &PgPool,
    user_id: i32,
    name: &str,
    raw_key: &str,
    is_active: bool,
    expires_at: Option<DateTime<Utc>>,
) -> i64 {
    let key_hash = hash_key(raw_key);
    let prefix: String = raw_key.chars().take(8).collect();
    let (id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO api_keys
            (name, key_prefix, key_hash, user_id, permissions, expires_at, is_active)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, $5, $6)
           RETURNING id"#,
    )
    .bind(name)
    .bind(prefix)
    .bind(&key_hash)
    .bind(user_id)
    .bind(expires_at)
    .bind(is_active)
    .fetch_one(pool)
    .await
    .expect("insert api key");
    id
}

async fn last_used_at(pool: &PgPool, key_id: i64) -> Option<DateTime<Utc>> {
    let (ts,): (Option<DateTime<Utc>>,) =
        sqlx::query_as("SELECT last_used_at FROM api_keys WHERE id = $1")
            .bind(key_id)
            .fetch_one(pool)
            .await
            .expect("select last_used_at");
    ts
}

// ===========================================================================
// CASE 1 — valid key returns identity + updates last_used_at (poll)
// ===========================================================================

#[tokio::test]
async fn case1_valid_key_returns_identity_and_refreshes_last_used_at() {
    let Some(pool) = connect().await else { return };
    let uid = make_user(&pool).await;
    let raw = format!("mk_valid_{}", unique());
    let key_id = insert_key(&pool, uid, "ci-bot", &raw, true, None).await;

    // Precondition: last_used_at starts NULL.
    assert!(
        last_used_at(&pool, key_id).await.is_none(),
        "expected last_used_at NULL before validation"
    );

    let svc = SecurityService::new(pool.clone());
    let identity = svc
        .validate_api_key(&raw)
        .await
        .expect("validate must not error")
        .expect("valid key must resolve to Some(identity)");

    assert_eq!(identity.key_id, key_id, "key_id mismatch");
    assert_eq!(identity.user_id, uid, "user_id mismatch");
    assert_eq!(identity.name, "ci-bot", "name mismatch");

    // last_used_at is refreshed in a fire-and-forget task; poll for it.
    let mut refreshed = None;
    for _ in 0..50 {
        if let Some(ts) = last_used_at(&pool, key_id).await {
            refreshed = Some(ts);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        refreshed.is_some(),
        "last_used_at was never refreshed after a successful validation"
    );
}

// ===========================================================================
// CASE 2 — malformed / hostile raw keys return Ok(None), never error/panic
// ===========================================================================

#[tokio::test]
async fn case2_invalid_keys_return_none_without_error() {
    let Some(pool) = connect().await else { return };
    let _uid = make_user(&pool).await; // ensure at least one user/table exists
    let svc = SecurityService::new(pool.clone());

    let one_mb = "A".repeat(1024 * 1024);
    let cases: Vec<(&str, String)> = vec![
        ("random", format!("nonexistent_{}", unique())),
        ("empty", String::new()),
        ("one_megabyte", one_mb),
        ("unicode", "🔑키値😀ключ".to_string()),
        ("embedded_nul", "abc\0def".to_string()),
        ("whitespace_only", "   \t\n ".to_string()),
        ("null_literal_text", "null".to_string()),
    ];

    for (label, key) in cases {
        let res = svc.validate_api_key(&key).await;
        assert!(
            res.is_ok(),
            "[{label}] validate_api_key must not return Err, got {res:?}"
        );
        assert!(
            res.unwrap().is_none(),
            "[{label}] non-existent key must resolve to None"
        );
    }
}

// ===========================================================================
// CASE 3 — revoked key (is_active = false) -> None
// ===========================================================================

#[tokio::test]
async fn case3_revoked_key_returns_none() {
    let Some(pool) = connect().await else { return };
    let uid = make_user(&pool).await;
    let raw = format!("mk_revoked_{}", unique());
    insert_key(&pool, uid, "revoked", &raw, false, None).await;

    let svc = SecurityService::new(pool.clone());
    let res = svc.validate_api_key(&raw).await.expect("no error");
    assert!(res.is_none(), "revoked (is_active=false) key must not authenticate");
}

// ===========================================================================
// CASE 4 — expiry window: past -> None, future -> Some, NULL -> Some
// ===========================================================================

#[tokio::test]
async fn case4_expiry_window_enforced() {
    let Some(pool) = connect().await else { return };
    let uid = make_user(&pool).await;
    let svc = SecurityService::new(pool.clone());

    // (a) expired in the past -> None
    let past_raw = format!("mk_past_{}", unique());
    insert_key(&pool, uid, "past", &past_raw, true, Some(Utc::now() - Duration::hours(1))).await;
    assert!(
        svc.validate_api_key(&past_raw).await.expect("no error").is_none(),
        "past expires_at must reject"
    );

    // (b) expires in the future -> Some
    let future_raw = format!("mk_future_{}", unique());
    let fid = insert_key(&pool, uid, "future", &future_raw, true, Some(Utc::now() + Duration::hours(1))).await;
    let fut = svc.validate_api_key(&future_raw).await.expect("no error");
    assert!(fut.is_some(), "future expires_at must authenticate");
    assert_eq!(fut.unwrap().key_id, fid);

    // (c) NULL expires_at -> Some (never expires)
    let never_raw = format!("mk_never_{}", unique());
    let nid = insert_key(&pool, uid, "never", &never_raw, true, None).await;
    let never = svc.validate_api_key(&never_raw).await.expect("no error");
    assert!(never.is_some(), "NULL expires_at must authenticate");
    assert_eq!(never.unwrap().key_id, nid);
}

// ===========================================================================
// CASE 5 — double-hash: sending the stored key_hash itself must NOT authenticate
// ===========================================================================

#[tokio::test]
async fn case5_sending_stored_hash_does_not_authenticate() {
    let Some(pool) = connect().await else { return };
    let uid = make_user(&pool).await;
    let raw = format!("mk_secret_{}", unique());
    insert_key(&pool, uid, "double-hash", &raw, true, None).await;

    let stored_hash = hash_key(&raw); // == api_keys.key_hash

    let svc = SecurityService::new(pool.clone());

    // Sanity: the real raw key authenticates.
    assert!(
        svc.validate_api_key(&raw).await.expect("no error").is_some(),
        "raw key should authenticate (sanity)"
    );
    // Attack: the stored hash value, used as the raw key, must be re-hashed
    // (SHA256(SHA256(raw)) != SHA256(raw)) and therefore fail.
    assert!(
        svc.validate_api_key(&stored_hash).await.expect("no error").is_none(),
        "stored key_hash must NOT be accepted as a raw key (double-hash defense)"
    );
}

// ===========================================================================
// CASE 6 — SQL injection payloads in the key slot -> None, parameterized-safe
// ===========================================================================

#[tokio::test]
async fn case6_sql_injection_payloads_are_inert() {
    let Some(pool) = connect().await else { return };
    let uid = make_user(&pool).await;
    // A real active key exists; injection must NOT surface it.
    let real = format!("mk_real_{}", unique());
    insert_key(&pool, uid, "real", &real, true, None).await;

    let svc = SecurityService::new(pool.clone());

    let payloads = [
        "' OR 1=1 --",
        "' OR '1'='1",
        "'; DROP TABLE api_keys; --",
        "' UNION SELECT id, user_id, name FROM api_keys --",
        "\\'; SELECT 1; --",
        "' OR is_active = true --",
    ];
    for p in payloads {
        let res = svc.validate_api_key(p).await;
        assert!(res.is_ok(), "injection [{p}] must not error: {res:?}");
        assert!(res.unwrap().is_none(), "injection [{p}] must not authenticate");
    }

    // Table survived the DROP attempt: the real key still validates.
    assert!(
        svc.validate_api_key(&real).await.expect("no error").is_some(),
        "api_keys table/row must survive injection attempts"
    );
}

// ===========================================================================
// Router-level AuthPrincipal tests
// ===========================================================================

mod router {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::get,
        Json, Router,
    };
    use minky::config::Config;
    use minky::middleware::AuthPrincipal;
    use minky::services::{EmbeddingConfig, EmbeddingService, WebSocketManager};
    use minky::AppState;
    use secrecy::SecretString;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_config() -> Config {
        Config {
            environment: "development".to_string(),
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: db_url().unwrap_or_else(|| "postgres://localhost/none".to_string()),
            database_max_connections: 3,
            database_min_connections: 1,
            database_acquire_timeout_secs: 30,
            database_max_lifetime_secs: 1800,
            database_idle_timeout_secs: 600,
            jwt_secret: SecretString::from("g002-qa-secret"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            upstage_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
            cors_allowed_origins: "http://localhost:3000".to_string(),
            local_embedding_enabled: false,
            vault_watch: Default::default(),
        }
    }

    fn build_state(pool: PgPool) -> AppState {
        let embedding = Arc::new(EmbeddingService::new(pool.clone(), EmbeddingConfig::default()));
        AppState {
            db: pool,
            config: test_config(),
            ws_manager: Arc::new(WebSocketManager::new()),
            embedding_service: embedding,
            vault_watcher: Arc::new(tokio::sync::Mutex::new(None)),
            redis_client: None,
            http_client: reqwest::Client::new(),
            korean_tokenizer: None,
        }
    }

    async fn protected(principal: AuthPrincipal) -> impl IntoResponse {
        Json(json!({ "user_id": principal.effective_user_id() }))
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/protected", get(protected))
            .with_state(state)
    }

    async fn send(app: Router, req: Request<Body>) -> (StatusCode, Value) {
        let resp = app.oneshot(req).await.expect("router oneshot");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    // (a) anonymous request -> 401
    #[tokio::test]
    async fn router_anonymous_request_is_401() {
        let Some(pool) = connect().await else { return };
        let state = build_state(pool);
        let req = Request::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();
        let (status, _body) = send(app(state), req).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "anonymous request to AuthPrincipal route must be 401"
        );
    }

    // (b) valid X-API-Key -> 200 with the key's user_id
    #[tokio::test]
    async fn router_valid_api_key_is_200() {
        let Some(pool) = connect().await else { return };
        let uid = make_user(&pool).await;
        let raw = format!("mk_router_{}", unique());
        insert_key(&pool, uid, "router-bot", &raw, true, None).await;

        let state = build_state(pool);
        let req = Request::builder()
            .uri("/protected")
            .header("x-api-key", &raw)
            .body(Body::empty())
            .unwrap();
        let (status, body) = send(app(state), req).await;
        assert_eq!(status, StatusCode::OK, "valid X-API-Key must be 200");
        assert_eq!(
            body["user_id"], uid,
            "service principal must resolve to api_keys.user_id"
        );
    }

    // (b2) revoked X-API-Key at router level -> 401
    #[tokio::test]
    async fn router_revoked_api_key_is_401() {
        let Some(pool) = connect().await else { return };
        let uid = make_user(&pool).await;
        let raw = format!("mk_router_revoked_{}", unique());
        insert_key(&pool, uid, "router-revoked", &raw, false, None).await;

        let state = build_state(pool);
        let req = Request::builder()
            .uri("/protected")
            .header("x-api-key", &raw)
            .body(Body::empty())
            .unwrap();
        let (status, _b) = send(app(state), req).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "revoked X-API-Key must be rejected at the router"
        );
    }

    // ---- BYPASS_AUTH demonstration --------------------------------------
    //
    // `is_bypass_auth()` caches BYPASS_AUTH in a process-global OnceLock, so we
    // cannot flip it inside the shared test process without poisoning siblings.
    // The parent test re-execs THIS binary, running only the `#[ignore]`d child
    // below, with BYPASS_AUTH=true. The child proves an anonymous request is
    // wrongly ADMITTED as admin (user_id=1) — i.e. why WP6 must force
    // BYPASS_AUTH=false in production.

    #[tokio::test]
    #[ignore = "child process only; driven by the subprocess parent test"]
    async fn bypass_child_anonymous_is_admitted() {
        assert_eq!(
            std::env::var("BYPASS_AUTH").ok().as_deref(),
            Some("true"),
            "child must run with BYPASS_AUTH=true"
        );
        let pool = connect().await.expect("child requires a DB connection");
        let state = build_state(pool);
        let req = Request::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();
        let (status, body) = send(app(state), req).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "with BYPASS_AUTH=true an anonymous request is admitted"
        );
        assert_eq!(
            body["user_id"], 1,
            "bypass admits as service admin (user id 1)"
        );
    }

    #[test]
    fn bypass_auth_allows_unauthenticated_via_subprocess() {
        let Some(url) = db_url() else { return };
        let exe = std::env::current_exe().expect("current test exe");
        let output = std::process::Command::new(exe)
            .args([
                "--exact",
                "router::bypass_child_anonymous_is_admitted",
                "--ignored",
                "--nocapture",
                "--test-threads",
                "1",
            ])
            .env("BYPASS_AUTH", "true")
            .env("G002_QA_DATABASE_URL", url)
            .output()
            .expect("spawn child test process");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "BYPASS child failed (this proves the bypass path):\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
        );
        assert!(
            stdout.contains("1 passed") || stdout.contains("test result: ok"),
            "expected child to report a passing bypass test; stdout:\n{stdout}"
        );
    }
}
