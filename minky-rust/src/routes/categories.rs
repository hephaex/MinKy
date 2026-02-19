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
