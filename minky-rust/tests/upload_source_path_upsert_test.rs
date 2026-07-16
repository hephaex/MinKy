//! Integration tests for S1: source_path-based upsert on `POST
//! /api/documents/upload` (routes/documents.rs::upload_document).
//!
//! Contract under test:
//!   - An optional multipart text field `source_path` is accepted.
//!   - When `source_path` is given and an ACTIVE document already occupies
//!     that path with DIFFERENT content, the old document is archived
//!     (`archived = true`, `archived_at` set) and a NEW document row is
//!     inserted with the same `source_path`, and the response's
//!     `replaced_document_id` is the archived document's id.
//!   - When the uploaded content is IDENTICAL (same content_hash) to an
//!     already-active document for this user, the existing global
//!     content_hash 409 (`DuplicateDocument`) contract is preserved
//!     UNCHANGED — regardless of source_path (same path or a different one).
//!   - When `source_path` is omitted, behavior is 100% unchanged (covered
//!     already by `upload_update_auth_qa.rs::api_key_upload_succeeds`; this
//!     file adds one assertion that the new `replaced_document_id` field is
//!     present-and-null so existing/naive JSON consumers are unaffected).
//!
//! Safety / CI: every test connects via `G002_QA_DATABASE_URL` (preferred) or
//! `DATABASE_URL`. When neither is set — or the DB cannot be reached / migrated
//! — the test is a no-op that PASSES, so CI without a database stays green.
//! Mirrors the connection/schema/auth helper pattern already established in
//! `upload_update_auth_qa.rs`.

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
// Environment / connection / schema (mirrors upload_update_auth_qa.rs)
// ---------------------------------------------------------------------------

