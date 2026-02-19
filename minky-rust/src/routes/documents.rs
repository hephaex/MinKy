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
    Query(query): Query<ListQuery>,
) -> AppResult<Json<ListResponse<DocumentResponse>>> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let total: (i64,) = if let Some(ref search) = query.search {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE title ILIKE '%' || $1 || '%'"
        )
        .bind(search)
        .fetch_one(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE category_id = $1"
        )
        .bind(cat_id)
        .fetch_one(&state.db)
        .await?
    } else {
        sqlx::query_as("SELECT COUNT(*) FROM documents")
            .fetch_one(&state.db)
            .await?
    };

    let documents: Vec<DocumentResponse> = if let Some(ref search) = query.search {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE title ILIKE '%' || $1 || '%'
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else if let Some(cat_id) = query.category_id {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               WHERE category_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#
        )
        .bind(cat_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as(
            r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
               FROM documents
               ORDER BY created_at DESC
               LIMIT $1 OFFSET $2"#
        )
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
    Path(id): Path<Uuid>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    let doc: Option<DocumentResponse> = sqlx::query_as(
        r#"SELECT id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at
           FROM documents
           WHERE id = $1"#
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let doc = doc.ok_or_else(|| AppError::NotFound(format!("Document {} not found", id)))?;

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
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDocumentRequest>,
) -> AppResult<Json<SingleResponse<DocumentResponse>>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    // Check document exists
    let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM documents WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;

    if existing.is_none() {
        return Err(AppError::NotFound(format!("Document {} not found", id)));
    }

    // Build update query dynamically based on provided fields
    let doc: DocumentResponse = sqlx::query_as(
        r#"UPDATE documents
           SET
               title = COALESCE($1, title),
               content = COALESCE($2, content),
               category_id = CASE WHEN $3::boolean THEN $4::integer ELSE category_id END,
               is_public = COALESCE($5, is_public),
               updated_at = NOW()
           WHERE id = $6
           RETURNING id, title, content, category_id, user_id, is_public, view_count, created_at, updated_at"#
    )
    .bind(payload.title.as_deref())
    .bind(payload.content.as_deref())
    .bind(payload.category_id.is_some())
    .bind(payload.category_id)
    .bind(payload.is_public)
    .bind(id)
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
    Path(id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    let result = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("Document {} not found", id)));
    }

    Ok(Json(DeleteResponse {
        success: true,
        message: format!("Document {} deleted successfully", id),
    }))
}
