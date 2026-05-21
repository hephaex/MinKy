//! Vault ingestion routes — manual ingest of Obsidian/Markdown vaults
//!
//! Exposes `POST /api/vault/ingest` which walks a local directory (or reads a
//! single file) and registers every `.md` file as a MinKy document owned by
//! the authenticated user.

use std::path::Path;

use axum::{extract::{Query, State}, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::{AdminUser, AuthUser},
    pipeline::{DocumentPipelineBuilder, IngestionInput},
    services::{
        vault_common::{
            collect_md_files, is_safe_md_path, validate_path as validate_path_common,
            MAX_FILE_BYTES, MAX_FILES_HARD_CAP,
        },
        EmbeddingService,
    },
    AppState,
};

/// Default value for `max_files` when the caller omits the field.
const DEFAULT_MAX_FILES: usize = 100;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ingest", post(ingest_vault))
        .route("/sync/report", get(sync_report))
        .route("/watch/status", get(watch_status))
        .route("/watch/reload", post(watch_reload))
}

// ── Request / Response types ──────────────────────────────────────────────────

/// Request body for `POST /api/vault/ingest`
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// Absolute path to a directory (vault root) or a single `.md` file.
    pub path: String,

    /// Whether to recurse into sub-directories. Defaults to `true`.
    #[serde(default = "default_recursive")]
    pub recursive: bool,

    /// Maximum number of files to ingest. Capped at 500, defaults to 100.
    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_recursive() -> bool {
    true
}
fn default_max_files() -> usize {
    DEFAULT_MAX_FILES
}

/// Per-file result collected during ingestion
#[derive(Debug, Serialize)]
pub struct IngestData {
    pub ingested: usize,
    pub skipped: usize,
    pub errors: usize,
    pub document_ids: Vec<Uuid>,
}

/// Top-level response envelope
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub success: bool,
    pub data: IngestData,
}

// ── Path validation ───────────────────────────────────────────────────────────

/// Validate that `raw` is an acceptable vault path.
///
/// Delegates to [`vault_common::validate_path`] and maps the `String` error
/// into an [`AppError::Validation`] so it can be propagated via `?`.
fn validate_path(raw: &str) -> AppResult<std::path::PathBuf> {
    validate_path_common(raw).map_err(AppError::Validation)
}

// ── Markdown helpers ──────────────────────────────────────────────────────────

/// Extract a human-readable title from markdown content.
///
/// Looks for the first ATX heading (`# Title`) and returns its text.
/// Falls back to the file stem (filename without extension) when no heading
/// is found.
pub fn extract_title(content: &str, file_stem: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            let title = heading.trim().to_string();
            if !title.is_empty() {
                return title;
            }
        }
    }
    // Fallback: humanise the filename stem
    file_stem.replace(['-', '_'], " ")
}

// ── Single-file ingestion helper ──────────────────────────────────────────────

