use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{CategoryTree, CategoryWithCount, CreateCategory, UpdateCategory},
    services::CategoryService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_categories).post(create_category))
        .route("/tree", get(list_categories_tree))
        .route("/{id}", get(get_category).put(update_category).delete(delete_category))
}

/// Category list query parameters
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[allow(dead_code)]
    pub flat: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CategoryListResponse {
    pub success: bool,
    pub data: Vec<CategoryWithCount>,
}

async fn list_categories(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(_query): Query<ListQuery>,
) -> AppResult<Json<CategoryListResponse>> {
    let service = CategoryService::new(state.db.clone());
    let categories = service.list_flat(auth_user.id).await?;

    Ok(Json(CategoryListResponse {
        success: true,
        data: categories,
    }))
}

#[derive(Debug, Serialize)]
pub struct CategoryTreeResponse {
    pub success: bool,
    pub data: Vec<CategoryTree>,
}

async fn list_categories_tree(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<CategoryTreeResponse>> {
    let service = CategoryService::new(state.db.clone());
    let tree = service.list_tree(auth_user.id).await?;

    Ok(Json(CategoryTreeResponse {
        success: true,
        data: tree,
    }))
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub success: bool,
    pub data: CategoryWithCount,
}

async fn get_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<CategoryResponse>> {
    let service = CategoryService::new(state.db.clone());
    let category = service.get(id, auth_user.id).await?;

    Ok(Json(CategoryResponse {
        success: true,
        data: CategoryWithCount {
            id: category.id,
            name: category.name,
            parent_id: category.parent_id,
            user_id: category.user_id,
            document_count: 0,
        },
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "Category name must be 1-100 characters"))]
    pub name: String,
    pub parent_id: Option<i32>,
}

async fn create_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateCategoryRequest>,
) -> AppResult<(StatusCode, Json<CategoryResponse>)> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = CategoryService::new(state.db.clone());
    let category = service
        .create(
            auth_user.id,
            CreateCategory {
                name: payload.name,
                parent_id: payload.parent_id,
            },
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(CategoryResponse {
            success: true,
            data: CategoryWithCount {
                id: category.id,
                name: category.name,
                parent_id: category.parent_id,
                user_id: category.user_id,
                document_count: 0,
            },
        }),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCategoryRequest {
    #[validate(length(min = 1, max = 100, message = "Category name must be 1-100 characters"))]
    pub name: Option<String>,
    pub parent_id: Option<i32>,
}

async fn update_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCategoryRequest>,
) -> AppResult<Json<CategoryResponse>> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = CategoryService::new(state.db.clone());
    let category = service
        .update(
            id,
            auth_user.id,
            UpdateCategory {
                name: payload.name,
                parent_id: payload.parent_id,
            },
        )
        .await?;

    Ok(Json(CategoryResponse {
        success: true,
        data: CategoryWithCount {
            id: category.id,
            name: category.name,
            parent_id: category.parent_id,
            user_id: category.user_id,
            document_count: 0,
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_category(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = CategoryService::new(state.db.clone());
    service.delete(id, auth_user.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Category deleted successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ListQuery tests
    #[test]
    fn test_list_query_flat_none() {
        let query = ListQuery { flat: None };
        assert!(query.flat.is_none());
    }

    #[test]
    fn test_list_query_flat_true() {
        let query = ListQuery { flat: Some(true) };
        assert_eq!(query.flat, Some(true));
    }

    #[test]
    fn test_list_query_flat_false() {
        let query = ListQuery { flat: Some(false) };
        assert_eq!(query.flat, Some(false));
    }

    // CreateCategoryRequest validation tests
    #[test]
    fn test_create_category_request_valid() {
        let req = CreateCategoryRequest {
            name: "Documents".to_string(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_category_request_with_parent() {
        let req = CreateCategoryRequest {
            name: "Sub-folder".to_string(),
            parent_id: Some(1),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_category_request_empty_name_fails() {
        let req = CreateCategoryRequest {
            name: "".to_string(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_category_request_name_too_long_fails() {
        let req = CreateCategoryRequest {
            name: "x".repeat(101),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_category_request_max_length_ok() {
        let req = CreateCategoryRequest {
            name: "x".repeat(100),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_category_request_unicode_name() {
        let req = CreateCategoryRequest {
            name: "문서함".to_string(),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    // UpdateCategoryRequest validation tests
    #[test]
    fn test_update_category_request_all_none() {
        let req = UpdateCategoryRequest {
            name: None,
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_category_request_name_only() {
        let req = UpdateCategoryRequest {
            name: Some("New Name".to_string()),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_category_request_parent_only() {
        let req = UpdateCategoryRequest {
            name: None,
            parent_id: Some(5),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_category_request_both_fields() {
        let req = UpdateCategoryRequest {
            name: Some("Updated".to_string()),
            parent_id: Some(10),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_category_request_empty_name_fails() {
        let req = UpdateCategoryRequest {
            name: Some("".to_string()),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_category_request_name_too_long_fails() {
        let req = UpdateCategoryRequest {
            name: Some("x".repeat(101)),
            parent_id: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // CategoryListResponse tests
    #[test]
    fn test_category_list_response_creation() {
        let categories = vec![CategoryWithCount {
            id: 1,
            name: "Work".to_string(),
            parent_id: None,
            user_id: 42,
            document_count: 10,
        }];
        let response = CategoryListResponse {
            success: true,
            data: categories,
        };
        assert!(response.success);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_category_list_response_empty() {
        let response = CategoryListResponse {
            success: true,
            data: vec![],
        };
        assert!(response.data.is_empty());
    }

    // CategoryTreeResponse tests
    #[test]
    fn test_category_tree_response_creation() {
        let tree = vec![CategoryTree {
            id: 1,
            name: "Root".to_string(),
            parent_id: None,
            document_count: 5,
            children: vec![],
        }];
        let response = CategoryTreeResponse {
            success: true,
            data: tree,
        };
        assert!(response.success);
        assert_eq!(response.data[0].name, "Root");
    }

    #[test]
    fn test_category_tree_with_children() {
        let tree = vec![CategoryTree {
            id: 1,
            name: "Parent".to_string(),
            parent_id: None,
            document_count: 3,
            children: vec![CategoryTree {
                id: 2,
                name: "Child".to_string(),
                parent_id: Some(1),
                document_count: 2,
                children: vec![],
            }],
        }];
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].parent_id, Some(1));
    }

    // CategoryResponse tests
    #[test]
    fn test_category_response_creation() {
        let category = CategoryWithCount {
            id: 5,
            name: "Projects".to_string(),
            parent_id: Some(1),
            user_id: 10,
            document_count: 20,
        };
        let response = CategoryResponse {
            success: true,
            data: category,
        };
        assert!(response.success);
        assert_eq!(response.data.parent_id, Some(1));
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let response = DeleteResponse {
            success: true,
            message: "Category deleted successfully".to_string(),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted"));
    }

    // CategoryWithCount tests
    #[test]
    fn test_category_with_count_root() {
        let cat = CategoryWithCount {
            id: 1,
            name: "Root".to_string(),
            parent_id: None,
            user_id: 1,
            document_count: 0,
        };
        assert!(cat.parent_id.is_none());
    }

    #[test]
    fn test_category_with_count_child() {
        let cat = CategoryWithCount {
            id: 2,
            name: "Child".to_string(),
            parent_id: Some(1),
            user_id: 1,
            document_count: 5,
        };
        assert_eq!(cat.parent_id, Some(1));
        assert_eq!(cat.document_count, 5);
    }
}
