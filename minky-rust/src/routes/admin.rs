use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AdminUser,
    models::{
        AuditLogEntry, BackupInfo, CreateBackupRequest, MaintenanceMode, SystemConfig,
        SystemStats, UpdateUserAdmin, UserAdmin,
    },
    services::AdminService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/stats", get(get_system_stats))
        .route("/users", get(list_users))
        .route("/users/{id}", get(get_user))
        .route("/users/{id}", put(update_user))
        .route("/users/{id}", delete(delete_user))
        .route("/audit-logs", get(get_audit_logs))
        .route("/backups", get(list_backups))
        .route("/backups", post(create_backup))
        .route("/config", get(get_config))
        .route("/config", put(update_config))
        .route("/maintenance", get(get_maintenance_mode))
        .route("/maintenance", put(set_maintenance_mode))
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub success: bool,
    pub data: SystemStats,
}

async fn get_system_stats(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<StatsResponse>> {
    let service = AdminService::new(state.db.clone());
    let stats = service.get_system_stats().await?;

    Ok(Json(StatsResponse {
        success: true,
        data: stats,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub success: bool,
    pub data: Vec<UserAdmin>,
}

async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ListUsersQuery>,
) -> AppResult<Json<UsersResponse>> {
    let service = AdminService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20).min(100);

    let users = service.list_users(page, limit).await?;

    Ok(Json(UsersResponse {
        success: true,
        data: users,
    }))
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub success: bool,
    pub data: UserAdmin,
}

async fn get_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
) -> AppResult<Json<UserResponse>> {
    let service = AdminService::new(state.db.clone());
    let users = service.list_users(1, 1000).await?;

    let user = users
        .into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| crate::error::AppError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse {
        success: true,
        data: user,
    }))
}

