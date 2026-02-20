//! Document Understanding API routes
//!
//! Exposes the AI-powered document analysis pipeline via HTTP.
//!
//! # Endpoints
//!
//! - `POST /api/documents/{id}/understand` — Trigger Claude analysis for a document
//! - `GET  /api/documents/{id}/understanding` — Retrieve cached analysis result

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use secrecy::ExposeSecret;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::AuthUser,
    models::{DocumentUnderstanding, DocumentUnderstandingResponse},
    services::{EmbeddingConfig, EmbeddingService, UnderstandingService},
    AppState,
};

/// Response returned after a successful document analysis
#[derive(Debug, Serialize)]
pub struct UnderstandingApiResponse {
    pub success: bool,
    pub document_id: Uuid,
    pub data: DocumentUnderstandingResponse,
}

/// Response for a cached (previously computed) understanding
#[derive(Debug, Serialize)]
pub struct CachedUnderstandingResponse {
    pub success: bool,
    pub document_id: Uuid,
    pub data: DocumentUnderstanding,
}

/// Trigger AI analysis for a document and persist the result
///
/// `POST /api/documents/{id}/understand`
///
/// Calls Claude to extract topics, summary, insights, technologies, and target
/// roles from the document. The result is upserted so subsequent `GET` calls
/// return the cached value immediately. Requires authentication.
async fn analyze_document(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<UnderstandingApiResponse>> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(AppError::Database)?;

    if !has_access {
        return Err(AppError::Forbidden);
    }

    let understanding_service = UnderstandingService::new(state.config.clone());
    let analysis = understanding_service
        .analyze_document_by_id(&state.db, document_id)
        .await?;

    // Persist so future GET /understanding returns instantly
    let embedding_config = build_embedding_config(&state);
    let embedding_service = EmbeddingService::new(state.db.clone(), embedding_config);
    embedding_service
        .save_document_understanding(document_id, analysis.clone())
        .await?;

    Ok(Json(UnderstandingApiResponse {
        success: true,
        document_id,
        data: analysis,
    }))
}

/// Retrieve a previously computed document understanding from the database
///
/// `GET /api/documents/{id}/understanding`
///
/// Returns `404` if the document has not been analyzed yet.
/// Requires authentication.
async fn get_understanding(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> Result<Json<CachedUnderstandingResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Verify user has access to this document
    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE id = $1 AND (user_id = $2 OR is_public = true))",
    )
    .bind(document_id)
    .bind(auth_user.id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    ))?;

    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "success": false,
                "error": "Access denied to this document"
            })),
        ));
    }

    let embedding_config = build_embedding_config(&state);
    let embedding_service = EmbeddingService::new(state.db.clone(), embedding_config);

    match embedding_service
        .get_document_understanding(document_id)
        .await
    {
        Ok(Some(understanding)) => Ok(Json(CachedUnderstandingResponse {
            success: true,
            document_id,
            data: understanding,
        })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "error": format!(
                    "Document {document_id} has not been analyzed yet. \
                     POST to /documents/{document_id}/understand first."
                )
            })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        )),
    }
}

/// Build an [`EmbeddingConfig`] from application state
fn build_embedding_config(state: &AppState) -> EmbeddingConfig {
    EmbeddingConfig {
        openai_api_key: state
            .config
            .openai_api_key
            .as_ref()
            .map(|k| k.expose_secret().to_string()),
        ..EmbeddingConfig::default()
    }
}

/// Build the router for document understanding endpoints
///
/// These routes are nested under `/documents` in the main router:
/// - `POST /documents/{id}/understand`
/// - `GET  /documents/{id}/understanding`
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{id}/understand", post(analyze_document))
        .route("/{id}/understanding", get(get_understanding))
}
