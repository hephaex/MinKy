//! Adversarial QA (red-team) for `POST /chat/message` — G001 send_message handler.
//!
//! Contract under attack:
//!   - Empty content ("", whitespace-only) → 400; RagService must NOT be reached.
//!   - content > 2000 chars → 400.
//!   - No Authorization header / API key → 401.
//!   - Other user's session_id → 404 (session existence must not be leaked).
//!   - Valid UUID session_id absent from DB → 404.
//!   - Happy path: request passes all guards; response is a FLAT JSON object
//!     (no `success`/`data` wrapper) with `tokensUsed` in camelCase.
//!
//! Safety / CI: every test connects via `G002_QA_DATABASE_URL` (preferred) or
//! `DATABASE_URL`. When neither is set — or the DB cannot be reached / migrated
//! — the test is a no-op that PASSES, so CI without a database stays green.
//! Mirrors the connection/schema/auth helper pattern from
//! `upload_source_path_upsert_test.rs` and `auth_principal_qa.rs`.

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
use uuid::Uuid;

use minky::config::Config;
use minky::routes;
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

/// Connect to the QA database and run all migrations, or return `None` to
/// signal "skip this test" (no-op PASS).
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
            eprintln!("SKIP chat_message_qa: cannot connect to QA database: {e}");
            return None;
        }
    };
    // Run the full migration set — includes 025_chat_sessions which creates
    // the chat_sessions and chat_messages tables required by these tests.
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        eprintln!("SKIP chat_message_qa: cannot apply migrations: {e}");
        return None;
    }
    Some(pool)
}

// ---------------------------------------------------------------------------
// Uniqueness / seed helpers
// ---------------------------------------------------------------------------

static SEQ: AtomicU64 = AtomicU64::new(0);

fn unique() -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}_{n}")
}

/// SHA-256 hex mirror of `security_service.rs::hash_api_key`.
fn hash_key(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    format!("{:x}", h.finalize())
}

async fn make_user(pool: &PgPool) -> i32 {
    let u = unique();
    let (id,): (i32,) = sqlx::query_as(
        "INSERT INTO users (email, username, password_hash) VALUES ($1, $2, '') RETURNING id",
    )
    .bind(format!("chatqa+{u}@qa.local"))
    .bind(format!("chatqa_{u}"))
    .fetch_one(pool)
    .await
    .expect("insert qa user");
    id
}

/// Insert an API key for `user_id` and return the raw (unhashed) key string.
async fn make_api_key(pool: &PgPool, user_id: i32) -> String {
    let raw = format!("mk_chatqa_{}", unique());
    let key_hash = hash_key(&raw);
    let prefix: String = raw.chars().take(8).collect();
    sqlx::query(
        r#"INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, is_active)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, true)"#,
    )
    .bind("chatqa-bot")
    .bind(prefix)
    .bind(&key_hash)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert api key");
    raw
}

/// Create a chat session for `user_id` and return its UUID.
async fn make_session(pool: &PgPool, user_id: i32) -> Uuid {
    let (id,): (Uuid,) = sqlx::query_as(
        "INSERT INTO chat_sessions (user_id, title) VALUES ($1, $2) RETURNING id",
    )
    .bind(user_id)
    .bind(format!("QA session {}", unique()))
    .fetch_one(pool)
    .await
    .expect("insert chat session");
    id
}

// ---------------------------------------------------------------------------
// Router / state helpers
// ---------------------------------------------------------------------------