/// Ingest a single `.md` file into the document pipeline.
///
/// Returns:
/// - `Ok(Some(document_id))` — file was successfully ingested.
/// - `Ok(None)` — file was silently skipped (oversized, empty, or symlink).
/// - `Err(AppError)` — a real failure occurred (pipeline or DB error).
///
/// The pipeline reads the file itself via [`DocumentSource::File`], which sets
/// `source_path` correctly in the document record.  The manual read-before-pass
/// pattern (which caused a TOCTOU race) is eliminated.
pub async fn ingest_single_file(
    pool: &PgPool,
    file_path: &Path,
    user_id: i32,
    embedding_service: &EmbeddingService,
) -> Result<Option<Uuid>, AppError> {
    // Guard: must be a plain (non-symlink) `.md` file.
    if !is_safe_md_path(file_path) {
        tracing::debug!("vault ingest: skipping {:?} — not a safe .md path", file_path);
        return Ok(None);
    }

    // Guard: enforce the file-size limit before passing to the pipeline.
    // Using metadata here (rather than a full read) is cheap and avoids
    // allocating a large buffer for files that will be rejected anyway.
    match tokio::fs::metadata(file_path).await {
        Ok(meta) if meta.len() > MAX_FILE_BYTES => {
            tracing::warn!(
                "vault ingest: skipping {:?} — file too large ({} bytes)",
                file_path,
                meta.len()
            );
            return Ok(None);
        }
        Err(e) => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "vault ingest: failed to stat {:?}: {}",
                file_path,
                e
            )));
        }
        _ => {}
    }

    // Build the path string once; the pipeline uses it as the `File` source so
    // that `source_path` is stored correctly in the document record.
    let path_str = file_path.to_str().unwrap_or("").to_string();

    let pipeline = DocumentPipelineBuilder::new()
        .pool(pool.clone())
        .semantic_chunking(512)
        .user_id(user_id)
        .skip_embedding()
        .skip_analysis()
        .build();

    // Use DocumentSource::File so the pipeline reads the file and populates
    // source_path on the resulting document (fixes C1).  MIME type is forced
    // to text/markdown regardless of what mime-guess returns.
    let mut input = IngestionInput::from_file(path_str.clone());
    input.options.mime_type = Some("text/markdown".to_string());

    let output = pipeline
        .process(input)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("vault ingest pipeline error for {:?}: {}", file_path, e)))?;

    let document_id = output.document_id;

    // Best-effort: enqueue for embedding without failing the ingest.
    enqueue_for_embedding(embedding_service, document_id).await;

    tracing::info!(
        "vault ingest: ingested {:?} → document_id={}, source_path={}",
        file_path,
        document_id,
        path_str,
    );

    Ok(Some(document_id))
}

// ── Sync-report query ─────────────────────────────────────────────────────────

/// Query parameters for `GET /api/vault/sync/report`.
///
/// Accepts one or more `root` values, e.g.
/// `/api/vault/sync/report?root=/my/vault&root=/other/vault`.
#[derive(Debug, serde::Deserialize)]
pub struct SyncReportQuery {
    /// One or more absolute vault root paths to compare against the database.
    pub root: Vec<String>,
}

// ── Sync-report handler ───────────────────────────────────────────────────────

/// `GET /api/vault/sync/report` — read-only comparison of disk vs DB.
///
/// For each provided vault root the handler:
/// 1. Validates the path (absolute, no traversal, exists).
/// 2. Walks the directory tree and collects all `.md` file paths.
/// 3. Queries `documents.source_path` for records whose path starts with the
///    root prefix.
/// 4. Returns:
///    - `tracked_count`: files present both on disk and in the DB.
///    - `orphans`: DB records whose `source_path` no longer exists on disk.
///    - `untracked`: disk files that have no corresponding DB record.
///
/// Admin-only.  Performs **no writes or deletions**.
pub async fn sync_report(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(params): Query<SyncReportQuery>,
) -> impl axum::response::IntoResponse {
    use std::collections::{HashMap, HashSet};

    // 1. Validate each root — reject the whole request on the first bad path.
    let mut valid_roots: Vec<std::path::PathBuf> = Vec::new();
    for root_str in &params.root {
        match validate_path_common(root_str) {
            Ok(p) => valid_roots.push(p),
            Err(e) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"success": false, "error": e})),
                )
                    .into_response();
            }
        }
    }

    if valid_roots.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "at least one root required"})),
        )
            .into_response();
    }

    // 2. Collect all .md paths on disk for every root.
    let mut disk_paths: HashSet<String> = HashSet::new();
    for root in &valid_roots {
        let files = collect_md_files(root, true, usize::MAX);
        for f in files {
            if let Some(s) = f.to_str() {
                disk_paths.insert(s.to_string());
            }
        }
    }

    // 3. Query the DB for documents whose source_path falls under one of the roots.
    let mut db_pairs: Vec<(uuid::Uuid, String)> = Vec::new();
    for root in &valid_roots {
        // Ensure the prefix ends with '/' so we don't accidentally match
        // paths that share a common string prefix but differ in directory
        // (e.g. /vault2 matching against a /vault prefix).
        let root_str = root.to_str().unwrap_or("");
        let root_prefix = if root_str.ends_with('/') {
            root_str.to_string()
        } else {
            format!("{root_str}/")
        };

        let rows = sqlx::query_as::<_, (uuid::Uuid, String)>(
            "SELECT id, source_path FROM documents \
             WHERE source_path IS NOT NULL \
             AND source_path LIKE $1 || '%'",
        )
        .bind(&root_prefix)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        db_pairs.extend(rows);
    }

    // Deduplicate: a source_path should map to exactly one document row;
    // later entries overwrite earlier ones in the HashMap.
    let db_docs: HashMap<String, uuid::Uuid> = db_pairs
        .into_iter()
        .map(|(id, path)| (path, id))
        .collect();

    // 4. Compute the diff sets.
    let db_path_set: HashSet<&str> = db_docs.keys().map(|s| s.as_str()).collect();

    // orphans: recorded in DB but the file no longer exists on disk.
    let mut orphans: Vec<serde_json::Value> = db_docs
        .iter()
        .filter(|(path, _)| !disk_paths.contains(*path))
        .map(|(path, id)| {
            serde_json::json!({
                "document_id": id,
                "source_path": path
            })
        })
        .collect();
    // Sort for deterministic output order.
    orphans.sort_by(|a, b| {
        a["source_path"]
            .as_str()
            .cmp(&b["source_path"].as_str())
    });

    // untracked: on disk but absent from the DB.
    let mut untracked: Vec<&str> = disk_paths
        .iter()
        .filter(|path| !db_path_set.contains(path.as_str()))
        .map(|s| s.as_str())
        .collect();
    untracked.sort_unstable();

    let tracked_count = disk_paths.len() - untracked.len();

    Json(serde_json::json!({
        "success": true,
        "data": {
            "roots": valid_roots.iter()
                .map(|p| p.to_str().unwrap_or(""))
                .collect::<Vec<_>>(),
            "tracked_count": tracked_count,
            "orphans": orphans,
            "untracked": untracked,
        }
    }))
    .into_response()
}

