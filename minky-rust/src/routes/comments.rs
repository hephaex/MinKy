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
    _auth_user: AuthUser,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // CreateCommentRequest validation tests
    #[test]
    fn test_create_comment_request_valid() {
        let req = CreateCommentRequest {
            content: "This is a comment".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_comment_request_empty_content_fails() {
        let req = CreateCommentRequest {
            content: "".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_comment_request_content_too_long_fails() {
        let req = CreateCommentRequest {
            content: "x".repeat(10001),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_comment_request_max_length_ok() {
        let req = CreateCommentRequest {
            content: "x".repeat(10000),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_comment_request_single_char_ok() {
        let req = CreateCommentRequest {
            content: "a".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_comment_request_with_parent() {
        let req = CreateCommentRequest {
            content: "This is a reply".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: Some(1),
        };
        let result = req.validate();
        assert!(result.is_ok());
        assert_eq!(req.parent_id, Some(1));
    }

    #[test]
    fn test_create_comment_request_unicode_content() {
        let req = CreateCommentRequest {
            content: "한글 댓글 테스트 🎉".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    // UpdateCommentRequest validation tests
    #[test]
    fn test_update_comment_request_valid() {
        let req = UpdateCommentRequest {
            content: "Updated content".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_comment_request_empty_content_fails() {
        let req = UpdateCommentRequest {
            content: "".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_comment_request_content_too_long_fails() {
        let req = UpdateCommentRequest {
            content: "x".repeat(10001),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // CommentListResponse tests
    #[test]
    fn test_comment_list_response_creation() {
        let comments = vec![CommentWithAuthor {
            id: 1,
            content: "Test comment".to_string(),
            document_id: Uuid::new_v4(),
            user_id: 42,
            author_name: "Alice".to_string(),
            parent_id: None,
            replies: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let response = CommentListResponse {
            success: true,
            data: comments,
            count: 1,
        };
        assert!(response.success);
        assert_eq!(response.count, 1);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_comment_list_response_empty() {
        let response = CommentListResponse {
            success: true,
            data: vec![],
            count: 0,
        };
        assert!(response.data.is_empty());
        assert_eq!(response.count, 0);
    }

    // CommentResponse tests
    #[test]
    fn test_comment_response_creation() {
        let doc_id = Uuid::new_v4();
        let data = CommentData {
            id: 1,
            content: "Great article!".to_string(),
            document_id: doc_id,
            user_id: 10,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = CommentResponse {
            success: true,
            data,
        };
        assert!(response.success);
        assert_eq!(response.data.id, 1);
        assert_eq!(response.data.document_id, doc_id);
    }

    // CommentData tests
    #[test]
    fn test_comment_data_root_comment() {
        let data = CommentData {
            id: 1,
            content: "Root comment".to_string(),
            document_id: Uuid::new_v4(),
            user_id: 5,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(data.parent_id.is_none());
    }

    #[test]
    fn test_comment_data_reply_comment() {
        let data = CommentData {
            id: 2,
            content: "This is a reply".to_string(),
            document_id: Uuid::new_v4(),
            user_id: 6,
            parent_id: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(data.parent_id, Some(1));
    }

    // CommentWithAuthor tests
    #[test]
    fn test_comment_with_author_creation() {
        let comment = CommentWithAuthor {
            id: 1,
            content: "Hello world".to_string(),
            document_id: Uuid::new_v4(),
            user_id: 100,
            author_name: "Bob".to_string(),
            parent_id: None,
            replies: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(comment.author_name, "Bob");
        assert!(comment.replies.is_empty());
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let response = DeleteResponse {
            success: true,
            message: "Comment deleted successfully".to_string(),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted"));
    }

    #[test]
    fn test_create_comment_request_multiline_content() {
        let req = CreateCommentRequest {
            content: "Line 1\nLine 2\nLine 3".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_comment_request_with_markdown() {
        let req = CreateCommentRequest {
            content: "**Bold** and *italic* with `code`".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }
}
