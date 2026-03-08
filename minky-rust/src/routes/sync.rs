use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
use crate::models::{
    CreateSyncConfig, ResolveConflictRequest, SyncConfig, SyncConflict, SyncHistoryEntry, SyncJob,
    SyncStats,
};
use crate::services::SyncService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i32>,
}

/// List sync configurations
async fn list_configs(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<SyncConfig>>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .list_configs(auth_user.id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get sync configuration
async fn get_config(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
) -> Result<Json<SyncConfig>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .get_config(config_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Config not found".to_string()))
}

/// Create sync configuration
async fn create_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(create): Json<CreateSyncConfig>,
) -> Result<Json<SyncConfig>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .create_config(auth_user.id, create)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete sync configuration
async fn delete_config(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(config_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .delete_config(auth_user.id, config_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Start sync
async fn start_sync(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
) -> Result<Json<SyncJob>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .start_sync(config_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get sync job status
async fn get_job_status(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(job_id): Path<String>,
) -> Result<Json<SyncJob>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .get_job_status(&job_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Job not found".to_string()))
}

/// Get sync history
async fn get_history(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<SyncHistoryEntry>>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());
    let limit = query.limit.unwrap_or(50);

    service
        .get_history(config_id, limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get pending conflicts
async fn get_conflicts(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
) -> Result<Json<Vec<SyncConflict>>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .get_conflicts(config_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Resolve conflict
async fn resolve_conflict(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
    Json(request): Json<ResolveConflictRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .resolve_conflict(config_id, request)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get sync statistics
async fn get_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(config_id): Path<i32>,
) -> Result<Json<SyncStats>, (StatusCode, String)> {
    let service = SyncService::new(state.db.clone());

    service
        .get_stats(config_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/configs", get(list_configs).post(create_config))
        .route("/configs/{config_id}", get(get_config).delete(delete_config))
        .route("/configs/{config_id}/sync", post(start_sync))
        .route("/configs/{config_id}/history", get(get_history))
        .route("/configs/{config_id}/conflicts", get(get_conflicts))
        .route("/configs/{config_id}/conflicts/resolve", post(resolve_conflict))
        .route("/configs/{config_id}/stats", get(get_stats))
        .route("/jobs/{job_id}", get(get_job_status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ConflictResolution, ConflictType, SyncDirection, SyncError, SyncProvider, SyncStatus,
    };

    // -------------------------------------------------------------------------
    // HistoryQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_history_query_deserialization() {
        let json = r#"{"limit": 100}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(100));
    }

    #[test]
    fn test_history_query_empty() {
        let json = r#"{}"#;
        let query: HistoryQuery = serde_json::from_str(json).unwrap();
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_history_query_default_logic() {
        let test_cases = vec![
            (None, 50),    // default
            (Some(25), 25), // specified
            (Some(100), 100),
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(50);
            assert_eq!(limit, expected);
        }
    }

    // -------------------------------------------------------------------------
    // SyncConfig serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sync_config_serialization() {
        let now = chrono::Utc::now();
        let config = SyncConfig {
            id: 1,
            name: "S3 Backup".to_string(),
            provider: SyncProvider::S3,
            remote_path: "s3://bucket/docs".to_string(),
            local_path: Some("/home/user/docs".to_string()),
            credentials: None,
            sync_direction: SyncDirection::Bidirectional,
            auto_sync: true,
            sync_interval_minutes: 60,
            include_attachments: true,
            is_active: true,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"name\":\"S3 Backup\""));
        assert!(json.contains("\"provider\":\"s3\""));
        assert!(json.contains("\"auto_sync\":true"));
    }

    #[test]
    fn test_sync_config_all_providers() {
        let providers = vec![
            (SyncProvider::Local, "local"),
            (SyncProvider::S3, "s3"),
            (SyncProvider::GCS, "gcs"),
            (SyncProvider::Azure, "azure"),
            (SyncProvider::Dropbox, "dropbox"),
            (SyncProvider::GoogleDrive, "googledrive"),
            (SyncProvider::OneDrive, "onedrive"),
            (SyncProvider::WebDAV, "webdav"),
        ];

        for (provider, expected_str) in providers {
            let json = serde_json::to_value(&provider).unwrap();
            assert_eq!(json, expected_str);
        }
    }

    // -------------------------------------------------------------------------
    // CreateSyncConfig tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_sync_config_minimal() {
        let json = r#"{
            "name": "My Sync",
            "provider": "local",
            "remote_path": "/remote/path"
        }"#;
        let config: CreateSyncConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "My Sync");
        assert!(matches!(config.provider, SyncProvider::Local));
        assert!(config.auto_sync.is_none());
    }

    #[test]
    fn test_create_sync_config_full() {
        let json = r#"{
            "name": "Cloud Backup",
            "provider": "s3",
            "remote_path": "s3://my-bucket/docs",
            "local_path": "/local/docs",
            "credentials": {"access_key": "xxx", "secret_key": "yyy"},
            "sync_direction": "push",
            "auto_sync": true,
            "sync_interval_minutes": 30,
            "include_attachments": false
        }"#;
        let config: CreateSyncConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "Cloud Backup");
        assert_eq!(config.auto_sync, Some(true));
        assert_eq!(config.sync_interval_minutes, Some(30));
        assert!(config.credentials.is_some());
    }

    // -------------------------------------------------------------------------
    // SyncJob serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sync_job_serialization() {
        let now = chrono::Utc::now();
        let job = SyncJob {
            id: "job-123".to_string(),
            config_id: 1,
            status: SyncStatus::Syncing,
            direction: SyncDirection::Pull,
            files_total: 100,
            files_synced: 50,
            files_failed: 2,
            bytes_transferred: 1024 * 1024 * 50,
            conflicts: vec![],
            errors: vec![],
            started_at: now,
            completed_at: None,
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"id\":\"job-123\""));
        assert!(json.contains("\"status\":\"syncing\""));
        assert!(json.contains("\"files_total\":100"));
    }

    #[test]
    fn test_sync_job_with_conflicts() {
        let now = chrono::Utc::now();
        let job = SyncJob {
            id: "job-456".to_string(),
            config_id: 2,
            status: SyncStatus::Conflict,
            direction: SyncDirection::Bidirectional,
            files_total: 50,
            files_synced: 48,
            files_failed: 0,
            bytes_transferred: 1024 * 100,
            conflicts: vec![SyncConflict {
                file_path: "/docs/readme.md".to_string(),
                local_modified: now,
                remote_modified: now,
                conflict_type: ConflictType::BothModified,
                resolution: None,
            }],
            errors: vec![],
            started_at: now,
            completed_at: None,
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"status\":\"conflict\""));
        assert!(json.contains("\"/docs/readme.md\""));
    }

    #[test]
    fn test_sync_job_with_errors() {
        let now = chrono::Utc::now();
        let job = SyncJob {
            id: "job-789".to_string(),
            config_id: 3,
            status: SyncStatus::Failed,
            direction: SyncDirection::Push,
            files_total: 10,
            files_synced: 5,
            files_failed: 5,
            bytes_transferred: 1024,
            conflicts: vec![],
            errors: vec![SyncError {
                file_path: "/docs/large-file.pdf".to_string(),
                error_code: "SIZE_LIMIT".to_string(),
                message: "File exceeds size limit".to_string(),
                retryable: false,
            }],
            started_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error_code\":\"SIZE_LIMIT\""));
    }

    // -------------------------------------------------------------------------
    // SyncConflict serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sync_conflict_serialization() {
        let now = chrono::Utc::now();
        let conflict = SyncConflict {
            file_path: "/docs/notes.md".to_string(),
            local_modified: now,
            remote_modified: now,
            conflict_type: ConflictType::DeletedRemotely,
            resolution: Some(ConflictResolution::UseLocal),
        };
        let json = serde_json::to_string(&conflict).unwrap();
        assert!(json.contains("\"conflict_type\":\"deleted_remotely\""));
        assert!(json.contains("\"resolution\":\"use_local\""));
    }

    // -------------------------------------------------------------------------
    // SyncHistoryEntry serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sync_history_entry_serialization() {
        let now = chrono::Utc::now();
        let entry = SyncHistoryEntry {
            id: 1,
            config_id: 5,
            status: SyncStatus::Completed,
            files_synced: 100,
            files_failed: 0,
            bytes_transferred: 1024 * 1024 * 200,
            duration_seconds: 300,
            started_at: now,
            completed_at: now,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"duration_seconds\":300"));
    }

    // -------------------------------------------------------------------------
    // ResolveConflictRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_resolve_conflict_request_use_remote() {
        let json = r#"{"file_path": "/docs/file.txt", "resolution": "use_remote"}"#;
        let request: ResolveConflictRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.file_path, "/docs/file.txt");
        assert!(matches!(request.resolution, ConflictResolution::UseRemote));
    }

    #[test]
    fn test_resolve_conflict_request_merge() {
        let json = r#"{"file_path": "/docs/merged.md", "resolution": "merge"}"#;
        let request: ResolveConflictRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.resolution, ConflictResolution::Merge));
    }

    #[test]
    fn test_resolve_conflict_request_skip() {
        let json = r#"{"file_path": "/docs/skipped.md", "resolution": "skip"}"#;
        let request: ResolveConflictRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.resolution, ConflictResolution::Skip));
    }

    // -------------------------------------------------------------------------
    // SyncStats serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sync_stats_serialization() {
        let now = chrono::Utc::now();
        let stats = SyncStats {
            total_syncs: 100,
            successful_syncs: 95,
            failed_syncs: 5,
            total_files_synced: 5000,
            total_bytes_transferred: 1024 * 1024 * 1024 * 10,
            last_sync: Some(now),
            next_scheduled_sync: Some(now + chrono::Duration::hours(1)),
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_syncs\":100"));
        assert!(json.contains("\"successful_syncs\":95"));
        assert!(json.contains("\"failed_syncs\":5"));
    }

    #[test]
    fn test_sync_stats_no_scheduled_sync() {
        let stats = SyncStats {
            total_syncs: 0,
            successful_syncs: 0,
            failed_syncs: 0,
            total_files_synced: 0,
            total_bytes_transferred: 0,
            last_sync: None,
            next_scheduled_sync: None,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_syncs\":0"));
        assert!(json.contains("\"last_sync\":null"));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_router_creation() {
        let _router: Router<AppState> = router();
        // Should be creatable without panicking
    }
}
