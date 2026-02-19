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
}
