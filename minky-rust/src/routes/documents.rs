use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{error::AppResult, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/{id}", get(get_document).put(update_document).delete(delete_document))
}

/// Query parameters for document listing (fields used when DB stub is replaced)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub page: i32,
    pub limit: i32,
    pub total_pages: i32,
}

async fn list_documents(
    State(_state): State<AppState>,
    Query(_query): Query<ListQuery>,
) -> AppResult<Json<ListResponse<DocumentResponse>>> {
    // TODO: Implement document listing
    Ok(Json(ListResponse {
        success: true,
        data: vec![],
        meta: PaginationMeta {
            total: 0,
            page: 1,
            limit: 20,
            total_pages: 0,
        },
    }))
}

/// Create document request (content/category_id used when DB stub is replaced)
#[allow(dead_code)]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SingleResponse<T> {
    pub success: bool,
    pub data: T,
}

async fn create_document(
    State(_state): State<AppState>,
    Json(_payload): Json<CreateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    // TODO: Implement document creation
    Ok(Json(SingleResponse {
        success: true,
        data: DocumentResponse {
            id: Uuid::new_v4(),
            title: "New Document".to_string(),
            content: "".to_string(),
            category_id: None,
            user_id: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    }))
}

async fn get_document(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    // TODO: Implement document retrieval
    Ok(Json(SingleResponse {
        success: true,
        data: DocumentResponse {
            id: Uuid::new_v4(),
            title: "Document".to_string(),
            content: "".to_string(),
            category_id: None,
            user_id: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    }))
}

/// Update document request (content/category_id used when DB stub is replaced)
#[allow(dead_code)]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: Option<String>,
    pub content: Option<String>,
    pub category_id: Option<i32>,
}

async fn update_document(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    // TODO: Implement document update
    Ok(Json(SingleResponse {
        success: true,
        data: DocumentResponse {
            id: Uuid::new_v4(),
            title: "Updated Document".to_string(),
            content: "".to_string(),
            category_id: None,
            user_id: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_document(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    // TODO: Implement document deletion
    Ok(Json(DeleteResponse {
        success: true,
        message: "Document deleted successfully".to_string(),
    }))
}
