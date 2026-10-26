//! Adversarial QA (red-team) for the `AuthPrincipal` path on the document
//! upload/update routes (G008 completion QA lane).
//!
//! Purpose: try to BREAK the `X-API-Key` (`AuthPrincipal::Service`) contract on
//! `POST /api/documents/upload` and `PUT /api/documents/{id}`, not to confirm
//! the happy path. Companion to `upload_update_auth_qa.rs` (which covers the
//! anonymous-401 + valid-key-succeeds cases).
//!
//! Attacks under test:
//!   1. Revoked (`is_active = false`) and expired (`expires_at < NOW()`) API
//!      keys must be rejected with `401` on both upload and update.
//!   2. Bogus / arbitrary `X-API-Key` values must yield `401` (fail-closed,
//!      never a panic / 500).
//!   3. Tenant isolation: an uploaded document's `user_id` is exactly the
//!      key's `api_keys.user_id`; a service principal for user B cannot mutate
//!      user A's document (route returns `404`, document stays unchanged).
//!   4. Credential precedence: when a valid Bearer JWT (user A) and a valid
//!      `X-API-Key` (user B) are sent together, the JWT wins (extraction order
//!      BYPASS -> Bearer JWT -> X-API-Key), so the document is created under
//!      user A.
//!
//! Safety / CI: every test connects via `G002_QA_DATABASE_URL` (preferred) or
//! `DATABASE_URL`. When neither is set — or the DB cannot be reached / migrated
//! — the test is a no-op that PASSES, so CI without a database stays green.
//! Schema is provisioned by running the crate's real migrations, so this suite
//! expects a dedicated / freshly-migrated integration Postgres (pgvector +
//! pgcrypto), exactly like a production `minky_rust_db`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;

use minky::config::Config;
use minky::routes::documents;
use minky::services::{EmbeddingConfig, EmbeddingService, WebSocketManager};
use minky::AppState;

// ---------------------------------------------------------------------------
// Environment / connection / schema
// ---------------------------------------------------------------------------

const JWT_SECRET: &str = "g008-qa2-secret";

fn db_url() -> Option<String> {
    std::env::var("G002_QA_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .filter(|s| !s.trim().is_empty())
}

/// Connect + migrate, or return `None` to signal "skip this test".
async fn setup() -> Option<PgPool> {
    let url = db_url()?;
    let pool = match PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SKIP: cannot connect to QA database: {e}");
            return None;
        }
    };
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        eprintln!("SKIP: cannot apply migrations to QA database: {e}");
        return None;
    }
    Some(pool)
}

static SEQ: AtomicU64 = AtomicU64::new(0);

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

async fn make_user(pool: &PgPool) -> (i32, String) {
    let u = unique();
    let email = format!("g008q2+{u}@qa.local");
    let (id,): (i32,) =
        sqlx::query_as("INSERT INTO users (email, username, password_hash) VALUES ($1, $2, '') RETURNING id")
            .bind(&email)
            .bind(format!("g008q2_{u}"))
            .fetch_one(pool)
            .await
            .expect("insert qa user");
    (id, email)
}

/// Insert an `api_keys` row keyed on `raw_key` with explicit activity / expiry,
/// returning the raw key string that a client would present.
async fn make_api_key(
    pool: &PgPool,
    user_id: i32,
    is_active: bool,
    expires_at: Option<DateTime<Utc>>,
) -> String {
    let raw = format!("mk_g008q2_{}", unique());
    let key_hash = hash_key(&raw);
    let prefix: String = raw.chars().take(8).collect();
    sqlx::query(
        r#"INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, is_active, expires_at)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, $5, $6)"#,
    )
    .bind("g008q2-bot")
    .bind(prefix)
    .bind(&key_hash)
    .bind(user_id)
    .bind(is_active)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("insert api key");
    raw
}

