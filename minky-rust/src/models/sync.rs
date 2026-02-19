use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sync provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncProvider {
    Local,
    S3,
    GCS,
    Azure,
    Dropbox,
    GoogleDrive,
    OneDrive,
    WebDAV,
}

/// Sync status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    #[default]
    Pending,
    Idle,
    Syncing,
    Completed,
    Failed,
    Conflict,
}

/// Sync configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncConfig {
    pub id: i32,
    pub name: String,
    pub provider: SyncProvider,
    pub remote_path: String,
    pub local_path: Option<String>,
    pub credentials: Option<serde_json::Value>,
    pub sync_direction: SyncDirection,
    pub auto_sync: bool,
    pub sync_interval_minutes: i32,
    pub include_attachments: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync direction
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    #[default]
    Bidirectional,
    Push,
    Pull,
}

/// Create sync config request
#[derive(Debug, Deserialize)]
pub struct CreateSyncConfig {
    pub name: String,
    pub provider: SyncProvider,
    pub remote_path: String,
    pub local_path: Option<String>,
    pub credentials: Option<serde_json::Value>,
    pub sync_direction: Option<SyncDirection>,
    pub auto_sync: Option<bool>,
    pub sync_interval_minutes: Option<i32>,
    pub include_attachments: Option<bool>,
}

/// Sync job
#[derive(Debug, Serialize)]
pub struct SyncJob {
    pub id: String,
    pub config_id: i32,
    pub status: SyncStatus,
    pub direction: SyncDirection,
    pub files_total: i32,
    pub files_synced: i32,
    pub files_failed: i32,
    pub bytes_transferred: i64,
    pub conflicts: Vec<SyncConflict>,
    pub errors: Vec<SyncError>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Sync conflict
#[derive(Debug, Clone, Serialize)]
pub struct SyncConflict {
    pub file_path: String,
    pub local_modified: DateTime<Utc>,
    pub remote_modified: DateTime<Utc>,
    pub conflict_type: ConflictType,
    pub resolution: Option<ConflictResolution>,
}

/// Conflict type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    BothModified,
    DeletedLocally,
    DeletedRemotely,
    TypeMismatch,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    UseLocal,
    UseRemote,
    Merge,
    Skip,
    Rename,
}

/// Sync error
#[derive(Debug, Clone, Serialize)]
pub struct SyncError {
    pub file_path: String,
    pub error_code: String,
    pub message: String,
    pub retryable: bool,
}

/// Sync file info
#[derive(Debug, Serialize)]
pub struct SyncFileInfo {
    pub path: String,
    pub size: i64,
    pub modified: DateTime<Utc>,
    pub checksum: String,
    pub sync_status: FileSyncStatus,
    pub last_synced: Option<DateTime<Utc>>,
}

/// File sync status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileSyncStatus {
    Synced,
    Pending,
    Modified,
    Conflict,
    Error,
}

/// Sync history entry
#[derive(Debug, Serialize)]
pub struct SyncHistoryEntry {
    pub id: i64,
    pub config_id: i32,
    pub status: SyncStatus,
    pub files_synced: i32,
    pub files_failed: i32,
    pub bytes_transferred: i64,
    pub duration_seconds: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

/// Resolve conflict request
#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub file_path: String,
    pub resolution: ConflictResolution,
}

/// Sync stats
#[derive(Debug, Serialize)]
pub struct SyncStats {
    pub total_syncs: i64,
    pub successful_syncs: i64,
    pub failed_syncs: i64,
    pub total_files_synced: i64,
    pub total_bytes_transferred: i64,
    pub last_sync: Option<DateTime<Utc>>,
    pub next_scheduled_sync: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_default_is_pending() {
        assert!(matches!(SyncStatus::default(), SyncStatus::Pending));
    }

    #[test]
    fn test_sync_direction_default_is_bidirectional() {
        assert!(matches!(SyncDirection::default(), SyncDirection::Bidirectional));
    }

    #[test]
    fn test_sync_status_serde_roundtrip_all_variants() {
        let variants = [
            SyncStatus::Pending,
            SyncStatus::Idle,
            SyncStatus::Syncing,
            SyncStatus::Completed,
            SyncStatus::Failed,
            SyncStatus::Conflict,
        ];
        for status in &variants {
            let json = serde_json::to_string(status).unwrap();
            let back: SyncStatus = serde_json::from_str(&json).unwrap();
            // Verify roundtrip by re-serialising
            assert_eq!(serde_json::to_string(&back).unwrap(), json);
        }
    }

    #[test]
    fn test_sync_provider_serde_lowercase() {
        let provider = SyncProvider::S3;
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json, "s3");
    }

    #[test]
    fn test_sync_provider_google_drive_lowercase() {
        let provider = SyncProvider::GoogleDrive;
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json, "googledrive");
    }

    #[test]
    fn test_sync_provider_web_dav() {
        let provider = SyncProvider::WebDAV;
        let json = serde_json::to_value(&provider).unwrap();
        assert_eq!(json, "webdav");
    }

    #[test]
    fn test_sync_direction_push_serde() {
        let dir = SyncDirection::Push;
        let json = serde_json::to_value(&dir).unwrap();
        assert_eq!(json, "push");
    }

    #[test]
    fn test_conflict_type_both_modified_snake_case() {
        let ct = ConflictType::BothModified;
        let json = serde_json::to_value(&ct).unwrap();
        assert_eq!(json, "both_modified");
    }

    #[test]
    fn test_conflict_type_deleted_locally_snake_case() {
        let ct = ConflictType::DeletedLocally;
        let json = serde_json::to_value(&ct).unwrap();
        assert_eq!(json, "deleted_locally");
    }

    #[test]
    fn test_conflict_resolution_use_local_snake_case() {
        let cr = ConflictResolution::UseLocal;
        let json = serde_json::to_value(&cr).unwrap();
        assert_eq!(json, "use_local");
    }

    #[test]
    fn test_conflict_resolution_use_remote_snake_case() {
        let cr = ConflictResolution::UseRemote;
        let json = serde_json::to_value(&cr).unwrap();
        assert_eq!(json, "use_remote");
    }

    #[test]
    fn test_file_sync_status_synced_lowercase() {
        let s = FileSyncStatus::Synced;
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json, "synced");
    }

    #[test]
    fn test_resolve_conflict_request_deserialize() {
        let json = r#"{"file_path": "/docs/readme.md", "resolution": "use_local"}"#;
        let req: ResolveConflictRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.file_path, "/docs/readme.md");
        assert!(matches!(req.resolution, ConflictResolution::UseLocal));
    }

    #[test]
    fn test_create_sync_config_optional_fields_absent() {
        let json = r#"{
            "name": "My S3 backup",
            "provider": "s3",
            "remote_path": "s3://my-bucket/docs"
        }"#;
        let config: CreateSyncConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "My S3 backup");
        assert!(config.local_path.is_none());
        assert!(config.sync_direction.is_none());
        assert!(config.auto_sync.is_none());
    }
}
