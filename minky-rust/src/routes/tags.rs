use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::AppResult,
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

async fn list_tags(State(state): State<AppState>) -> AppResult<Json<TagListResponse>> {
    // TODO: Extract user_id from JWT
    let user_id = 1;

    let service = TagService::new(state.db.clone());
    let tags = service.list(user_id).await?;

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
    Path(id): Path<i32>,
) -> AppResult<Json<TagResponse>> {
    let user_id = 1;

    let service = TagService::new(state.db.clone());
    let tag = service.get(id, user_id).await?;

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
    Json(payload): Json<CreateTagRequest>,
) -> AppResult<Json<TagResponse>> {
    let user_id = 1;

    let service = TagService::new(state.db.clone());
    let tag = service
        .create(user_id, CreateTag { name: payload.name })
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

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTagRequest {
    #[validate(length(min = 1, max = 100, message = "Tag name must be 1-100 characters"))]
    pub name: Option<String>,
}

async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTagRequest>,
) -> AppResult<Json<TagResponse>> {
    let user_id = 1;

    let service = TagService::new(state.db.clone());
    let tag = service
        .update(id, user_id, UpdateTag { name: payload.name })
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
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let user_id = 1;

    let service = TagService::new(state.db.clone());
    service.delete(id, user_id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Tag deleted successfully".to_string(),
    }))
}
