//! Adversarial QA for `/api/pipeline/*` (6th Fable Redteam, P1 x2).
//!
//! Contract:
//!   - No credentials -> 401 on ingest, ingest/url and jobs/{id}; nothing is created.
//!   - A body-supplied `user_id` is ignored: the document is owned by the
//!     authenticated principal (JWT user or API-key owner).
//!   - ingest/url refuses internal targets (loopback, RFC1918, cloud metadata,
//!     file://) with a 4xx and never reaches the network.
//!
//! Unlike the older QA suites this one does NOT no-op when the database is
//! missing: an unset/unreachable DB panics, so "0 failed" can never mean "skipped".

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::SecretString;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use minky::config::Config;
use minky::routes;
use minky::services::{EmbeddingConfig, EmbeddingService, WebSocketManager};
use minky::AppState;

const JWT_SECRET: &str = "pipelineqa-secret";

fn db_url() -> String {
    std::env::var("G002_QA_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .filter(|s| !s.trim().is_empty())
        .expect("pipeline_auth_qa requires G002_QA_DATABASE_URL or DATABASE_URL (no silent skip)")
}

async fn setup() -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&db_url())
        .await
        .expect("connect QA database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");
    pool
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
    .bind(format!("pipeqa+{u}@qa.local"))
    .bind(format!("pipeqa_{u}"))
    .fetch_one(pool)
    .await
    .expect("insert qa user");
    id
}

async fn make_api_key(pool: &PgPool, user_id: i32) -> String {
    let raw = format!("mk_pipeqa_{}", unique());
    let prefix: String = raw.chars().take(8).collect();
    sqlx::query(
        r#"INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, is_active)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, true)"#,
    )
    .bind("pipeqa-bot")
    .bind(prefix)
    .bind(hash_key(&raw))
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert api key");
    raw
}

