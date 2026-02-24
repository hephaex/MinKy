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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // CreateTagRequest validation tests
    #[test]
    fn test_create_tag_request_valid() {
        let req = CreateTagRequest {
            name: "rust".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag_request_empty_name_fails() {
        let req = CreateTagRequest {
            name: "".to_string(),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tag_request_name_too_long_fails() {
        let req = CreateTagRequest {
            name: "x".repeat(101),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tag_request_max_length_ok() {
        let req = CreateTagRequest {
            name: "x".repeat(100),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag_request_single_char_ok() {
        let req = CreateTagRequest {
            name: "a".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag_request_unicode_name() {
        let req = CreateTagRequest {
            name: "한국어태그".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    // UpdateTagRequest validation tests
    #[test]
    fn test_update_tag_request_none_ok() {
        let req = UpdateTagRequest { name: None };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_tag_request_valid_name() {
        let req = UpdateTagRequest {
            name: Some("new-tag".to_string()),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_tag_request_empty_name_fails() {
        let req = UpdateTagRequest {
            name: Some("".to_string()),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_tag_request_name_too_long_fails() {
        let req = UpdateTagRequest {
            name: Some("x".repeat(101)),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // TagListResponse tests
    #[test]
    fn test_tag_list_response_creation() {
        let tags = vec![TagWithCount {
            id: 1,
            name: "rust".to_string(),
            user_id: 42,
            document_count: 5,
            created_at: Utc::now(),
        }];
        let response = TagListResponse {
            success: true,
            data: tags,
        };
        assert!(response.success);
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].name, "rust");
    }

    #[test]
    fn test_tag_list_response_empty() {
        let response = TagListResponse {
            success: true,
            data: vec![],
        };
        assert!(response.data.is_empty());
    }

    // TagResponse tests
    #[test]
    fn test_tag_response_creation() {
        let tag = TagWithCount {
            id: 1,
            name: "web".to_string(),
            user_id: 10,
            document_count: 3,
            created_at: Utc::now(),
        };
        let response = TagResponse {
            success: true,
            data: tag,
        };
        assert!(response.success);
        assert_eq!(response.data.id, 1);
        assert_eq!(response.data.document_count, 3);
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let response = DeleteResponse {
            success: true,
            message: "Tag deleted successfully".to_string(),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted"));
    }

    // TagWithCount tests
    #[test]
    fn test_tag_with_count_zero_documents() {
        let tag = TagWithCount {
            id: 5,
            name: "empty-tag".to_string(),
            user_id: 1,
            document_count: 0,
            created_at: Utc::now(),
        };
        assert_eq!(tag.document_count, 0);
    }

    #[test]
    fn test_tag_with_count_many_documents() {
        let tag = TagWithCount {
            id: 10,
            name: "popular".to_string(),
            user_id: 2,
            document_count: 1000,
            created_at: Utc::now(),
        };
        assert_eq!(tag.document_count, 1000);
    }

    #[test]
    fn test_create_tag_request_with_spaces() {
        let req = CreateTagRequest {
            name: "tag with spaces".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag_request_with_hyphen() {
        let req = CreateTagRequest {
            name: "my-custom-tag".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_tag_request_with_underscore() {
        let req = CreateTagRequest {
            name: "my_custom_tag".to_string(),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }
}
