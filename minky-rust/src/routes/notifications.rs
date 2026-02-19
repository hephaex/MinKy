use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::Notification,
    services::NotificationService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/count", get(get_unread_count))
        .route("/{id}/read", put(mark_as_read))
        .route("/read-all", post(mark_all_as_read))
        .route("/{id}", delete(delete_notification))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub include_read: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub success: bool,
    pub data: Vec<Notification>,
    pub unread_count: i64,
}

async fn list_notifications(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<NotificationListResponse>> {
    let service = NotificationService::new(state.db.clone());
    let notifications = service
        .list(
            auth_user.id,
            query.include_read.unwrap_or(true),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await?;
    let unread_count = service.get_unread_count(auth_user.id).await?;

    Ok(Json(NotificationListResponse {
        success: true,
        data: notifications,
        unread_count,
    }))
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub success: bool,
    pub count: i64,
}

async fn get_unread_count(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<UnreadCountResponse>> {
    let service = NotificationService::new(state.db.clone());
    let count = service.get_unread_count(auth_user.id).await?;

    Ok(Json(UnreadCountResponse {
        success: true,
        count,
    }))
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub success: bool,
    pub data: Notification,
}

async fn mark_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<NotificationResponse>> {
    let service = NotificationService::new(state.db.clone());
    let notification = service.mark_as_read(id, auth_user.id).await?;

    Ok(Json(NotificationResponse {
        success: true,
        data: notification,
    }))
}

#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    pub success: bool,
    pub count: i64,
}

async fn mark_all_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<MarkAllReadResponse>> {
    let service = NotificationService::new(state.db.clone());
    let count = service.mark_all_as_read(auth_user.id).await?;

    Ok(Json(MarkAllReadResponse {
        success: true,
        count,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_notification(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = NotificationService::new(state.db.clone());
    service.delete(id, auth_user.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Notification deleted successfully".to_string(),
    }))
}
