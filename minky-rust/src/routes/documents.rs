use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{error::{AppError, AppResult}, middleware::AuthUser, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/{id}", get(get_document).put(update_document).delete(delete_document))
}

/// Query parameters for document listing
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub category_id: Option<i32>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub is_public: bool,
    pub view_count: i32,
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
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse<DocumentResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;
    let user_id = auth_user.id;

    // Only return user's own documents or public documents
    let total: (i64,) = if let Some(ref search) = query.search {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE (user_id = $1 OR is_public = true) AND title ILIKE '%' || $2 || '%'"
        )
        .bind(user_id)
        .bind(search)
        .fetch_one(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE (user_id = $1 OR is_public = true) AND category_id = $2"
        )
        .bind(user_id)
        .bind(cat_id)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM documents WHERE user_id = $1 OR is_public = true")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?
    };

    let documents: Vec<DocumentResponse> = if let Some(ref search) = query.search {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE (user_id = $1 OR is_public = true) AND title ILIKE '%' || $2 || '%'
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#
        )
        .bind(user_id)
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE (user_id = $1 OR is_public = true) AND category_id = $2
               ORDER BY created_at DESC
               LIMIT $3 OFFSET $4"#
        )
        .bind(user_id)
        .bind(cat_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE user_id = $1 OR is_public = true
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    };

    let total_pages = ((total.0 as f64) / (limit as f64)).ceil() as i32;

    Ok(Json(ListResponse {
        success: true,
        data: documents,
        meta: PaginationMeta {
            total: total.0,
            page,
            limit,
            total_pages,
        },
    }))
}

/// Create document request
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SingleResponse<T> {
    pub success: bool,
    pub data: T,
}

async fn create_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = auth_user.id;
    let is_public = payload.is_public.unwrap_or(false);

    let doc: DocumentResponse = sqlx::query_as(
        r#"INSERT INTO documents (title, content, category_id, user_id, is_public)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at"#
    )
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(payload.category_id)
    .bind(user_id)
    .bind(is_public)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(SingleResponse {
        success: true,
        data: doc,
    }))
}

async fn get_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    let user_id = auth_user.id;

    // Only allow access to own documents or public documents
    let doc: Option<DocumentResponse> = sqlx::query_as(
        r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
           FROM documents
           WHERE id = $1 AND (user_id = $2 OR is_public = true)"#
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    let doc = doc.ok_or_else(|| AppError::NotFound(format!("Document {} not found or access denied", id)))?;

    // Increment view count
    sqlx::query("UPDATE documents SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(SingleResponse {
        success: true,
        data: doc,
    }))
}

