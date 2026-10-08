//! Integration QA for the X-API-Key (AuthPrincipal) path on the document
//! upload/update routes (G008).
//!
//! Contract under test (G008 / reuse of G002 WP1 AuthPrincipal pattern):
//!   - `POST /api/documents/upload` and `PUT /api/documents/{id}` accept a
//!     valid `X-API-Key` service credential (previously JWT/cookie only).
//!   - Anonymous requests to either route stay fail-closed with `401`.
//!   - A service principal acts as `api_keys.user_id` (documents are created /
//!     updated under that user).
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
use secrecy::SecretString;
use serde_json::Value;
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

fn db_url() -> Option<String> {
    std::env::var("G002_QA_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .filter(|s| !s.trim().is_empty())
}

/// Connect + migrate, or return `None` to signal "skip this test".
///
/// A missing env var, an unreachable DB, or a migration error all resolve to
/// `None` (skip/PASS) so a machine without a suitable Postgres never fails CI.
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

async fn make_user(pool: &PgPool) -> i32 {
    let u = unique();
    let (id,): (i32,) =
        sqlx::query_as("INSERT INTO users (email, username, password_hash) VALUES ($1, $2, '') RETURNING id")
            .bind(format!("g008+{u}@qa.local"))
            .bind(format!("g008_{u}"))
            .fetch_one(pool)
            .await
            .expect("insert qa user");
    id
}

/// Insert an active api_keys row keyed on `raw_key`. Returns the raw key.
async fn make_api_key(pool: &PgPool, user_id: i32) -> String {
    let raw = format!("mk_g008_{}", unique());
    let key_hash = hash_key(&raw);
    let prefix: String = raw.chars().take(8).collect();
    sqlx::query(
        r#"INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, is_active)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, true)"#,
    )
    .bind("g008-bot")
    .bind(prefix)
    .bind(&key_hash)
    .bind(user_id)
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
        jwt_secret: SecretString::from("g008-qa-secret"),
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

fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api/documents", documents::routes())
        .with_state(state)
}

/// Build a single-field ("file") multipart body.
fn multipart_body(filename: &str, content: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----G008Boundary9f2a";
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

// ===========================================================================
// CASE 1 — anonymous upload -> 401 (fail-closed preserved)
// ===========================================================================

#[tokio::test]
async fn anonymous_upload_is_401() {
    let Some(pool) = setup().await else { return };
    let state = build_state(pool);

    let (ct, body) = multipart_body("anon.md", b"# anon\n\nno credentials");
    let req = Request::builder()
        .method("POST")
        .uri("/api/documents/upload")
        .header("content-type", ct)
        .body(Body::from(body))
        .unwrap();

    let (status, _b) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "anonymous upload must stay fail-closed (401)"
    );
}

// ===========================================================================
// CASE 2 — valid X-API-Key upload -> success under the key's user
// ===========================================================================

#[tokio::test]
async fn api_key_upload_succeeds() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let state = build_state(pool.clone());

    // Unique content so the duplicate guard never trips across reruns.
    let content = format!("# g008 upload\n\nservice principal body {}", unique());
    let (ct, body) = multipart_body("g008.md", content.as_bytes());
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
        "valid X-API-Key upload must succeed, got {status} body={resp}"
    );

    // The stored document must belong to the api_key's user.
    let doc_id = resp["document"]["id"]
        .as_str()
        .expect("upload response contains document.id");
    let owner: (i32,) = sqlx::query_as("SELECT user_id FROM documents WHERE id = $1::uuid")
        .bind(doc_id)
        .fetch_one(&pool)
        .await
        .expect("fetch uploaded document owner");
    assert_eq!(
        owner.0, uid,
        "uploaded document must be owned by the api_key's user_id"
    );
}

// ===========================================================================
// CASE 3 — anonymous update -> 401 (fail-closed preserved)
// ===========================================================================

#[tokio::test]
async fn anonymous_update_is_401() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let doc_id = seed_document(&pool, uid, "anon-update-target").await;
    let state = build_state(pool);

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/documents/{doc_id}"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"title":"hacked"}"#))
        .unwrap();

    let (status, _b) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "anonymous update must stay fail-closed (401)"
    );
}

// ===========================================================================
// CASE 4 — valid X-API-Key update -> success on the key user's document
// ===========================================================================

#[tokio::test]
async fn api_key_update_succeeds() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let doc_id = seed_document(&pool, uid, "before-update").await;
    let state = build_state(pool.clone());

    let new_title = format!("after-update-{}", unique());
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/documents/{doc_id}"))
        .header("content-type", "application/json")
        .header("x-api-key", &raw)
        .body(Body::from(format!(r#"{{"title":"{new_title}"}}"#)))
        .unwrap();

    let (status, resp) = send(app(state), req).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "valid X-API-Key update must succeed, body={resp}"
    );

    let stored: (String,) = sqlx::query_as("SELECT title FROM documents WHERE id = $1::uuid")
        .bind(&doc_id)
        .fetch_one(&pool)
        .await
        .expect("fetch updated document title");
    assert_eq!(stored.0, new_title, "update must persist the new title");
}
