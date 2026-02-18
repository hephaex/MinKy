use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    models::{
        ApiKey, ApiKeyWithSecret, BlockIpRequest, CreateApiKeyRequest, IpBlock, SecurityEvent,
        SecurityEventType, SecurityReport, SecuritySettings, SessionInfo, Severity,
    },
    services::SecurityService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/events", get(get_security_events))
        .route("/ip-blocks", get(list_blocked_ips))
        .route("/ip-blocks", post(block_ip))
        .route("/ip-blocks/{ip}", delete(unblock_ip))
        .route("/api-keys", get(list_api_keys))
        .route("/api-keys", post(create_api_key))
        .route("/api-keys/{id}", delete(revoke_api_key))
        .route("/sessions", get(get_sessions))
        .route("/sessions/{id}", delete(revoke_session))
        .route("/sessions/revoke-all", post(revoke_all_sessions))
        .route("/report", get(get_security_report))
        .route("/settings", get(get_settings))
        .route("/settings", put(update_settings))
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub severity: Option<Severity>,
    pub event_type: Option<SecurityEventType>,
}

#[derive(Debug, Serialize)]
pub struct EventsResponse {
    pub success: bool,
    pub data: Vec<SecurityEvent>,
}

async fn get_security_events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> AppResult<Json<EventsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(50).min(200);

    let events = service
        .get_events(page, limit, query.severity, query.event_type)
        .await?;

    Ok(Json(EventsResponse {
        success: true,
        data: events,
    }))
}

#[derive(Debug, Serialize)]
pub struct IpBlocksResponse {
    pub success: bool,
    pub data: Vec<IpBlock>,
}

async fn list_blocked_ips(State(state): State<AppState>) -> AppResult<Json<IpBlocksResponse>> {
    let service = SecurityService::new(state.db.clone());
    let blocks = service.get_blocked_ips().await?;

    Ok(Json(IpBlocksResponse {
        success: true,
        data: blocks,
    }))
}

#[derive(Debug, Serialize)]
pub struct IpBlockResponse {
    pub success: bool,
    pub data: IpBlock,
}

async fn block_ip(
    State(state): State<AppState>,
    Json(payload): Json<BlockIpRequest>,
) -> AppResult<Json<IpBlockResponse>> {
    // TODO: Get admin user from auth
    let user_id = 1;

    let service = SecurityService::new(state.db.clone());
    let block = service.block_ip(user_id, payload).await?;

    Ok(Json(IpBlockResponse {
        success: true,
        data: block,
    }))
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

async fn unblock_ip(
    State(state): State<AppState>,
    Path(ip): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    let service = SecurityService::new(state.db.clone());
    service.unblock_ip(&ip).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "IP unblocked".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiKeysResponse {
    pub success: bool,
    pub data: Vec<ApiKey>,
}

async fn list_api_keys(State(state): State<AppState>) -> AppResult<Json<ApiKeysResponse>> {
    // TODO: Get user from auth
    let user_id = 1;

    let service = SecurityService::new(state.db.clone());
    let keys = service.list_api_keys(user_id).await?;

    Ok(Json(ApiKeysResponse {
        success: true,
        data: keys,
    }))
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub success: bool,
    pub data: ApiKeyWithSecret,
}

async fn create_api_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> AppResult<Json<ApiKeyResponse>> {
    // TODO: Get user from auth
    let user_id = 1;

    let service = SecurityService::new(state.db.clone());
    let key = service.create_api_key(user_id, payload).await?;

    Ok(Json(ApiKeyResponse {
        success: true,
        data: key,
    }))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<MessageResponse>> {
    // TODO: Get user from auth
    let user_id = 1;

    let service = SecurityService::new(state.db.clone());
    service.revoke_api_key(user_id, id).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "API key revoked".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct SessionsResponse {
    pub success: bool,
    pub data: Vec<SessionInfo>,
}

async fn get_sessions(State(state): State<AppState>) -> AppResult<Json<SessionsResponse>> {
    // TODO: Get user and current session from auth
    let user_id = 1;
    let current_session_id = Some("current");

    let service = SecurityService::new(state.db.clone());
    let sessions = service
        .get_user_sessions(user_id, current_session_id)
        .await?;

    Ok(Json(SessionsResponse {
        success: true,
        data: sessions,
    }))
}

async fn revoke_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    // TODO: Get user from auth
    let user_id = 1;

    let service = SecurityService::new(state.db.clone());
    service.revoke_session(user_id, &id).await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "Session revoked".to_string(),
    }))
}

async fn revoke_all_sessions(State(state): State<AppState>) -> AppResult<Json<MessageResponse>> {
    // TODO: Get user and current session from auth
    let user_id = 1;
    let current_session_id = "current";

    let service = SecurityService::new(state.db.clone());
    let count = service
        .revoke_all_sessions(user_id, current_session_id)
        .await?;

    Ok(Json(MessageResponse {
        success: true,
        message: format!("Revoked {} session(s)", count),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub success: bool,
    pub data: SecurityReport,
}

async fn get_security_report(
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> AppResult<Json<ReportResponse>> {
    let service = SecurityService::new(state.db.clone());
    let days = query.days.unwrap_or(30).min(365);
    let report = service.get_security_report(days).await?;

    Ok(Json(ReportResponse {
        success: true,
        data: report,
    }))
}

#[derive(Debug, Serialize)]
pub struct SettingsResponse {
    pub success: bool,
    pub data: SecuritySettings,
}

async fn get_settings(State(state): State<AppState>) -> AppResult<Json<SettingsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let settings = service.get_settings().await?;

    Ok(Json(SettingsResponse {
        success: true,
        data: settings,
    }))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(payload): Json<SecuritySettings>,
) -> AppResult<Json<SettingsResponse>> {
    let service = SecurityService::new(state.db.clone());
    let settings = service.update_settings(payload).await?;

    Ok(Json(SettingsResponse {
        success: true,
        data: settings,
    }))
}
