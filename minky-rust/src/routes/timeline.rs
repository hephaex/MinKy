use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
use crate::models::{
    ActivityHeatmap, DailyActivity, DocumentHistory, TimelineEventType, TimelineQuery,
    TimelineResponse, TimelineStats, UserActivityStream,
};
use crate::services::TimelineService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct DailyActivityQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UserActivityQuery {
    pub limit: Option<i32>,
}

/// Get timeline events
async fn get_timeline(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_timeline(query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Log timeline event
async fn log_event(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(request): Json<LogEventRequest>,
) -> Result<Json<LogEventResponse>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .log_event(
            request.event_type,
            request.user_id,
            request.document_id,
            &request.description,
            request.metadata,
        )
        .await
        .map(|event_id| Json(LogEventResponse { event_id }))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get daily activity
async fn get_daily_activity(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<DailyActivityQuery>,
) -> Result<Json<Vec<DailyActivity>>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let days = query.days.unwrap_or(30);

    service
        .get_daily_activity(days)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get activity heatmap
async fn get_heatmap(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<HeatmapQuery>,
) -> Result<Json<ActivityHeatmap>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let year = query.year.unwrap_or(chrono::Utc::now().year());

    service
        .get_activity_heatmap(year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get user activity stream
async fn get_user_activity(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(user_id): Path<i32>,
    Query(query): Query<UserActivityQuery>,
) -> Result<Json<UserActivityStream>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());
    let limit = query.limit.unwrap_or(50);

    service
        .get_user_activity(user_id, limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get document history
async fn get_document_history(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentHistory>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_document_history(document_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get timeline statistics
async fn get_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<TimelineStats>, (StatusCode, String)> {
    let service = TimelineService::new(state.db.clone());

    service
        .get_stats()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct LogEventRequest {
    pub event_type: TimelineEventType,
    pub user_id: i32,
    pub document_id: Option<uuid::Uuid>,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct LogEventResponse {
    pub event_id: String,
}

use chrono::Datelike;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_timeline).post(log_event))
        .route("/daily", get(get_daily_activity))
        .route("/heatmap", get(get_heatmap))
        .route("/users/{user_id}", get(get_user_activity))
        .route("/documents/{document_id}", get(get_document_history))
        .route("/stats", get(get_stats))
}
