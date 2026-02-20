//! Embedding API routes
//!
//! Provides endpoints for:
//! - Generating document and chunk embeddings
//! - Semantic search via vector similarity
//! - Finding similar documents
//! - Embedding statistics
//! - Queue management

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use secrecy::ExposeSecret;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::AuthUser,
    models::{
        ChunkEmbedding, CreateChunkEmbeddingsRequest, CreateDocumentEmbeddingRequest,
        DocumentEmbedding, EmbeddingModel, EmbeddingQueueEntry, EmbeddingStats,
        SemanticSearchRequest, SemanticSearchResult,
    },
    services::{EmbeddingConfig, EmbeddingService},
    AppState,
};

use super::common::{into_error_response, ApiResponse};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn build_service(state: &AppState) -> EmbeddingService {
    let openai_api_key = state
        .config
        .openai_api_key
        .as_ref()
        .map(|k| k.expose_secret().to_owned());

    let config = EmbeddingConfig {
        openai_api_key,
        voyage_api_key: None,
        default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
        chunk_size: 512,
        chunk_overlap: 50,
    };

    EmbeddingService::new(state.db.clone(), config)
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Query parameters for the similar-documents endpoint
#[derive(Debug, Deserialize)]
pub struct SimilarQuery {
    /// Maximum number of results to return (default: 10, max: 50)
    pub limit: Option<i32>,
}

/// Request body for adding a document to the embedding queue
#[derive(Debug, Deserialize)]
pub struct QueueRequest {
    /// Processing priority – higher values are processed first (default: 0)
    #[serde(default)]
    pub priority: i32,
}


// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/embeddings/document/:id
///
/// Generate (or regenerate) the document-level embedding for the given document.
/// Requires authentication.
async fn create_document_embedding(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DocumentEmbedding>>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| into_error_response(AppError::Database(e)))?;

    if !has_access {
        return Err(into_error_response(AppError::Forbidden));
    }

    let service = build_service(&state);

    let req = CreateDocumentEmbeddingRequest {
        document_id,
        model: None,
    };

    service
        .create_document_embedding(req)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// POST /api/embeddings/chunks/:id
///
/// Generate chunk-level embeddings for the given document.
/// The request body supplies the chunks to embed. Requires authentication.
async fn create_chunk_embeddings(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(mut payload): Json<CreateChunkEmbeddingsRequest>,
) -> Result<Json<ApiResponse<Vec<ChunkEmbedding>>>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| into_error_response(AppError::Database(e)))?;

    if !has_access {
        return Err(into_error_response(AppError::Forbidden));
    }

    // Ensure the path parameter overrides any document_id in the body.
    payload.document_id = document_id;

    let service = build_service(&state);

    service
        .create_chunk_embeddings(payload)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/embeddings/document/:id
///
/// Return the stored document-level embedding for the given document.
/// Requires authentication.
async fn get_document_embedding(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DocumentEmbedding>>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| into_error_response(AppError::Database(e)))?;

    if !has_access {
        return Err(into_error_response(AppError::Forbidden));
    }

    let embedding: Option<DocumentEmbedding> = sqlx::query_as(
        "SELECT * FROM document_embeddings WHERE document_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        into_error_response(AppError::Database(e))
    })?;

    let embedding = embedding.ok_or_else(|| {
        into_error_response(AppError::NotFound(format!(
            "Embedding not found for document {document_id}"
        )))
    })?;

    Ok(ApiResponse::ok(embedding))
}

/// POST /api/embeddings/search
///
/// Perform a semantic search against the chunk embedding store.
/// Requires authentication. Results are filtered to documents the user can access.
async fn semantic_search(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(mut payload): Json<SemanticSearchRequest>,
) -> Result<Json<ApiResponse<Vec<SemanticSearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
    if payload.query.trim().is_empty() {
        return Err(into_error_response(AppError::Validation(
            "Query must not be empty".into(),
        )));
    }

    // Set user_id to filter results to accessible documents
    payload.user_id = Some(auth_user.id);

    let service = build_service(&state);

    service
        .semantic_search(payload)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/embeddings/similar/:id
///
/// Return documents similar to the given document using vector similarity.
/// Requires authentication.
async fn find_similar_documents(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Query(query): Query<SimilarQuery>,
) -> Result<Json<ApiResponse<Vec<SemanticSearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| into_error_response(AppError::Database(e)))?;

    if !has_access {
        return Err(into_error_response(AppError::Forbidden));
    }

    let limit = query.limit.unwrap_or(10).min(50);
    let service = build_service(&state);

    service
        .find_similar_documents(document_id, limit)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/embeddings/stats
///
/// Return overall embedding statistics (document counts, queue depth, etc.).
/// Requires authentication (admin-level stats).
async fn get_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<ApiResponse<EmbeddingStats>>, (StatusCode, Json<serde_json::Value>)> {
    let service = build_service(&state);

    service
        .get_stats()
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// POST /api/embeddings/queue/:id
///
/// Add the given document to the async embedding generation queue.
/// Requires authentication.
async fn queue_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<QueueRequest>,
) -> Result<Json<ApiResponse<EmbeddingQueueEntry>>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| into_error_response(AppError::Database(e)))?;

    if !has_access {
        return Err(into_error_response(AppError::Forbidden));
    }

    let service = build_service(&state);

    service
        .queue_document(document_id, payload.priority)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the embeddings router.
///
/// All routes are relative to `/api/embeddings` (as mounted in `routes/mod.rs`).
pub fn router() -> Router<AppState> {
    Router::new()
        // Document embedding
        .route("/document/{id}", post(create_document_embedding))
        .route("/document/{id}", get(get_document_embedding))
        // Chunk embeddings
        .route("/chunks/{id}", post(create_chunk_embeddings))
        // Search
        .route("/search", post(semantic_search))
        .route("/similar/{id}", get(find_similar_documents))
        // Stats
        .route("/stats", get(get_stats))
        // Queue
        .route("/queue/{id}", post(queue_document))
}
