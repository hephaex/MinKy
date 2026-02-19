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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_config_serde_roundtrip() {
        let config = SystemConfig {
            max_upload_size_mb: 50,
            allowed_file_types: vec!["pdf".to_string(), "md".to_string(), "txt".to_string()],
            enable_registration: true,
            require_email_verification: false,
            session_timeout_minutes: 60,
            rate_limit_requests_per_minute: 100,
        };

        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: SystemConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.max_upload_size_mb, 50);
        assert_eq!(deserialized.allowed_file_types.len(), 3);
        assert!(deserialized.enable_registration);
        assert!(!deserialized.require_email_verification);
        assert_eq!(deserialized.session_timeout_minutes, 60);
        assert_eq!(deserialized.rate_limit_requests_per_minute, 100);
    }

    #[test]
    fn test_system_config_allowed_file_types_contains() {
        let config = SystemConfig {
            max_upload_size_mb: 10,
            allowed_file_types: vec!["pdf".to_string(), "md".to_string()],
            enable_registration: false,
            require_email_verification: true,
            session_timeout_minutes: 30,
            rate_limit_requests_per_minute: 60,
        };

        assert!(config.allowed_file_types.contains(&"pdf".to_string()));
        assert!(config.allowed_file_types.contains(&"md".to_string()));
        assert!(!config.allowed_file_types.contains(&"exe".to_string()));
    }

    #[test]
    fn test_maintenance_mode_disabled_by_default() {
        let mode = MaintenanceMode {
            enabled: false,
            message: None,
            estimated_end: None,
        };

        assert!(!mode.enabled);
        assert!(mode.message.is_none());
        assert!(mode.estimated_end.is_none());
    }

    #[test]
    fn test_maintenance_mode_enabled_with_message() {
        let mode = MaintenanceMode {
            enabled: true,
            message: Some("System upgrade in progress".to_string()),
            estimated_end: None,
        };

        assert!(mode.enabled);
        assert_eq!(
            mode.message.as_deref(),
            Some("System upgrade in progress")
        );
    }

    #[test]
    fn test_maintenance_mode_serde_roundtrip() {
        let mode = MaintenanceMode {
            enabled: true,
            message: Some("Maintenance".to_string()),
            estimated_end: None,
        };

        let serialized = serde_json::to_string(&mode).unwrap();
        let deserialized: MaintenanceMode = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.enabled, mode.enabled);
        assert_eq!(deserialized.message, mode.message);
    }

    #[test]
    fn test_system_stats_active_user_rate() {
        let stats = SystemStats {
            total_users: 100,
            active_users: 75,
            total_documents: 500,
            total_storage_bytes: 1024 * 1024 * 100, // 100 MB
            database_size_bytes: 1024 * 1024 * 50,  // 50 MB
            cache_hit_rate: 0.85,
            uptime_seconds: 3600,
        };

        let active_rate = stats.active_users as f64 / stats.total_users as f64;
        assert_eq!(active_rate, 0.75);
        assert_eq!(stats.cache_hit_rate, 0.85);
        // Storage should be >= database size
        assert!(stats.total_storage_bytes >= stats.database_size_bytes);
    }
}
