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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DocumentUnderstandingResponse;

    // -------------------------------------------------------------------------
    // UnderstandingApiResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_understanding_api_response_serialization() {
        let response = UnderstandingApiResponse {
            success: true,
            document_id: Uuid::new_v4(),
            data: DocumentUnderstandingResponse {
                topics: vec!["rust".to_string(), "web".to_string()],
                summary: "A document about Rust web development".to_string(),
                problem_solved: Some("How to build APIs".to_string()),
                insights: vec!["Use async/await".to_string()],
                technologies: vec!["axum".to_string(), "tokio".to_string()],
                relevant_for: vec!["backend developers".to_string()],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"topics\""));
        assert!(json.contains("\"rust\""));
        assert!(json.contains("\"summary\""));
    }

    #[test]
    fn test_understanding_api_response_with_empty_optional() {
        let response = UnderstandingApiResponse {
            success: true,
            document_id: Uuid::new_v4(),
            data: DocumentUnderstandingResponse {
                topics: vec![],
                summary: "Summary".to_string(),
                problem_solved: None,
                insights: vec![],
                technologies: vec![],
                relevant_for: vec![],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"topics\":[]"));
    }

    // -------------------------------------------------------------------------
    // CachedUnderstandingResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_cached_understanding_response_serialization() {
        let response = CachedUnderstandingResponse {
            success: true,
            document_id: Uuid::new_v4(),
            data: DocumentUnderstanding {
                id: Uuid::new_v4(),
                document_id: Uuid::new_v4(),
                topics: vec!["kubernetes".to_string()],
                summary: Some("K8s guide".to_string()),
                problem_solved: Some("Container orchestration".to_string()),
                insights: vec!["Use namespaces".to_string()],
                technologies: vec!["docker".to_string()],
                relevant_for: vec!["devops".to_string()],
                related_document_ids: vec![],
                analyzed_at: chrono::Utc::now(),
                analyzer_model: Some("claude-haiku".to_string()),
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"kubernetes\""));
        assert!(json.contains("\"analyzed_at\""));
    }

    #[test]
    fn test_cached_understanding_response_document_id_present() {
        let doc_id = Uuid::new_v4();
        let response = CachedUnderstandingResponse {
            success: true,
            document_id: doc_id,
            data: DocumentUnderstanding {
                id: Uuid::new_v4(),
                document_id: doc_id,
                topics: vec![],
                summary: None,
                problem_solved: None,
                insights: vec![],
                technologies: vec![],
                relevant_for: vec![],
                related_document_ids: vec![],
                analyzed_at: chrono::Utc::now(),
                analyzer_model: None,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(&doc_id.to_string()));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_routes_creation() {
        let _router: Router<AppState> = routes();
        // Should be creatable without panicking
    }
}