async fn update_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUserAdmin>,
) -> AppResult<Json<UserResponse>> {
    let service = AdminService::new(state.db.clone());
    let user = service.update_user(id, payload).await?;

    Ok(Json(UserResponse {
        success: true,
        data: user,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_user(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = AdminService::new(state.db.clone());
    service.delete_user(id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "User deleted".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct AuditLogsQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub user_id: Option<i32>,
    pub action: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogsResponse {
    pub success: bool,
    pub data: Vec<AuditLogEntry>,
}

async fn get_audit_logs(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<AuditLogsQuery>,
) -> AppResult<Json<AuditLogsResponse>> {
    let service = AdminService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50).min(200);

    let logs = service
        .get_audit_logs(page, limit, query.user_id, query.action.as_deref())
        .await?;

    Ok(Json(AuditLogsResponse {
        success: true,
        data: logs,
    }))
}

#[derive(Debug, Serialize)]
pub struct BackupsResponse {
    pub success: bool,
    pub data: Vec<BackupInfo>,
}

async fn list_backups(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<BackupsResponse>> {
    let service = AdminService::new(state.db.clone());
    let backups = service.list_backups().await?;

    Ok(Json(BackupsResponse {
        success: true,
        data: backups,
    }))
}

#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub success: bool,
    pub data: BackupInfo,
}

async fn create_backup(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<CreateBackupRequest>,
) -> AppResult<Json<BackupResponse>> {
    let service = AdminService::new(state.db.clone());
    let backup = service.create_backup(payload).await?;

    Ok(Json(BackupResponse {
        success: true,
        data: backup,
    }))
}

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub success: bool,
    pub data: SystemConfig,
}

async fn get_config(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<ConfigResponse>> {
    let service = AdminService::new(state.db.clone());
    let config = service.get_system_config().await?;

    Ok(Json(ConfigResponse {
        success: true,
        data: config,
    }))
}

async fn update_config(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<SystemConfig>,
) -> AppResult<Json<ConfigResponse>> {
    let service = AdminService::new(state.db.clone());
    let config = service.update_system_config(payload).await?;

    Ok(Json(ConfigResponse {
        success: true,
        data: config,
    }))
}

#[derive(Debug, Serialize)]
pub struct MaintenanceResponse {
    pub success: bool,
    pub data: MaintenanceMode,
}

async fn get_maintenance_mode(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<MaintenanceResponse>> {
    let service = AdminService::new(state.db.clone());
    let mode = service.get_maintenance_mode().await?;

    Ok(Json(MaintenanceResponse {
        success: true,
        data: mode,
    }))
}

async fn set_maintenance_mode(
    State(state): State<AppState>,
    _admin: AdminUser,
    Json(payload): Json<MaintenanceMode>,
) -> AppResult<Json<MaintenanceResponse>> {
    let service = AdminService::new(state.db.clone());
    let mode = service.set_maintenance_mode(payload).await?;

    Ok(Json(MaintenanceResponse {
        success: true,
        data: mode,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ListUsersQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_list_users_query_deserialization() {
        let json = r#"{"page": 2, "limit": 50}"#;
        let query: ListUsersQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, Some(2));
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_list_users_query_default() {
        let json = r#"{}"#;
        let query: ListUsersQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, None);
        assert_eq!(query.limit, None);
    }

    #[test]
    fn test_list_users_pagination_logic() {
        let test_cases = vec![
            ((None, None), (1, 20)),       // defaults
            ((Some(3), Some(50)), (3, 50)), // specified
            ((Some(1), Some(200)), (1, 100)), // limit clamped
        ];

        for ((page_input, limit_input), (expected_page, expected_limit)) in test_cases {
            let page = page_input.unwrap_or(1);
            let limit = limit_input.unwrap_or(20).min(100);
            assert_eq!((page, limit), (expected_page, expected_limit));
        }
    }

    // -------------------------------------------------------------------------
    // AuditLogsQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_audit_logs_query_deserialization() {
        let json = r#"{"page": 1, "limit": 100, "user_id": 5, "action": "create"}"#;
        let query: AuditLogsQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page, Some(1));
        assert_eq!(query.limit, Some(100));
        assert_eq!(query.user_id, Some(5));
        assert_eq!(query.action, Some("create".to_string()));
    }

    #[test]
    fn test_audit_logs_query_empty() {
        let json = r#"{}"#;
        let query: AuditLogsQuery = serde_json::from_str(json).unwrap();
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.user_id.is_none());
        assert!(query.action.is_none());
    }

    #[test]
    fn test_audit_logs_limit_clamping() {
        let test_cases = vec![
            (None, 50),      // default
            (Some(100), 100), // below max
            (Some(200), 200), // at max
            (Some(300), 200), // above max, clamped
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(50).min(200);
            assert_eq!(limit, expected);
        }
    }

    // -------------------------------------------------------------------------
    // Response serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_stats_response_serialization() {
        let response = StatsResponse {
            success: true,
            data: SystemStats {
                total_users: 100,
                active_users: 50,
                total_documents: 1000,
                total_storage_bytes: 1024 * 1024 * 100,
                database_size_bytes: 1024 * 1024 * 50,
                cache_hit_rate: 0.85,
                uptime_seconds: 86400,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total_users\":100"));
        assert!(json.contains("\"cache_hit_rate\":0.85"));
    }

    #[test]
    fn test_users_response_serialization() {
        let response = UsersResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":[]"));
    }

    #[test]
    fn test_user_response_serialization() {
        let now = chrono::Utc::now();
        let response = UserResponse {
            success: true,
            data: UserAdmin {
                id: 1,
                username: "testuser".to_string(),
                email: "test@example.com".to_string(),
                role: "admin".to_string(),
                is_active: true,
                created_at: now,
                last_login: Some(now),
                document_count: 25,
                storage_used_bytes: 1024 * 1024,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"username\":\"testuser\""));
        assert!(json.contains("\"role\":\"admin\""));
    }

    #[test]
    fn test_delete_response_serialization() {
        let response = DeleteResponse {
            success: true,
            message: "User deleted".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"User deleted\""));
    }

    #[test]
    fn test_audit_logs_response_serialization() {
        let response = AuditLogsResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_backups_response_serialization() {
        let response = BackupsResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_backup_response_serialization() {
        let now = chrono::Utc::now();
        let response = BackupResponse {
            success: true,
            data: BackupInfo {
                id: "backup-001".to_string(),
                filename: "backup_2026-03-08.sql.gz".to_string(),
                size_bytes: 1024 * 1024 * 10,
                created_at: now,
                backup_type: "full".to_string(),
                status: "completed".to_string(),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"id\":\"backup-001\""));
        assert!(json.contains("\"backup_type\":\"full\""));
    }

    #[test]
    fn test_config_response_serialization() {
        let response = ConfigResponse {
            success: true,
            data: SystemConfig {
                max_upload_size_mb: 100,
                allowed_file_types: vec!["pdf".to_string(), "md".to_string()],
                enable_registration: true,
                require_email_verification: false,
                session_timeout_minutes: 60,
                rate_limit_requests_per_minute: 100,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"max_upload_size_mb\":100"));
        assert!(json.contains("\"enable_registration\":true"));
    }

    #[test]
    fn test_maintenance_response_serialization() {
        let now = chrono::Utc::now();
        let response = MaintenanceResponse {
            success: true,
            data: MaintenanceMode {
                enabled: true,
                message: Some("System maintenance in progress".to_string()),
                estimated_end: Some(now),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"message\":\"System maintenance in progress\""));
    }

    #[test]
    fn test_maintenance_response_disabled() {
        let response = MaintenanceResponse {
            success: true,
            data: MaintenanceMode {
                enabled: false,
                message: None,
                estimated_end: None,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"enabled\":false"));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_routes_creation() {
        let _router: Router<AppState> = routes();
        // Should be creatable without panicking
    }
}
