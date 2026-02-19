use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{CommentWithAuthor, CreateComment, UpdateComment},
    services::CommentService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/document/{document_id}", get(list_comments))
        .route("/", post(create_comment))
        .route("/{id}", put(update_comment).delete(delete_comment))
}

#[derive(Debug, Serialize)]
pub struct CommentListResponse {
    pub success: bool,
    pub data: Vec<CommentWithAuthor>,
    pub count: i64,
}

async fn list_comments(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<CommentListResponse>> {
    let service = CommentService::new(state.db.clone());
    let comments = service.list_for_document(document_id).await?;
    let count = service.count_for_document(document_id).await?;

    Ok(Json(CommentListResponse {
        success: true,
        data: comments,
        count,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, max = 10000, message = "Comment must be 1-10000 characters"))]
    pub content: String,
    pub document_id: Uuid,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub success: bool,
    pub data: CommentData,
}

#[derive(Debug, Serialize)]
pub struct CommentData {
    pub id: i32,
    pub content: String,
    pub document_id: Uuid,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

async fn create_comment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateCommentRequest>,
) -> AppResult<(StatusCode, Json<CommentResponse>)> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = CommentService::new(state.db.clone());
    let comment = service
        .create(
            auth_user.id,
            CreateComment {
                content: payload.content,
                document_id: payload.document_id,
                parent_id: payload.parent_id,
            },
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(CommentResponse {
            success: true,
            data: CommentData {
                id: comment.id,
                content: comment.content,
                document_id: comment.document_id,
                user_id: comment.user_id,
                parent_id: comment.parent_id,
                created_at: comment.created_at,
                updated_at: comment.updated_at,
            },
        }),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCommentRequest {
    #[validate(length(min = 1, max = 10000, message = "Comment must be 1-10000 characters"))]
    pub content: String,
}

async fn update_comment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCommentRequest>,
) -> AppResult<Json<CommentResponse>> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = CommentService::new(state.db.clone());
    let comment = service
        .update(id, auth_user.id, UpdateComment { content: payload.content })
        .await?;

    Ok(Json(CommentResponse {
        success: true,
        data: CommentData {
            id: comment.id,
            content: comment.content,
            document_id: comment.document_id,
            user_id: comment.user_id,
            parent_id: comment.parent_id,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_comment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = CommentService::new(state.db.clone());
    service.delete(id, auth_user.id, auth_user.is_admin()).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Comment deleted successfully".to_string(),
    }))
}
