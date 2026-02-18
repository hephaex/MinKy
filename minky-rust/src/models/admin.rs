use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System statistics
#[derive(Debug, Serialize)]
pub struct SystemStats {
    pub total_users: i64,
    pub active_users: i64,
    pub total_documents: i64,
    pub total_storage_bytes: i64,
    pub database_size_bytes: i64,
    pub cache_hit_rate: f64,
    pub uptime_seconds: i64,
}

/// User management data
#[derive(Debug, Serialize)]
pub struct UserAdmin {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub document_count: i64,
    pub storage_used_bytes: i64,
}

/// Update user request
#[derive(Debug, Deserialize)]
pub struct UpdateUserAdmin {
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

/// System configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemConfig {
    pub max_upload_size_mb: i32,
    pub allowed_file_types: Vec<String>,
    pub enable_registration: bool,
    pub require_email_verification: bool,
    pub session_timeout_minutes: i32,
    pub rate_limit_requests_per_minute: i32,
}

/// Audit log entry
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Backup info
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub filename: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub backup_type: String,
    pub status: String,
}

/// Create backup request
#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    pub backup_type: Option<String>,
    pub include_attachments: Option<bool>,
}

/// Maintenance mode
#[derive(Debug, Serialize, Deserialize)]
pub struct MaintenanceMode {
    pub enabled: bool,
    pub message: Option<String>,
    pub estimated_end: Option<DateTime<Utc>>,
}