fn mint_jwt(user_id: i32) -> String {
    let now = Utc::now();
    let claims = json!({
        "sub": user_id.to_string(),
        "email": "pipeqa@qa.local",
        "role": "user",
        "exp": (now + Duration::hours(1)).timestamp(),
        "iat": now.timestamp(),
        "token_type": "access",
    });
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

fn test_config(url: &str) -> Config {
    Config {
        environment: "development".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8000,
        database_url: url.to_string(),
        database_max_connections: 3,
        database_min_connections: 1,
        database_acquire_timeout_secs: 30,
        database_max_lifetime_secs: 1800,
        database_idle_timeout_secs: 600,
        jwt_secret: SecretString::from(JWT_SECRET),
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

fn app(pool: PgPool) -> Router {
    let embedding = Arc::new(EmbeddingService::new(pool.clone(), EmbeddingConfig::default()));
    let state = AppState {
        db: pool,
        config: test_config(&db_url()),
        ws_manager: Arc::new(WebSocketManager::new()),
        embedding_service: embedding,
        vault_watcher: Arc::new(tokio::sync::Mutex::new(None)),
        redis_client: None,
        http_client: reqwest::Client::new(),
        korean_tokenizer: None,
    };
    Router::new().nest("/api", routes::api_routes()).with_state(state)
}

enum Auth<'a> {
    None,
    ApiKey(&'a str),
    Bearer(&'a str),
}

async fn call(app: Router, method: &str, path: &str, auth: Auth<'_>, body: Option<Value>) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(path);
    match auth {
        Auth::None => {}
        Auth::ApiKey(k) => b = b.header("x-api-key", k),
        Auth::Bearer(t) => b = b.header("authorization", format!("Bearer {t}")),
    }
    let req = match body {
        Some(v) => b
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let resp = app.oneshot(req).await.expect("router call");
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn owner_of(pool: &PgPool, doc_id: &str) -> Option<i32> {
    let id = Uuid::parse_str(doc_id).expect("document_id is a uuid");
    sqlx::query_as::<_, (Option<i32>,)>("SELECT user_id FROM documents WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("document row")
        .0
}

#[tokio::test]
async fn case1_unauthenticated_requests_are_401_and_create_nothing() {
    let pool = setup().await;
    let title = format!("unauth-{}", unique());
    let before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE title = $1")
        .bind(&title)
        .fetch_one(&pool)
        .await
        .unwrap();

    let (s, _) = call(
        app(pool.clone()),
        "POST",
        "/api/pipeline/ingest",
        Auth::None,
        Some(json!({"title": title, "content": "x", "skip_embedding": true, "skip_analysis": true})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "ingest without credentials");

    let (s, _) = call(
        app(pool.clone()),
        "POST",
        "/api/pipeline/ingest/url",
        Auth::None,
        Some(json!({"url": "http://127.0.0.1:1/"})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "ingest/url without credentials");

    let (s, _) = call(
        app(pool.clone()),
        "GET",
        &format!("/api/pipeline/jobs/{}", Uuid::new_v4()),
        Auth::None,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "jobs without credentials");

    let after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents WHERE title = $1")
        .bind(&title)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before.0, after.0, "no document may be created by an unauthenticated call");
}

#[tokio::test]
async fn case2_forged_jwt_is_401() {
    let pool = setup().await;
    let forged = encode(
        &Header::default(),
        &json!({"sub": "1", "email": "x@x", "role": "admin",
                "exp": (Utc::now() + Duration::hours(1)).timestamp(),
                "iat": Utc::now().timestamp(), "token_type": "access"}),
        &EncodingKey::from_secret(b"not-the-secret"),
    )
    .unwrap();
    let (s, _) = call(
        app(pool),
        "POST",
        "/api/pipeline/ingest",
        Auth::Bearer(&forged),
        Some(json!({"title": "t", "content": "c", "skip_embedding": true, "skip_analysis": true})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn case3_body_user_id_is_ignored_for_jwt_principal() {
    let pool = setup().await;
    let me = make_user(&pool).await;
    let victim = make_user(&pool).await;
    let jwt = mint_jwt(me);
    let (s, body) = call(
        app(pool.clone()),
        "POST",
        "/api/pipeline/ingest",
        Auth::Bearer(&jwt),
        Some(json!({
            "title": format!("own-{}", unique()), "content": "owned by me, not the victim",
            "user_id": victim, "skip_embedding": true, "skip_analysis": true
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "body: {body}");
    let doc = body["document_id"].as_str().expect("document_id");
    assert_eq!(owner_of(&pool, doc).await, Some(me), "owner must be the JWT user");
    assert_ne!(owner_of(&pool, doc).await, Some(victim));
}

#[tokio::test]
async fn case4_api_key_principal_owns_document() {
    let pool = setup().await;
    let owner = make_user(&pool).await;
    let victim = make_user(&pool).await;
    let key = make_api_key(&pool, owner).await;
    let (s, body) = call(
        app(pool.clone()),
        "POST",
        "/api/pipeline/ingest",
        Auth::ApiKey(&key),
        Some(json!({
            "title": format!("svc-{}", unique()), "content": "service ingest",
            "user_id": victim, "skip_embedding": true, "skip_analysis": true
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "body: {body}");
    let doc = body["document_id"].as_str().expect("document_id");
    assert_eq!(owner_of(&pool, doc).await, Some(owner), "owner must be api_keys.user_id");
}

#[tokio::test]
async fn case5_ingest_url_rejects_internal_targets_when_authenticated() {
    let pool = setup().await;
    let me = make_user(&pool).await;
    let jwt = mint_jwt(me);
    for url in [
        "http://127.0.0.1:1/",
        "http://localhost/",
        "http://169.254.169.254/latest/meta-data/",
        "http://192.168.29.22/",
        "http://10.0.0.1/",
        "http://[::1]/",
        "file:///etc/passwd",
    ] {
        let (s, body) = call(
            app(pool.clone()),
            "POST",
            "/api/pipeline/ingest/url",
            Auth::Bearer(&jwt),
            Some(json!({ "url": url })),
        )
        .await;
        assert!(
            s.is_client_error(),
            "{url} must be rejected with 4xx, got {s} body={body}"
        );
        assert_ne!(s, StatusCode::UNAUTHORIZED, "{url}: authenticated caller reached the handler");
    }
}