fn db_url() -> Option<String> {
    std::env::var("G002_QA_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .filter(|s| !s.trim().is_empty())
}

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

/// Mirror of the private `hash_api_key` in `security_service.rs`.
fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn make_user(pool: &PgPool) -> i32 {
    let u = unique();
    let (id,): (i32,) =
        sqlx::query_as("INSERT INTO users (email, username, password_hash) VALUES ($1, $2, '') RETURNING id")
            .bind(format!("s1upsert+{u}@qa.local"))
            .bind(format!("s1upsert_{u}"))
            .fetch_one(pool)
            .await
            .expect("insert qa user");
    id
}

async fn make_api_key(pool: &PgPool, user_id: i32) -> String {
    let raw = format!("mk_s1upsert_{}", unique());
    let key_hash = hash_key(&raw);
    let prefix: String = raw.chars().take(8).collect();
    sqlx::query(
        r#"INSERT INTO api_keys (name, key_prefix, key_hash, user_id, permissions, is_active)
           VALUES ($1, $2, $3, $4, '[]'::jsonb, true)"#,
    )
    .bind("s1upsert-bot")
    .bind(prefix)
    .bind(&key_hash)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert api key");
    raw
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
        jwt_secret: SecretString::from("s1upsert-qa-secret"),
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

/// Build a multipart body with a "file" field and an optional "source_path"
/// text field.
fn multipart_body(filename: &str, content: &[u8], source_path: Option<&str>) -> (String, Vec<u8>) {
    let boundary = "----S1UpsertBoundary7c1e";
    let mut body = Vec::new();

    if let Some(sp) = source_path {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"source_path\"\r\n\r\n",
        );
        body.extend_from_slice(sp.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

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

async fn upload(
    app: Router,
    api_key: &str,
    filename: &str,
    content: &[u8],
    source_path: Option<&str>,
) -> (StatusCode, Value) {
    let (ct, body) = multipart_body(filename, content, source_path);
    let req = Request::builder()
        .method("POST")
        .uri("/api/documents/upload")
        .header("content-type", ct)
        .header("x-api-key", api_key)
        .body(Body::from(body))
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
// CASE 1 — upload with source_path stores it, replaced_document_id is null
// ===========================================================================

#[tokio::test]
async fn upload_with_source_path_stores_it_and_replaced_is_null() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let state = build_state(pool.clone());

    let sp = format!("/vault/s1/{}.md", unique());
    let content = format!("# S1 first version\n\nbody {}", unique());
    let (status, resp) = upload(
        app(state),
        &raw,
        "case1.md",
        content.as_bytes(),
        Some(&sp),
    )
    .await;

    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "upload with source_path must succeed, got {status} body={resp}"
    );
    assert!(
        resp.get("replaced_document_id").is_some(),
        "response must include a replaced_document_id key (additive field), body={resp}"
    );
    assert!(
        resp["replaced_document_id"].is_null(),
        "first upload at a fresh source_path must not replace anything, body={resp}"
    );

    let doc_id = resp["document"]["id"].as_str().expect("document.id present");
    let stored: (Option<String>, bool) = sqlx::query_as(
        "SELECT source_path, archived FROM documents WHERE id = $1::uuid",
    )
    .bind(doc_id)
    .fetch_one(&pool)
    .await
    .expect("fetch stored document");
    assert_eq!(stored.0.as_deref(), Some(sp.as_str()), "source_path must be persisted");
    assert!(!stored.1, "freshly uploaded document must be active (archived=false)");
}

// ===========================================================================
// CASE 2 — re-upload with same source_path, different content: archive+insert
// ===========================================================================

#[tokio::test]
async fn reupload_same_source_path_different_content_archives_old_and_creates_new() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let sp = format!("/vault/s1/{}.md", unique());

    // v1
    let content_v1 = format!("# S1 v1\n\nbody {}", unique());
    let (status1, resp1) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case2.md",
        content_v1.as_bytes(),
        Some(&sp),
    )
    .await;
    assert!(status1.is_success(), "v1 upload must succeed, got {status1} body={resp1}");
    let v1_id = resp1["document"]["id"].as_str().expect("v1 document.id").to_string();

    // v2: same source_path, DIFFERENT content
    let content_v2 = format!("# S1 v2\n\nbody {}", unique());
    let (status2, resp2) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case2.md",
        content_v2.as_bytes(),
        Some(&sp),
    )
    .await;
    assert!(
        status2.is_success(),
        "v2 upload (replace) must succeed, got {status2} body={resp2}"
    );

    let replaced = resp2["replaced_document_id"]
        .as_str()
        .expect("replaced_document_id must be set when replacing an active source_path document");
    assert_eq!(replaced, v1_id, "replaced_document_id must be v1's id");

    let v2_id = resp2["document"]["id"].as_str().expect("v2 document.id").to_string();
    assert_ne!(v2_id, v1_id, "v2 must be a NEW document row, not an update-in-place of v1");

    // v1 is archived; v2 is active. Both share the same source_path (proves
    // migration 024's archived-scoped unique index, not the old unscoped one).
    let v1_row: (bool, Option<chrono::DateTime<chrono::Utc>>, Option<String>) = sqlx::query_as(
        "SELECT archived, archived_at, source_path FROM documents WHERE id = $1::uuid",
    )
    .bind(&v1_id)
    .fetch_one(&pool)
    .await
    .expect("fetch v1");
    assert!(v1_row.0, "v1 must be archived after being replaced");
    assert!(v1_row.1.is_some(), "v1 archived_at must be set");
    assert_eq!(v1_row.2.as_deref(), Some(sp.as_str()), "v1 keeps its source_path as history");

    let v2_row: (bool, Option<String>, String) = sqlx::query_as(
        "SELECT archived, source_path, content FROM documents WHERE id = $1::uuid",
    )
    .bind(&v2_id)
    .fetch_one(&pool)
    .await
    .expect("fetch v2");
    assert!(!v2_row.0, "v2 must be active (archived=false)");
    assert_eq!(v2_row.1.as_deref(), Some(sp.as_str()), "v2 must carry the same source_path");
    assert_eq!(v2_row.2, content_v2, "v2 must store the new content");

    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM documents WHERE user_id = $1 AND source_path = $2 AND archived = false",
    )
    .bind(uid)
    .bind(&sp)
    .fetch_one(&pool)
    .await
    .expect("count active rows at source_path");
    assert_eq!(active_count.0, 1, "exactly one active document may occupy a source_path");
}