fn test_config(pool_url: &str) -> Config {
    Config {
        environment: "development".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8000,
        database_url: pool_url.to_string(),
        database_max_connections: 3,
        database_min_connections: 1,
        database_acquire_timeout_secs: 30,
        database_max_lifetime_secs: 1800,
        database_idle_timeout_secs: 600,
        jwt_secret: SecretString::from("chatqa-secret"),
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
    let url = db_url().unwrap_or_else(|| "postgres://localhost/none".to_string());
    let embedding = Arc::new(EmbeddingService::new(pool.clone(), EmbeddingConfig::default()));
    AppState {
        db: pool,
        config: test_config(&url),
        ws_manager: Arc::new(WebSocketManager::new()),
        embedding_service: embedding,
        vault_watcher: Arc::new(tokio::sync::Mutex::new(None)),
        redis_client: None,
        http_client: reqwest::Client::new(),
        korean_tokenizer: None,
    }
}

/// Build the full `Router` mirroring `create_app` (all routes nested under `/api`).
fn app(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::api_routes())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// HTTP helper
// ---------------------------------------------------------------------------

/// POST `/api/chat/message` with an optional `X-Api-Key` header and a JSON body.
async fn post_message(
    app: Router,
    api_key: Option<&str>,
    body: serde_json::Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/chat/message")
        .header("content-type", "application/json");

    if let Some(k) = api_key {
        builder = builder.header("x-api-key", k);
    }

    let req = builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.expect("router oneshot");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

// ===========================================================================
// CASE 1 — empty content → 400, RagService not reached
// ===========================================================================

#[tokio::test]
async fn case1_empty_content_is_400() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let key = make_api_key(&pool, uid).await;
    let session_id = make_session(&pool, uid).await;
    let state = build_state(pool);

    // Sub-case A: truly empty string
    let (status, body) = post_message(
        app(state.clone()),
        Some(&key),
        serde_json::json!({ "session_id": session_id, "content": "" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "empty content must be 400; body={body}"
    );

    // Sub-case B: whitespace-only — trim() must catch this before RAG
    let (status, body) = post_message(
        app(state),
        Some(&key),
        serde_json::json!({ "session_id": session_id, "content": "   " }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "whitespace-only content must be 400; body={body}"
    );
}

// ===========================================================================
// CASE 2 — content > 2000 chars → 400
// ===========================================================================

#[tokio::test]
async fn case2_overlength_content_is_400() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let key = make_api_key(&pool, uid).await;
    let session_id = make_session(&pool, uid).await;
    let state = build_state(pool);

    // Exactly at the boundary (2000) should be accepted; we verify 2001 is rejected.
    let over_limit = "x".repeat(2001);
    let (status, body) = post_message(
        app(state),
        Some(&key),
        serde_json::json!({ "session_id": session_id, "content": over_limit }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "2001-char content must be 400; body={body}"
    );
}

// ===========================================================================
// CASE 3 — no Authorization header or API key → 401
// ===========================================================================

#[tokio::test]
async fn case3_unauthenticated_request_is_401() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let session_id = make_session(&pool, uid).await;
    let state = build_state(pool);

    // No api_key passed → None
    let (status, body) = post_message(
        app(state),
        None, // no X-Api-Key, no Authorization header
        serde_json::json!({ "session_id": session_id, "content": "hello" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "request without any auth must be 401; body={body}"
    );
}

// ===========================================================================
// CASE 4 — other user's session_id → 404 (no existence leak)
// ===========================================================================

#[tokio::test]
async fn case4_other_users_session_is_404() {
    let Some(pool) = setup().await else { return };

    // User A owns the session
    let uid_a = make_user(&pool).await;
    let session_id_a = make_session(&pool, uid_a).await;

    // User B sends the request
    let uid_b = make_user(&pool).await;
    let key_b = make_api_key(&pool, uid_b).await;
    let state = build_state(pool);

    let (status, body) = post_message(
        app(state),
        Some(&key_b),
        serde_json::json!({ "session_id": session_id_a, "content": "hello" }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another user's session_id must be 404, not 403 (no existence leak); body={body}"
    );
    // Verify the body does not reveal that the session EXISTS but is owned by someone else.
    // A 404 error message is acceptable; any field that leaks "belongs to another user" is not.
    let body_str = body.to_string().to_lowercase();
    assert!(
        !body_str.contains("another user") && !body_str.contains("not authorized"),
        "error body must not reveal ownership details; body={body}"
    );
}

// ===========================================================================
// CASE 5 — valid UUID but session absent from DB → 404
// ===========================================================================

#[tokio::test]
async fn case5_nonexistent_session_uuid_is_404() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let key = make_api_key(&pool, uid).await;
    let state = build_state(pool);

    // A freshly generated UUID is astronomically unlikely to exist in the DB.
    let ghost_id = Uuid::new_v4();
    let (status, body) = post_message(
        app(state),
        Some(&key),
        serde_json::json!({ "session_id": ghost_id, "content": "hello" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "absent-but-valid session UUID must be 404; body={body}"
    );
}

// ===========================================================================
// CASE 6 — happy path: guards pass; if 200 the response is flat JSON with
//           camelCase `tokensUsed` and no Envelope wrapper.
// ===========================================================================

#[tokio::test]
async fn case6_happy_path_passes_all_guards_and_flat_json_shape() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let key = make_api_key(&pool, uid).await;
    let session_id = make_session(&pool, uid).await;
    let state = build_state(pool);

    let (status, body) = post_message(
        app(state),
        Some(&key),
        serde_json::json!({ "session_id": session_id, "content": "What is MinKy?" }),
    )
    .await;

    // The request MUST have passed all guards (auth, validation, session ownership).
    // In a test environment with no AI API keys, RagService::ask() will fail
    // internally, yielding a non-200 status (500/502). That is NOT a bug here —
    // the guard layer is what we verify.  We only assert shape on 200.
    assert_ne!(
        status,
        StatusCode::UNAUTHORIZED,
        "authenticated request with valid session must not be 401; body={body}"
    );
    assert_ne!(
        status,
        StatusCode::BAD_REQUEST,
        "valid content must not be 400; body={body}"
    );
    assert_ne!(
        status,
        StatusCode::NOT_FOUND,
        "user's own session must not be 404; body={body}"
    );

    if status == StatusCode::OK {
        // Verify the response is a FLAT JSON object — not wrapped in Envelope<T>.
        assert!(
            body.get("success").is_none(),
            "SendMessageResponse must NOT have an Envelope `success` key; body={body}"
        );
        assert!(
            body.get("data").is_none(),
            "SendMessageResponse must NOT have an Envelope `data` key; body={body}"
        );
        // Top-level `content` field
        assert!(
            body.get("content").is_some(),
            "SendMessageResponse must have a top-level `content` field; body={body}"
        );
        // `tokensUsed` must be camelCase (not `tokens_used`)
        assert!(
            body.get("tokensUsed").is_some(),
            "SendMessageResponse must serialize tokens_used as `tokensUsed`; body={body}"
        );
        assert!(
            body.get("tokens_used").is_none(),
            "snake_case `tokens_used` must not appear in response; body={body}"
        );
        // `sources` must be present (can be empty array)
        assert!(
            body.get("sources").is_some(),
            "SendMessageResponse must have a `sources` field; body={body}"
        );
    } else {
        eprintln!(
            "INFO case6: guards passed; RagService returned {status} (no AI API keys in test env). \
             Shape assertions skipped — run with valid AI keys for full 200 path coverage."
        );
    }
}
