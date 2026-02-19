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
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        ChunkEmbedding, CreateChunkEmbeddingsRequest, CreateDocumentEmbeddingRequest,
        DocumentEmbedding, EmbeddingModel, EmbeddingQueueEntry, EmbeddingStats,
        SemanticSearchRequest, SemanticSearchResult,
    },
    services::{EmbeddingConfig, EmbeddingService},
    AppState,
};

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

/// Generic success wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data,
        })
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/embeddings/document/:id
///
/// Generate (or regenerate) the document-level embedding for the given document.
async fn create_document_embedding(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DocumentEmbedding>>, (StatusCode, Json<serde_json::Value>)> {
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
/// The request body supplies the chunks to embed.
async fn create_chunk_embeddings(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(mut payload): Json<CreateChunkEmbeddingsRequest>,
) -> Result<Json<ApiResponse<Vec<ChunkEmbedding>>>, (StatusCode, Json<serde_json::Value>)> {
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
async fn get_document_embedding(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DocumentEmbedding>>, (StatusCode, Json<serde_json::Value>)> {
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
async fn semantic_search(
    State(state): State<AppState>,
    Json(payload): Json<SemanticSearchRequest>,
) -> Result<Json<ApiResponse<Vec<SemanticSearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
    if payload.query.trim().is_empty() {
        return Err(into_error_response(AppError::Validation(
            "Query must not be empty".into(),
        )));
    }

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
async fn find_similar_documents(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Query(query): Query<SimilarQuery>,
) -> Result<Json<ApiResponse<Vec<SemanticSearchResult>>>, (StatusCode, Json<serde_json::Value>)> {
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
async fn get_stats(
    State(state): State<AppState>,
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
async fn queue_document(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<QueueRequest>,
) -> Result<Json<ApiResponse<EmbeddingQueueEntry>>, (StatusCode, Json<serde_json::Value>)> {
    let service = build_service(&state);

    service
        .queue_document(document_id, payload.priority)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

fn into_error_response(err: AppError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &err {
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Validation(_) => StatusCode::BAD_REQUEST,
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::Forbidden => StatusCode::FORBIDDEN,
        AppError::Conflict(_) => StatusCode::CONFLICT,
        AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        AppError::Configuration(_) | AppError::Internal(_) | AppError::Database(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        AppError::ExternalService(_) => StatusCode::BAD_GATEWAY,
    };

    let body = Json(serde_json::json!({
        "success": false,
        "error": err.to_string(),
    }));

    (status, body)
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