/// Update document request
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDocumentRequest {
    #[validate(length(min = 1, max = 500, message = "Title must be 1-500 characters"))]
    pub title: Option<String>,
    pub content: Option<String>,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

async fn update_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user_id = auth_user.id;

    // Check document exists AND user owns it
    let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM documents WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .fetch_optional(&state.db)
        .await?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("Document {} not found or access denied", id)));
    }

    // Build update query dynamically based on provided fields (only owner can update)
    let doc: DocumentResponse = sqlx::query_as(
        r#"UPDATE documents
           SET
               title = COALESCE($1, title),
               content = COALESCE($2, content),
               category_id = CASE WHEN $3::boolean THEN $4::integer ELSE category_id END,
               is_public = COALESCE($5, is_public),
               updated_at = NOW()
           WHERE id = $6 AND user_id = $7
           RETURNING id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at"#
    )
    .bind(payload.title.as_deref())
    .bind(payload.content.as_deref())
    .bind(payload.category_id.is_some())
    .bind(payload.category_id)
    .bind(payload.is_public)
    .bind(id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(SingleResponse {
        success: true,
        data: doc,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    let user_id = auth_user.id;

    // Only allow owner to delete their document
    let result = sqlx::query("DELETE FROM documents WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Document {} not found or access denied", id)));
    }

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Document {} deleted successfully", id),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ListQuery tests
    #[test]
    fn test_list_query_default_values() {
        let query = ListQuery {
            page: None,
            limit: None,
            category_id: None,
            search: None,
        };
        assert!(query.page.is_none());
        assert!(query.limit.is_none());
        assert!(query.category_id.is_none());
        assert!(query.search.is_none());
    }

    #[test]
    fn test_list_query_with_values() {
        let query = ListQuery {
            page: Some(2),
            limit: Some(50),
            category_id: Some(5),
            search: Some("test".to_string()),
        };
        assert_eq!(query.page, Some(2));
        assert_eq!(query.limit, Some(50));
        assert_eq!(query.category_id, Some(5));
        assert_eq!(query.search, Some("test".to_string()));
    }

    // Pagination calculation tests
    #[test]
    fn test_page_min_value_clamped_to_1() {
        let query = ListQuery {
            page: Some(0),
            limit: None,
            category_id: None,
            search: None,
        };
        let page = query.page.unwrap_or(1).max(1);
        assert_eq!(page, 1);
    }

    #[test]
    fn test_page_negative_clamped_to_1() {
        let query = ListQuery {
            page: Some(-5),
            limit: None,
            category_id: None,
            search: None,
        };
        let page = query.page.unwrap_or(1).max(1);
        assert_eq!(page, 1);
    }

    #[test]
    fn test_limit_default_is_20() {
        let query = ListQuery {
            page: None,
            limit: None,
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 20);
    }

    #[test]
    fn test_limit_max_clamped_to_100() {
        let query = ListQuery {
            page: None,
            limit: Some(500),
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 100);
    }

    #[test]
    fn test_limit_min_clamped_to_1() {
        let query = ListQuery {
            page: None,
            limit: Some(0),
            category_id: None,
            search: None,
        };
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        assert_eq!(limit, 1);
    }

    #[test]
    fn test_offset_calculation() {
        let page = 3;
        let limit = 20;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 40);
    }

    #[test]
    fn test_offset_first_page() {
        let page = 1;
        let limit = 20;
        let offset = (page - 1) * limit;
        assert_eq!(offset, 0);
    }

    // DocumentResponse tests
    #[test]
    fn test_document_response_creation() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Test Doc".to_string(),
            content: "Content here".to_string(),
            category_id: Some(1),
            user_id: 42,
            is_public: true,
            view_count: 10,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(doc.title, "Test Doc");
        assert_eq!(doc.user_id, 42);
        assert!(doc.is_public);
    }

    #[test]
    fn test_document_response_private() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Private Doc".to_string(),
            content: "Secret".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(!doc.is_public);
        assert!(doc.category_id.is_none());
    }

    // PaginationMeta tests
    #[test]
    fn test_pagination_meta_creation() {
        let meta = PaginationMeta {
            total: 100,
            page: 1,
            limit: 20,
            total_pages: 5,
        };
        assert_eq!(meta.total, 100);
        assert_eq!(meta.total_pages, 5);
    }

    #[test]
    fn test_total_pages_calculation() {
        let total: i64 = 95;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 5);
    }

    #[test]
    fn test_total_pages_exact_division() {
        let total: i64 = 100;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 5);
    }

    #[test]
    fn test_total_pages_single_page() {
        let total: i64 = 15;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 1);
    }

    #[test]
    fn test_total_pages_empty() {
        let total: i64 = 0;
        let limit = 20;
        let total_pages = ((total as f64) / (limit as f64)).ceil() as i32;
        assert_eq!(total_pages, 0);
    }

    // CreateDocumentRequest validation tests
    #[test]
    fn test_create_document_request_valid() {
        let req = CreateDocumentRequest {
            title: "Valid Title".to_string(),
            content: "Some content".to_string(),
            category_id: None,
            is_public: Some(true),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_request_empty_title_fails() {
        let req = CreateDocumentRequest {
            title: "".to_string(),
            content: "Content".to_string(),
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_document_request_title_too_long_fails() {
        let req = CreateDocumentRequest {
            title: "x".repeat(501),
            content: "Content".to_string(),
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_document_request_max_title_length_ok() {
        let req = CreateDocumentRequest {
            title: "x".repeat(500),
            content: "Content".to_string(),
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_document_is_public_default_false() {
        let req = CreateDocumentRequest {
            title: "Test".to_string(),
            content: "Content".to_string(),
            category_id: None,
            is_public: None,
        };
        let is_public = req.is_public.unwrap_or(false);
        assert!(!is_public);
    }

    // UpdateDocumentRequest validation tests
    #[test]
    fn test_update_document_request_all_none() {
        let req = UpdateDocumentRequest {
            title: None,
            content: None,
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_document_request_partial() {
        let req = UpdateDocumentRequest {
            title: Some("New Title".to_string()),
            content: None,
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_document_request_empty_title_fails() {
        let req = UpdateDocumentRequest {
            title: Some("".to_string()),
            content: None,
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_document_request_title_too_long_fails() {
        let req = UpdateDocumentRequest {
            title: Some("x".repeat(501)),
            content: None,
            category_id: None,
            is_public: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // ListResponse tests
    #[test]
    fn test_list_response_creation() {
        let docs = vec![DocumentResponse {
            id: Uuid::new_v4(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        let response = ListResponse {
            success: true,
            data: docs,
            meta: PaginationMeta {
                total: 1,
                page: 1,
                limit: 20,
                total_pages: 1,
            },
        };
        assert!(response.success);
        assert_eq!(response.data.len(), 1);
    }

    #[test]
    fn test_list_response_empty() {
        let response: ListResponse<DocumentResponse> = ListResponse {
            success: true,
            data: vec![],
            meta: PaginationMeta {
                total: 0,
                page: 1,
                limit: 20,
                total_pages: 0,
            },
        };
        assert!(response.data.is_empty());
        assert_eq!(response.meta.total, 0);
    }

    // SingleResponse tests
    #[test]
    fn test_single_response_creation() {
        let doc = DocumentResponse {
            id: Uuid::new_v4(),
            title: "Single".to_string(),
            content: "Content".to_string(),
            category_id: Some(5),
            user_id: 10,
            is_public: true,
            view_count: 100,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = SingleResponse {
            success: true,
            data: doc,
        };
        assert!(response.success);
        assert_eq!(response.data.title, "Single");
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let id = Uuid::new_v4();
        let response = DeleteResponse {
            success: true,
            message: format!("Document {} deleted successfully", id),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted successfully"));
    }

    #[test]
    fn test_delete_response_contains_uuid() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let response = DeleteResponse {
            success: true,
            message: format!("Document {} deleted successfully", id),
        };
        assert!(response.message.contains("550e8400-e29b-41d4-a716-446655440000"));
    }
}