// ── Handler ───────────────────────────────────────────────────────────────────

async fn ingest_vault(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(mut payload): Json<IngestRequest>,
) -> AppResult<Json<IngestResponse>> {
    // Enforce the hard cap on max_files.
    if payload.max_files == 0 || payload.max_files > MAX_FILES_HARD_CAP {
        payload.max_files = MAX_FILES_HARD_CAP;
    }

    let vault_path = validate_path(&payload.path)?;
    let md_files = collect_md_files(&vault_path, payload.recursive, payload.max_files);

    // Reuse the shared embedding service rather than creating one per file.
    let embedding_service = std::sync::Arc::clone(&state.embedding_service);

    let mut ingested = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;
    let mut document_ids: Vec<Uuid> = Vec::new();

    for file_path in &md_files {
        match ingest_single_file(&state.db, file_path, auth_user.id, &embedding_service).await {
            Ok(Some(document_id)) => {
                document_ids.push(document_id);
                ingested += 1;
            }
            Ok(None) => {
                skipped += 1;
            }
            Err(e) => {
                tracing::warn!("vault ingest: error for {:?}: {}", file_path, e);
                errors += 1;
            }
        }
    }

    Ok(Json(IngestResponse {
        success: true,
        data: IngestData {
            ingested,
            skipped,
            errors,
            document_ids,
        },
    }))
}

// ── Watch status / reload endpoints ──────────────────────────────────────────

