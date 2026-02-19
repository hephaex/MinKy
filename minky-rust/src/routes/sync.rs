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
