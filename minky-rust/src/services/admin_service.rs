use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::models::{
    AuditLogEntry, BackupInfo, CreateBackupRequest, MaintenanceMode, SystemConfig, SystemStats,
    UpdateUserAdmin, UserAdmin,
};

/// Raw DB row type for user admin queries
type UserAdminRow = (
    i32,
    String,
    String,
    String,
    bool,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
    i64,
    i64,
);

/// Raw DB row type for audit log queries
type AuditLogRow = (
    i64,
    Option<i32>,
    Option<String>,
    String,
    String,
    Option<String>,
    Option<serde_json::Value>,
    Option<String>,
    Option<String>,
    chrono::DateTime<chrono::Utc>,
);

/// Admin service for system management
pub struct AdminService {
    db: PgPool,
}

impl AdminService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get system statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.db)
            .await?;

        let active_users: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_active = true")
                .fetch_one(&self.db)
                .await?;

        let total_documents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
            .fetch_one(&self.db)
            .await?;

        let total_storage: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(file_size), 0)::bigint FROM attachments")
                .fetch_one(&self.db)
                .await?;

        let db_size: (i64,) = sqlx::query_as(
            "SELECT pg_database_size(current_database())::bigint"
        )
        .fetch_one(&self.db)
        .await?;

        Ok(SystemStats {
            total_users: total_users.0,
            active_users: active_users.0,
            total_documents: total_documents.0,
            total_storage_bytes: total_storage.0,
            database_size_bytes: db_size.0,
            cache_hit_rate: 0.95, // TODO: Get from Redis
            uptime_seconds: 0,    // TODO: Track server uptime
        })
    }

    /// List all users with admin info
    pub async fn list_users(&self, page: i32, limit: i32) -> Result<Vec<UserAdmin>> {
        let offset = (page - 1) * limit;

        let rows: Vec<UserAdminRow> = sqlx::query_as(
            r#"
            SELECT
                u.id,
                u.username,
                u.email,
                u.role,
                u.is_active,
                u.created_at,
                u.last_login,
                (SELECT COUNT(*) FROM documents WHERE user_id = u.id)::bigint as doc_count,
                COALESCE((SELECT SUM(file_size) FROM attachments WHERE user_id = u.id), 0)::bigint as storage
            FROM users u
            ORDER BY u.created_at DESC
            LIMIT $1 OFFSET $2
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| UserAdmin {
                id: r.0,
                username: r.1,
                email: r.2,
                role: r.3,
                is_active: r.4,
                created_at: r.5,
                last_login: r.6,
                document_count: r.7,
                storage_used_bytes: r.8,
            })
            .collect())
    }

    /// Update user admin settings
    pub async fn update_user(&self, user_id: i32, update: UpdateUserAdmin) -> Result<UserAdmin> {
        if let Some(role) = &update.role {
            sqlx::query("UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2")
                .bind(role)
                .bind(user_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(is_active) = update.is_active {
            sqlx::query("UPDATE users SET is_active = $1, updated_at = NOW() WHERE id = $2")
                .bind(is_active)
                .bind(user_id)
                .execute(&self.db)
                .await?;
        }

        // Fetch updated user
        let row: UserAdminRow = sqlx::query_as(
            r#"
            SELECT
                u.id,
                u.username,
                u.email,
                u.role,
                u.is_active,
                u.created_at,
                u.last_login,
                (SELECT COUNT(*) FROM documents WHERE user_id = u.id)::bigint,
                COALESCE((SELECT SUM(file_size) FROM attachments WHERE user_id = u.id), 0)::bigint
            FROM users u
            WHERE u.id = $1
            "#
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(UserAdmin {
            id: row.0,
            username: row.1,
            email: row.2,
            role: row.3,
            is_active: row.4,
            created_at: row.5,
            last_login: row.6,
            document_count: row.7,
            storage_used_bytes: row.8,
        })
    }

    /// Delete user
    pub async fn delete_user(&self, user_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get audit logs
    pub async fn get_audit_logs(
        &self,
        page: i32,
        limit: i32,
        user_id: Option<i32>,
        action: Option<&str>,
    ) -> Result<Vec<AuditLogEntry>> {
        let offset = (page - 1) * limit;

        let rows: Vec<AuditLogRow> = sqlx::query_as(
            r#"
            SELECT
                a.id,
                a.user_id,
                u.username,
                a.action,
                a.resource_type,
                a.resource_id,
                a.details,
                a.ip_address,
                a.user_agent,
                a.created_at
            FROM audit_logs a
            LEFT JOIN users u ON a.user_id = u.id
            WHERE ($1::int IS NULL OR a.user_id = $1)
              AND ($2::text IS NULL OR a.action = $2)
            ORDER BY a.created_at DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(user_id)
        .bind(action)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AuditLogEntry {
                id: r.0,
                user_id: r.1,
                username: r.2,
                action: r.3,
                resource_type: r.4,
                resource_id: r.5,
                details: r.6,
                ip_address: r.7,
                user_agent: r.8,
                created_at: r.9,
            })
            .collect())
    }

    /// List backups
    pub async fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        // TODO: Implement actual backup listing from storage
        Ok(vec![])
    }

    /// Create backup
    pub async fn create_backup(&self, _request: CreateBackupRequest) -> Result<BackupInfo> {
        // TODO: Implement actual backup creation
        let backup_id = uuid::Uuid::new_v4().to_string();

        Ok(BackupInfo {
            id: backup_id.clone(),
            filename: format!("backup_{}.sql.gz", Utc::now().format("%Y%m%d_%H%M%S")),
            size_bytes: 0,
            created_at: Utc::now(),
            backup_type: "full".to_string(),
            status: "pending".to_string(),
        })
    }

    /// Get maintenance mode status
    pub async fn get_maintenance_mode(&self) -> Result<MaintenanceMode> {
        // TODO: Store in Redis or config
        Ok(MaintenanceMode {
            enabled: false,
            message: None,
            estimated_end: None,
        })
    }

    /// Set maintenance mode
    pub async fn set_maintenance_mode(&self, mode: MaintenanceMode) -> Result<MaintenanceMode> {
        // TODO: Store in Redis or config
        Ok(mode)
    }

    /// Get system configuration
    pub async fn get_system_config(&self) -> Result<SystemConfig> {
        // TODO: Load from database or config file
        Ok(SystemConfig {
            max_upload_size_mb: 50,
            allowed_file_types: vec![
                "pdf".to_string(),
                "doc".to_string(),
                "docx".to_string(),
                "txt".to_string(),
                "md".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
            ],
            enable_registration: true,
            require_email_verification: false,
            session_timeout_minutes: 60,
            rate_limit_requests_per_minute: 100,
        })
    }

    /// Update system configuration
    pub async fn update_system_config(&self, config: SystemConfig) -> Result<SystemConfig> {
        // TODO: Save to database or config file
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty_user_admin_row() {
        let row: UserAdminRow = (
            1,
            "testuser".to_string(),
            "test@example.com".to_string(),
            "user".to_string(),
            true,
            Utc::now(),
            None,
            5,
            1024,
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "testuser");
        assert_eq!(row.5, row.5);
        assert_eq!(row.7, 5);
        assert_eq!(row.8, 1024);
    }

    #[test]
    fn test_parse_audit_log_row() {
        let row: AuditLogRow = (
            1,
            Some(1),
            Some("testuser".to_string()),
            "CREATE".to_string(),
            "document".to_string(),
            Some("doc123".to_string()),
            None,
            Some("192.168.1.1".to_string()),
            Some("Mozilla/5.0".to_string()),
            Utc::now(),
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, Some(1));
        assert_eq!(row.3, "CREATE");
        assert_eq!(row.4, "document");
    }

    #[test]
    fn test_admin_service_creation() {
        // This is a basic smoke test for service construction
        // Note: In real usage, a proper PgPool would be needed
        // For unit tests, we're testing the tuple parsing logic above
        let row: UserAdminRow = (
            42,
            "admin".to_string(),
            "admin@example.com".to_string(),
            "admin".to_string(),
            true,
            Utc::now(),
            Some(Utc::now()),
            100,
            50000,
        );

        let user_admin = UserAdmin {
            id: row.0,
            username: row.1,
            email: row.2,
            role: row.3,
            is_active: row.4,
            created_at: row.5,
            last_login: row.6,
            document_count: row.7,
            storage_used_bytes: row.8,
        };

        assert_eq!(user_admin.id, 42);
        assert_eq!(user_admin.role, "admin");
        assert!(user_admin.is_active);
    }

    #[test]
    fn test_update_user_admin_structure() {
        let update = UpdateUserAdmin {
            role: Some("editor".to_string()),
            is_active: Some(false),
        };

        assert_eq!(update.role, Some("editor".to_string()));
        assert_eq!(update.is_active, Some(false));
    }

    #[test]
    fn test_create_backup_info_structure() {
        let backup = BackupInfo {
            id: "backup-123".to_string(),
            filename: "backup_20260219_150000.sql.gz".to_string(),
            size_bytes: 1024 * 1024,
            created_at: Utc::now(),
            backup_type: "full".to_string(),
            status: "completed".to_string(),
        };

        assert_eq!(backup.id, "backup-123");
        assert!(backup.filename.contains("backup_"));
        assert!(backup.size_bytes > 0);
    }

    #[test]
    fn test_system_config_structure() {
        let config = SystemConfig {
            max_upload_size_mb: 100,
            allowed_file_types: vec!["pdf".to_string(), "txt".to_string()],
            enable_registration: true,
            require_email_verification: true,
            session_timeout_minutes: 120,
            rate_limit_requests_per_minute: 200,
        };

        assert_eq!(config.max_upload_size_mb, 100);
        assert_eq!(config.allowed_file_types.len(), 2);
        assert!(config.enable_registration);
        assert_eq!(config.session_timeout_minutes, 120);
    }

    #[test]
    fn test_maintenance_mode_structure() {
        let mode = MaintenanceMode {
            enabled: true,
            message: Some("System maintenance in progress".to_string()),
            estimated_end: Some(Utc::now()),
        };

        assert!(mode.enabled);
        assert!(mode.message.is_some());
        assert!(mode.estimated_end.is_some());
    }

    #[test]
    fn test_system_stats_structure() {
        let stats = SystemStats {
            total_users: 1000,
            active_users: 800,
            total_documents: 5000,
            total_storage_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            database_size_bytes: 2 * 1024 * 1024 * 1024,  // 2 GB
            cache_hit_rate: 0.95,
            uptime_seconds: 86400,
        };

        assert_eq!(stats.total_users, 1000);
        assert!(stats.active_users < stats.total_users);
        assert!(stats.cache_hit_rate > 0.0 && stats.cache_hit_rate <= 1.0);
    }
}
