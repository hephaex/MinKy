use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
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
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
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

async fn get_system_stats(State(state): State<AppState>) -> AppResult<Json<StatsResponse>> {
    // TODO: Verify admin role
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

async fn list_backups(State(state): State<AppState>) -> AppResult<Json<BackupsResponse>> {
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

async fn get_config(State(state): State<AppState>) -> AppResult<Json<ConfigResponse>> {
    let service = AdminService::new(state.db.clone());
    let config = service.get_system_config().await?;

    Ok(Json(ConfigResponse {
        success: true,
        data: config,
    }))
}

async fn update_config(
    State(state): State<AppState>,
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
    Json(payload): Json<MaintenanceMode>,
) -> AppResult<Json<MaintenanceResponse>> {
    let service = AdminService::new(state.db.clone());
    let mode = service.set_maintenance_mode(payload).await?;

    Ok(Json(MaintenanceResponse {
        success: true,
        data: mode,
    }))
}