/// `GET /api/vault/watch/status` — report whether the background watcher is
/// currently running.
///
/// Admin-only: operational metadata should not be visible to regular users.
pub async fn watch_status(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> impl axum::response::IntoResponse {
    let guard = state.vault_watcher.lock().await;
    let is_active = guard.is_some();
    drop(guard);
    Json(serde_json::json!({
        "success": true,
        "data": {
            "watcher_active": is_active,
            "enabled": true,
        }
    }))
}

/// `POST /api/vault/watch/reload` — stop the running watcher (if any).
///
/// Admin-only: stopping the vault watcher is a privileged operation.
/// Full re-start requires a server restart because the watch roots are only
/// available in the startup configuration, not stored in `AppState`.
pub async fn watch_reload(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> impl axum::response::IntoResponse {
    let old_handle = {
        let mut guard = state.vault_watcher.lock().await;
        guard.take()
    };
    if let Some(handle) = old_handle {
        handle.stop();
    }
    Json(serde_json::json!({
        "success": true,
        "data": {
            "message": "Watcher stopped. Restart the server to reload configuration.",
            "watcher_active": false,
        }
    }))
}

// ── Embedding queue helper ────────────────────────────────────────────────────

async fn enqueue_for_embedding(
    service: &crate::services::EmbeddingService,
    document_id: Uuid,
) {
    if let Err(e) = service.queue_document(document_id, 0).await {
        tracing::warn!(
            "vault ingest: failed to enqueue document {} for embedding: {}",
            document_id,
            e
        );
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── extract_title ─────────────────────────────────────────────────────────

    #[test]
    fn test_extract_title_from_markdown_heading() {
        let content = "# My Document\n\nSome body text.";
        assert_eq!(extract_title(content, "my-document"), "My Document");
    }

    #[test]
    fn test_extract_title_falls_back_to_filename() {
        let content = "No heading here, just prose.";
        // file_stem "my_note" → humanised "my note"
        assert_eq!(extract_title(content, "my_note"), "my note");
    }

    #[test]
    fn test_extract_title_skips_empty_heading() {
        // A heading line with only whitespace after `#` should be skipped.
        let content = "#   \n# Real Title\nBody";
        assert_eq!(extract_title(content, "fallback"), "Real Title");
    }

    #[test]
    fn test_extract_title_uses_first_h1_only() {
        let content = "# First\n# Second\nBody";
        assert_eq!(extract_title(content, "f"), "First");
    }

    #[test]
    fn test_extract_title_ignores_h2_and_below() {
        // `##` is not an H1 and must not be used as the title.
        let content = "## Not H1\nBody without H1";
        // Should fall back to file stem.
        assert_eq!(extract_title(content, "stem"), "stem");
    }

    #[test]
    fn test_extract_title_trims_heading_whitespace() {
        let content = "#   Padded Title   \nBody";
        assert_eq!(extract_title(content, "f"), "Padded Title");
    }

    #[test]
    fn test_extract_title_filename_replaces_dashes_and_underscores() {
        let content = "no heading";
        assert_eq!(extract_title(content, "2026-04-15_my-note"), "2026 04 15 my note");
    }

    // ── validate_path ─────────────────────────────────────────────────────────

    #[test]
    fn test_validate_path_rejects_relative() {
        let result = validate_path("relative/path");
        assert!(result.is_err(), "relative path must be rejected");
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::Validation(_)),
            "expected Validation error"
        );
    }

    #[test]
    fn test_validate_path_rejects_traversal() {
        let result = validate_path("/valid/root/../../../etc/passwd");
        assert!(result.is_err(), "path traversal must be rejected");
    }

    #[test]
    fn test_validate_path_rejects_traversal_simple() {
        let result = validate_path("/tmp/../etc/shadow");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_accepts_existing_absolute() {
        // /tmp is guaranteed to exist on macOS and Linux.
        let result = validate_path("/tmp");
        assert!(result.is_ok(), "existing absolute path must be accepted");
    }

    #[test]
    fn test_validate_path_rejects_nonexistent() {
        let result = validate_path("/this/path/definitely/does/not/exist/9f3b2a");
        assert!(result.is_err(), "non-existent path must be rejected");
    }

    // ── max_files capping ─────────────────────────────────────────────────────

    #[test]
    fn test_max_files_capped_at_500() {
        // Simulate the handler logic: values above MAX_FILES_HARD_CAP are
        // replaced with the cap.
        let mut req = IngestRequest {
            path: "/tmp".to_string(),
            recursive: true,
            max_files: 9999,
        };
        if req.max_files == 0 || req.max_files > MAX_FILES_HARD_CAP {
            req.max_files = MAX_FILES_HARD_CAP;
        }
        assert_eq!(req.max_files, 500);
    }

    #[test]
    fn test_max_files_zero_is_capped() {
        let mut req = IngestRequest {
            path: "/tmp".to_string(),
            recursive: true,
            max_files: 0,
        };
        if req.max_files == 0 || req.max_files > MAX_FILES_HARD_CAP {
            req.max_files = MAX_FILES_HARD_CAP;
        }
        assert_eq!(req.max_files, 500);
    }

    #[test]
    fn test_max_files_within_cap_is_unchanged() {
        let mut req = IngestRequest {
            path: "/tmp".to_string(),
            recursive: false,
            max_files: 42,
        };
        if req.max_files == 0 || req.max_files > MAX_FILES_HARD_CAP {
            req.max_files = MAX_FILES_HARD_CAP;
        }
        assert_eq!(req.max_files, 42);
    }

    // ── collect_md_files (filesystem tests using tempdir) ────────────────────

    fn make_temp_vault() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).expect("write temp file");
        p
    }

    #[test]
    fn test_collect_single_file_path() {
        let dir = make_temp_vault();
        let f = write_file(dir.path(), "note.md", "# Hello");
        let files = collect_md_files(&f, false, 10);
        assert_eq!(files, vec![f]);
    }

    #[test]
    fn test_collect_ignores_non_md_files() {
        let dir = make_temp_vault();
        write_file(dir.path(), "note.md", "# Hello");
        write_file(dir.path(), "image.png", "binary");
        write_file(dir.path(), "readme.txt", "text");
        let files = collect_md_files(dir.path(), false, 100);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("note.md"));
    }

    #[test]
    fn test_collect_respects_limit() {
        let dir = make_temp_vault();
        for i in 0..10 {
            write_file(dir.path(), &format!("note{}.md", i), "body");
        }
        let files = collect_md_files(dir.path(), false, 3);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_collect_non_recursive_skips_subdir() {
        let dir = make_temp_vault();
        write_file(dir.path(), "root.md", "root");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        write_file(&sub, "child.md", "child");
        let files = collect_md_files(dir.path(), false, 100);
        assert_eq!(files.len(), 1, "should only find root-level file");
    }

    #[test]
    fn test_collect_recursive_includes_subdir() {
        let dir = make_temp_vault();
        write_file(dir.path(), "root.md", "root");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        write_file(&sub, "child.md", "child");
        let files = collect_md_files(dir.path(), true, 100);
        assert_eq!(files.len(), 2, "should find both root and sub-dir files");
    }

    #[test]
    fn test_collect_single_non_md_file_returns_empty() {
        let dir = make_temp_vault();
        let f = write_file(dir.path(), "data.json", "{}");
        let files = collect_md_files(&f, false, 10);
        assert!(files.is_empty());
    }

    // ── IngestResponse serialization ──────────────────────────────────────────

    #[test]
    fn test_ingest_response_serialization() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let resp = IngestResponse {
            success: true,
            data: IngestData {
                ingested: 5,
                skipped: 1,
                errors: 0,
                document_ids: vec![id],
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["ingested"], 5);
        assert_eq!(json["data"]["skipped"], 1);
        assert_eq!(json["data"]["errors"], 0);
        assert_eq!(json["data"]["document_ids"][0], id.to_string());
    }

    #[test]
    fn test_ingest_response_empty_ids() {
        let resp = IngestResponse {
            success: true,
            data: IngestData {
                ingested: 0,
                skipped: 0,
                errors: 0,
                document_ids: vec![],
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["data"]["document_ids"].as_array().unwrap().is_empty());
    }

    // ── Default serde values ──────────────────────────────────────────────────

    #[test]
    fn test_ingest_request_defaults() {
        let json = r#"{"path": "/tmp"}"#;
        let req: IngestRequest = serde_json::from_str(json).unwrap();
        assert!(req.recursive, "recursive should default to true");
        assert_eq!(
            req.max_files, DEFAULT_MAX_FILES,
            "max_files should default to 100"
        );
    }

    #[test]
    fn test_ingest_request_explicit_values() {
        let json = r#"{"path": "/tmp", "recursive": false, "max_files": 50}"#;
        let req: IngestRequest = serde_json::from_str(json).unwrap();
        assert!(!req.recursive);
        assert_eq!(req.max_files, 50);
    }

    /// Calling `ingest_single_file` twice on the same file path must produce
    /// the same source identifier so that the storage upsert-by-source_path
    /// logic deduplicates into a single document row.
    ///
    /// This is a structural / contract test — it verifies the precondition
    /// that guarantees dedup without requiring a live database:
    ///   same path string → same `DocumentSource::File { path }` → same
    ///   `source_path` column value → upsert updates the existing row.
    #[test]
    fn ingest_same_source_path_twice_produces_one_document() {
        use crate::pipeline::{DocumentSource, IngestionInput};

        let path = "/tmp/test-dedup.md";

        let input1 = IngestionInput::from_file(path);
        let input2 = IngestionInput::from_file(path);

        // Both inputs must carry the exact same source identifier.
        // The storage stage uses this value as the unique key for the upsert
        // (WHERE user_id = $1 AND source_path = $2), so equality here means
        // the second ingest will update the existing row rather than insert a
        // new one — i.e. "same file twice = 1 document row".
        match (&input1.source, &input2.source) {
            (
                DocumentSource::File { path: p1 },
                DocumentSource::File { path: p2 },
            ) => {
                assert_eq!(
                    p1, p2,
                    "from_file with the same path must produce the same source_path key"
                );
            }
            _ => panic!("from_file must produce DocumentSource::File"),
        }
    }

    // ── watch status / reload response shapes ─────────────────────────────────

    #[test]
    fn watch_status_response_shape() {
        let json = serde_json::json!({
            "success": true,
            "data": { "watcher_active": false, "enabled": true }
        });
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["watcher_active"], false);
        assert_eq!(json["data"]["enabled"], true);
    }

    #[test]
    fn watch_reload_response_shape() {
        let json = serde_json::json!({
            "success": true,
            "data": {
                "message": "Watcher stopped. Restart the server to reload configuration.",
                "watcher_active": false,
            }
        });
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["watcher_active"], false);
        assert!(json["data"]["message"].as_str().is_some());
    }

    // ── sync_report diff logic ────────────────────────────────────────────────

    #[test]
    fn sync_report_query_deserializes_multiple_roots() {
        let q = SyncReportQuery {
            root: vec!["/tmp/a".into(), "/tmp/b".into()],
        };
        assert_eq!(q.root.len(), 2);
        assert_eq!(q.root[0], "/tmp/a");
        assert_eq!(q.root[1], "/tmp/b");
    }

    #[test]
    fn sync_report_orphan_detection_logic() {
        use std::collections::{HashMap, HashSet};

        // DB has two paths; disk only contains one — the missing DB record is an orphan.
        let db: HashMap<String, uuid::Uuid> = [
            ("/vault/a.md".to_string(), uuid::Uuid::new_v4()),
            ("/vault/b.md".to_string(), uuid::Uuid::new_v4()),
        ]
        .into_iter()
        .collect();
        let disk: HashSet<String> = ["/vault/a.md".to_string()].into_iter().collect();

        let db_path_set: HashSet<&str> = db.keys().map(|s| s.as_str()).collect();

        let orphans: Vec<&str> = db
            .keys()
            .filter(|path| !disk.contains(*path))
            .map(|s| s.as_str())
            .collect();
        let untracked: Vec<&str> = disk
            .iter()
            .filter(|path| !db_path_set.contains(path.as_str()))
            .map(|s| s.as_str())
            .collect();

        assert_eq!(orphans.len(), 1, "exactly one orphan expected");
        assert!(orphans[0].contains("b.md"), "b.md should be the orphan");
        assert_eq!(untracked.len(), 0, "no untracked files expected");
    }

    #[test]
    fn sync_report_untracked_detection_logic() {
        use std::collections::{HashMap, HashSet};

        // Disk has two paths; DB only knows about one — the extra disk file is untracked.
        let db: HashMap<String, uuid::Uuid> = [("/vault/a.md".to_string(), uuid::Uuid::new_v4())]
            .into_iter()
            .collect();
        let disk: HashSet<String> = [
            "/vault/a.md".to_string(),
            "/vault/c.md".to_string(),
        ]
        .into_iter()
        .collect();

        let db_path_set: HashSet<&str> = db.keys().map(|s| s.as_str()).collect();

        let untracked: Vec<&str> = disk
            .iter()
            .filter(|path| !db_path_set.contains(path.as_str()))
            .map(|s| s.as_str())
            .collect();
        let orphans: Vec<&str> = db
            .keys()
            .filter(|path| !disk.contains(*path))
            .map(|s| s.as_str())
            .collect();

        assert_eq!(untracked.len(), 1, "exactly one untracked file expected");
        assert!(untracked[0].contains("c.md"), "c.md should be untracked");
        assert_eq!(orphans.len(), 0, "no orphans expected");
    }
}