// ===========================================================================
// CASE 3 — re-upload with same source_path AND same content: unchanged 409
// ===========================================================================

#[tokio::test]
async fn reupload_same_source_path_same_content_still_returns_409() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let sp = format!("/vault/s1/{}.md", unique());
    let content = format!("# S1 identical\n\nbody {}", unique());

    let (status1, resp1) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case3.md",
        content.as_bytes(),
        Some(&sp),
    )
    .await;
    assert!(status1.is_success(), "first upload must succeed, got {status1} body={resp1}");

    // Re-upload identical content at the SAME source_path.
    let (status2, resp2) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case3.md",
        content.as_bytes(),
        Some(&sp),
    )
    .await;
    assert_eq!(
        status2,
        StatusCode::CONFLICT,
        "identical content at the same source_path must still 409 (unchanged contract), body={resp2}"
    );

    let active_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM documents WHERE user_id = $1 AND source_path = $2",
    )
    .bind(uid)
    .bind(&sp)
    .fetch_one(&pool)
    .await
    .expect("count rows at source_path");
    assert_eq!(
        active_count.0, 1,
        "a rejected duplicate upload must not archive or insert anything"
    );
}

// ===========================================================================
// CASE 4 — same content at a DIFFERENT source_path: still 409 (global dedup)
// ===========================================================================

#[tokio::test]
async fn same_content_at_different_source_path_still_returns_409() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;
    let sp_a = format!("/vault/s1/{}-a.md", unique());
    let sp_b = format!("/vault/s1/{}-b.md", unique());
    let content = format!("# S1 shared content\n\nbody {}", unique());

    let (status1, resp1) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case4a.md",
        content.as_bytes(),
        Some(&sp_a),
    )
    .await;
    assert!(status1.is_success(), "first upload must succeed, got {status1} body={resp1}");

    // Same content, different source_path -> global content_hash dedup still fires.
    let (status2, resp2) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case4b.md",
        content.as_bytes(),
        Some(&sp_b),
    )
    .await;
    assert_eq!(
        status2,
        StatusCode::CONFLICT,
        "identical content at a different source_path must still 409 (global content_hash dedup is independent of source_path), body={resp2}"
    );

    let sp_b_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM documents WHERE user_id = $1 AND source_path = $2",
    )
    .bind(uid)
    .bind(&sp_b)
    .fetch_one(&pool)
    .await
    .expect("count rows at sp_b");
    assert_eq!(sp_b_count.0, 0, "a globally-deduped upload must not create a row at the new source_path");
}

// ===========================================================================
// CASE 5 — upload WITHOUT source_path is unaffected (backward compatibility)
// ===========================================================================

#[tokio::test]
async fn upload_without_source_path_is_unaffected() {
    let Some(pool) = setup().await else { return };
    let uid = make_user(&pool).await;
    let raw = make_api_key(&pool, uid).await;

    let content = format!("# No source_path\n\nbody {}", unique());
    let (status, resp) = upload(
        app(build_state(pool.clone())),
        &raw,
        "case5.md",
        content.as_bytes(),
        None,
    )
    .await;

    assert!(status.is_success(), "upload without source_path must still succeed, got {status} body={resp}");
    assert!(
        resp["replaced_document_id"].is_null(),
        "no source_path given -> replaced_document_id must be null, body={resp}"
    );

    let doc_id = resp["document"]["id"].as_str().expect("document.id present");
    let stored: (Option<String>,) = sqlx::query_as(
        "SELECT source_path FROM documents WHERE id = $1::uuid",
    )
    .bind(doc_id)
    .fetch_one(&pool)
    .await
    .expect("fetch stored document");
    assert!(stored.0.is_none(), "no source_path given -> stored source_path must stay NULL");
}