/// Seed one document owned by `user_id`, returning its UUID (as string).
async fn seed_document(pool: &PgPool, user_id: i32, title: &str) -> String {
    let (id,): (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO documents (title, content, user_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(title)
    .bind("original body")
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document");
    id.to_string()
}

/// Mint a valid `access` JWT for `user_id` using the same secret the test state
/// is built with, matching the `Claims` shape decoded by `AuthUser`.
fn mint_jwt(user_id: i32, email: &str) -> String {
    let now = Utc::now();
    let claims = json!({
        "sub": user_id.to_string(),
        "email": email,
        "role": "user",
        "token_type": "access",
        "exp": (now + Duration::hours(1)).timestamp(),
        "iat": now.timestamp(),
    });
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

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
        jwt_secret: SecretString::from(JWT_SECRET),
        jwt_expiration_hours: 24,
        opensearch_url: None,
        openai_api_key: None,
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
    // Guard: the JWT-precedence attack is only meaningful when the state signs
    // with the same secret this suite mints tokens with.
    debug_assert_eq!(test_config().jwt_secret.expose_secret(), JWT_SECRET);
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

fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api/documents", documents::routes())
        .with_state(state)
}

/// Build a single-field ("file") multipart body.
fn multipart_body(filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----G008q2Boundary9f2a";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
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

/// Fetch `(user_id, title)` for a document, or `None` if it does not exist.
async fn doc_owner_title(pool: &PgPool, doc_id: &str) -> Option<(i32, String)> {
    sqlx::query_as("SELECT user_id, title FROM documents WHERE id = $1::uuid")
        .bind(doc_id)
        .fetch_optional(pool)
        .await
        .expect("fetch document owner/title")
}

// ===========================================================================
// ATTACK 1 — revoked / expired API keys are rejected (401) on upload & update
// ===========================================================================

#[tokio::test]
async fn revoked_key_upload_is_401() {
    let Some(pool) = setup().await else { return };
    let (uid, _email) = make_user(&pool).await;
    let raw = make_api_key(&pool, uid, false, None).await; // is_active = false
    let state = build_state(pool);

    let content = format!("# revoked\n\n{}", unique());
    let (ct, body) = multipart_body("revoked.md", content.as_bytes());
    let req = Request::builder()
        .method("POST")
        .uri("/api/documents/upload")
        .header("content-type", ct)
        .header("x-api-key", &raw)
        .body(Body::from(body))
        .unwrap();

    let (status, _b) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "revoked (is_active=false) key must be rejected on upload"
    );
}

#[tokio::test]
async fn expired_key_update_is_401() {
    let Some(pool) = setup().await else { return };
    let (uid, _email) = make_user(&pool).await;
    // expired one hour ago
    let raw = make_api_key(&pool, uid, true, Some(Utc::now() - Duration::hours(1))).await;
    let doc_id = seed_document(&pool, uid, "expired-key-target").await;
    let state = build_state(pool.clone());

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/documents/{doc_id}"))
        .header("content-type", "application/json")
        .header("x-api-key", &raw)
        .body(Body::from(r#"{"title":"should-not-apply"}"#))
        .unwrap();

    let (status, _b) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "expired key must be rejected on update"
    );

    // Document must be untouched.
    let (_, title) = doc_owner_title(&pool, &doc_id).await.expect("doc exists");
    assert_eq!(
        title, "expired-key-target",
        "expired-key update must not mutate the document"
    );
}

// ===========================================================================
// ATTACK 2 — arbitrary / malformed X-API-Key -> 401, no panic / no 500
// ===========================================================================

#[tokio::test]
async fn bogus_key_upload_is_401_no_panic() {
    let Some(pool) = setup().await else { return };
    let state = build_state(pool);

    // A spread of hostile / malformed key slots. None exist in api_keys.
    let hostile = [
        "totally-made-up-key",
        "' OR '1'='1",
        "mk_",
        "🔥🔥🔥 not-a-key 🔥🔥🔥",
        // a raw SHA256-looking string (never authenticates: server re-hashes input)
        &hash_key("some-guess"),
    ];

    for (i, key) in hostile.iter().enumerate() {
        let content = format!("# bogus {i}\n\n{}", unique());
        let (ct, body) = multipart_body("bogus.md", content.as_bytes());
        let req = Request::builder()
            .method("POST")
            .uri("/api/documents/upload")
            .header("content-type", ct)
            .header("x-api-key", *key)
            .body(Body::from(body))
            .unwrap();

        let (status, _b) = send(app(state.clone()), req).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "bogus key #{i} ({key:?}) must fail-closed with 401, not panic/500"
        );
    }
}

// ===========================================================================
// ATTACK 3 — tenant isolation: uploaded doc owned by key user; cross-user
//            update on another user's document is refused (404, unchanged).
// ===========================================================================

#[tokio::test]
async fn upload_owner_is_exactly_key_user() {
    let Some(pool) = setup().await else { return };
    let (uid, _email) = make_user(&pool).await;
    let raw = make_api_key(&pool, uid, true, None).await;
    let state = build_state(pool.clone());

    let content = format!("# owner check\n\n{}", unique());
    let (ct, body) = multipart_body("owner.md", content.as_bytes());
    let req = Request::builder()
        .method("POST")
        .uri("/api/documents/upload")
        .header("content-type", ct)
        .header("x-api-key", &raw)
        .body(Body::from(body))
        .unwrap();

    let (status, resp) = send(app(state), req).await;
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "valid key upload must succeed, got {status} body={resp}"
    );
    let doc_id = resp["document"]["id"]
        .as_str()
        .expect("upload response has document.id");

    let (owner, _title) = doc_owner_title(&pool, doc_id).await.expect("doc exists");
    assert_eq!(
        owner, uid,
        "uploaded document owner must equal the api_key's user_id exactly"
    );
}

#[tokio::test]
async fn cross_user_update_is_refused() {
    let Some(pool) = setup().await else { return };
    // Victim: user A owns a document.
    let (uid_a, _email_a) = make_user(&pool).await;
    let doc_a = seed_document(&pool, uid_a, "victim-owned").await;

    // Attacker: user B holds a valid service key.
    let (uid_b, _email_b) = make_user(&pool).await;
    let raw_b = make_api_key(&pool, uid_b, true, None).await;
    assert_ne!(uid_a, uid_b, "attacker and victim must be different users");

    let state = build_state(pool.clone());
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/documents/{doc_a}"))
        .header("content-type", "application/json")
        .header("x-api-key", &raw_b)
        .body(Body::from(r#"{"title":"pwned-by-b"}"#))
        .unwrap();

    let (status, _b) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "user B must not be able to update user A's document (owner-scoped -> 404)"
    );

    // Victim document must be completely unchanged and still owned by A.
    let (owner, title) = doc_owner_title(&pool, &doc_a).await.expect("victim doc exists");
    assert_eq!(owner, uid_a, "victim document owner must be unchanged");
    assert_eq!(
        title, "victim-owned",
        "victim document title must not be mutated by the cross-user attempt"
    );
}

// ===========================================================================
// ATTACK 4 — credential precedence: Bearer JWT (user A) + X-API-Key (user B)
//            together -> JWT wins, document created under user A.
// ===========================================================================

#[tokio::test]
async fn jwt_beats_api_key_on_upload() {
    let Some(pool) = setup().await else { return };
    // User A authenticates by JWT; user B provides a valid service key.
    let (uid_a, email_a) = make_user(&pool).await;
    let (uid_b, _email_b) = make_user(&pool).await;
    let raw_b = make_api_key(&pool, uid_b, true, None).await;
    assert_ne!(uid_a, uid_b, "JWT user and key user must differ");

    let jwt = mint_jwt(uid_a, &email_a);
    let state = build_state(pool.clone());

    let content = format!("# precedence\n\n{}", unique());
    let (ct, body) = multipart_body("precedence.md", content.as_bytes());
    let req = Request::builder()
        .method("POST")
        .uri("/api/documents/upload")
        .header("content-type", ct)
        .header("authorization", format!("Bearer {jwt}"))
        .header("x-api-key", &raw_b)
        .body(Body::from(body))
        .unwrap();

    let (status, resp) = send(app(state), req).await;
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "JWT+key upload must succeed, got {status} body={resp}"
    );
    let doc_id = resp["document"]["id"]
        .as_str()
        .expect("upload response has document.id");

    let (owner, _title) = doc_owner_title(&pool, doc_id).await.expect("doc exists");
    assert_eq!(
        owner, uid_a,
        "when both credentials are present the JWT must win (doc owned by JWT user A, not key user B)"
    );
    assert_ne!(
        owner, uid_b,
        "X-API-Key user B must NOT own the document when a valid JWT is present"
    );
}
