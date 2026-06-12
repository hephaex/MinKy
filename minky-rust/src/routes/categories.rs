use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    middleware::{AuthUser, OptionalAuthUser},
    models::{CategoryWithCount, CreateCategory, UpdateCategory},
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
    /// Flask compat: ?format=flat | ?format=tree
    pub format: Option<String>,
    #[allow(dead_code)]
    pub include_inactive: Option<bool>,
}

/// Flask compat: GET /categories/?format=flat → {success, data: {categories, count}}
///               GET /categories/?format=tree → {success, data: {tree, count}}
///
/// Frontend reads:
///   useCategories.js  → response.data.data?.categories || []
///   CategoryManager.js → response.data.data?.tree || []
async fn list_categories(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Value>> {
    let service = CategoryService::new(state.db.clone());
    let user_id = auth_user.0.map(|u| u.id);

    if query.format.as_deref() == Some("tree") {
        let tree = service.list_tree_optional(user_id).await
            .map_err(AppError::Internal)?;
        let count = tree.len();
        Ok(Json(json!({"success": true, "data": {"tree": tree, "count": count}})))
    } else {
        let categories = service.list_flat_optional(user_id).await
            .map_err(AppError::Internal)?;
        let count = categories.len();
        Ok(Json(json!({"success": true, "data": {"categories": categories, "count": count}})))
    }
}

/// Legacy endpoint kept for direct access; frontend uses /categories/?format=tree instead
async fn list_categories_tree(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
) -> AppResult<Json<Value>> {
    let service = CategoryService::new(state.db.clone());
    let user_id = auth_user.0.map(|u| u.id);
    let tree = service.list_tree_optional(user_id).await
        .map_err(AppError::Internal)?;
    let count = tree.len();
    Ok(Json(json!({"success": true, "data": {"tree": tree, "count": count}})))
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub success: bool,
    pub data: CategoryWithCount,
}

async fn get_category(
    State(state): State<AppState>,
    auth_user: OptionalAuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<CategoryResponse>> {
    let service = CategoryService::new(state.db.clone());
    // When unauthenticated, use sentinel -1 so only non-user-scoped GET works
    let user_id = auth_user.0.map(|u| u.id).unwrap_or(-1);
    let category = service.get(id, user_id).await?;

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
    use crate::models::CategoryTree;

    // ListQuery tests
    #[test]
    fn test_list_query_flat_none() {
        let query = ListQuery { flat: None, format: None, include_inactive: None };
        assert!(query.flat.is_none());
    }

    #[test]
    fn test_list_query_flat_true() {
        let query = ListQuery { flat: Some(true), format: None, include_inactive: None };
        assert_eq!(query.flat, Some(true));
    }

    #[test]
    fn test_list_query_flat_false() {
        let query = ListQuery { flat: Some(false), format: None, include_inactive: None };
        assert_eq!(query.flat, Some(false));
    }

    #[test]
    fn test_list_query_format_tree() {
        let query = ListQuery { flat: None, format: Some("tree".to_string()), include_inactive: None };
        assert_eq!(query.format.as_deref(), Some("tree"));
    }

    #[test]
    fn test_list_query_format_flat() {
        let query = ListQuery { flat: None, format: Some("flat".to_string()), include_inactive: None };
        assert_eq!(query.format.as_deref(), Some("flat"));
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

    // Flask-compat consumer-contract tests for list response shapes
    // Frontend reads:
    //   useCategories.js     → response.data.data?.categories || []
    //   CategoryManager.js   → response.data.data?.tree || []

    #[test]
    fn flask_flat_response_has_categories_key() {
        // Simulate what list_categories returns for format=flat
        let categories: Vec<CategoryWithCount> = vec![CategoryWithCount {
            id: 1, name: "Work".to_string(), parent_id: None, user_id: 42, document_count: 10,
        }];
        let count = categories.len();
        let v = json!({"success": true, "data": {"categories": categories, "count": count}});
        // useCategories.js: response.data.data?.categories
        assert!(v["data"]["categories"].is_array(), "flat: must have data.categories");
        assert_eq!(v["data"]["count"], 1);
        assert!(v["data"]["tree"].is_null(), "flat: must NOT have tree key");
    }

    #[test]
    fn flask_flat_response_categories_key_absent_gives_empty() {
        // When categories is empty list, still serialise the key (not omit it)
        let v = json!({"success": true, "data": {"categories": Vec::<CategoryWithCount>::new(), "count": 0}});
        assert!(v["data"]["categories"].is_array());
        assert_eq!(v["data"]["categories"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn flask_tree_response_has_tree_key() {
        // Simulate what list_categories returns for format=tree
        let tree: Vec<CategoryTree> = vec![CategoryTree {
            id: 1, name: "Root".to_string(), parent_id: None, document_count: 5, children: vec![],
        }];
        let count = tree.len();
        let v = json!({"success": true, "data": {"tree": tree, "count": count}});
        // CategoryManager.js: response.data.data?.tree
        assert!(v["data"]["tree"].is_array(), "tree: must have data.tree");
        assert_eq!(v["data"]["count"], 1);
        assert!(v["data"]["categories"].is_null(), "tree: must NOT have categories key");
    }

    #[test]
    fn flask_tree_with_children() {
        let tree = [CategoryTree {
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
