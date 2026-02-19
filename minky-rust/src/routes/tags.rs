use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{CreateTag, TagWithCount, UpdateTag},
    services::TagService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags).post(create_tag))
        .route("/{id}", get(get_tag).put(update_tag).delete(delete_tag))
}

#[derive(Debug, Serialize)]
pub struct TagListResponse {
    pub success: bool,
    pub data: Vec<TagWithCount>,
}

async fn list_tags(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<TagListResponse>> {
    let service = TagService::new(state.db.clone());
    let tags = service.list(auth_user.id).await?;

    Ok(Json(TagListResponse {
        success: true,
        data: tags,
    }))
}

#[derive(Debug, Serialize)]
pub struct TagResponse {
    pub success: bool,
    pub data: TagWithCount,
}

async fn get_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<TagResponse>> {
    let service = TagService::new(state.db.clone());
    let tag = service.get(id, auth_user.id).await?;

    Ok(Json(TagResponse {
        success: true,
        data: TagWithCount {
            id: tag.id,
            name: tag.name,
            user_id: tag.user_id,
            document_count: 0,
            created_at: tag.created_at,
        },
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagRequest {
    #[validate(length(min = 1, max = 100, message = "Tag name must be 1-100 characters"))]
    pub name: String,
}

async fn create_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateTagRequest>,
) -> AppResult<(StatusCode, Json<TagResponse>)> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = TagService::new(state.db.clone());
    let tag = service
        .create(auth_user.id, CreateTag { name: payload.name })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(TagResponse {
            success: true,
            data: TagWithCount {
                id: tag.id,
                name: tag.name,
                user_id: tag.user_id,
                document_count: 0,
                created_at: tag.created_at,
            },
        }),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTagRequest {
    #[validate(length(min = 1, max = 100, message = "Tag name must be 1-100 characters"))]
    pub name: Option<String>,
}

async fn update_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTagRequest>,
) -> AppResult<Json<TagResponse>> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = TagService::new(state.db.clone());
    let tag = service
        .update(id, auth_user.id, UpdateTag { name: payload.name })
        .await?;

    Ok(Json(TagResponse {
        success: true,
        data: TagWithCount {
            id: tag.id,
            name: tag.name,
            user_id: tag.user_id,
            document_count: 0,
            created_at: tag.created_at,
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_tag(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = TagService::new(state.db.clone());
    service.delete(id, auth_user.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Tag deleted successfully".to_string(),
    }))
}
