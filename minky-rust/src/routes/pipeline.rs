//! Pipeline API routes
//!
//! Endpoints for document ingestion and pipeline status

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::AuthPrincipal;
use crate::pipeline::{DocumentPipelineBuilder, IngestionInput, PipelineOutput};
use crate::AppState;

/// Request body for document ingestion
#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    /// Document title
    pub title: String,

    /// Document content (text or markdown)
    pub content: String,

    /// Whether document should be public
    #[serde(default)]
    pub is_public: bool,

    /// Skip AI analysis
    #[serde(default)]
    pub skip_analysis: bool,

    /// Skip embedding generation
    #[serde(default)]
    pub skip_embedding: bool,
}

/// Response for document ingestion
#[derive(Debug, Serialize)]
pub struct IngestResponse {
    /// Document ID
    pub document_id: Uuid,

    /// Document title
    pub title: String,

    /// Number of chunks created
    pub chunks_count: usize,

    /// Whether AI analysis was performed
    pub analyzed: bool,

    /// Extracted topics
    pub topics: Vec<String>,

    /// Document summary
    pub summary: Option<String>,
}

impl From<PipelineOutput> for IngestResponse {
    fn from(output: PipelineOutput) -> Self {
        Self {
            document_id: output.document_id,
            title: output.title,
            chunks_count: output.chunks_count,
            analyzed: output.analyzed,
            topics: output.topics,
            summary: output.summary,
        }
    }
}

/// Pipeline job status
#[derive(Debug, Serialize)]
pub struct PipelineStatus {
    /// Job ID
    pub job_id: Uuid,

    /// Current status
    pub status: String,

    /// Current stage (if running)
    pub current_stage: Option<String>,

    /// Document ID (if completed)
    pub document_id: Option<Uuid>,

    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Ingest a document through the pipeline
///
/// POST /api/pipeline/ingest
pub async fn ingest_document(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Json(request): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    // Build the pipeline based on request options
    let mut builder = DocumentPipelineBuilder::new()
        .pool(state.db.clone())
        .app_config(state.config.clone())
        .semantic_chunking(512);

    // Configure embedding if API key is available
    if let Some(ref api_key) = state.config.openai_api_key {
        use secrecy::ExposeSecret;
        if !request.skip_embedding {
            builder = builder.openai_embedding(api_key.expose_secret());
        }
    }

    // Configure analysis if API key is available
    if let Some(ref api_key) = state.config.upstage_api_key {
        use secrecy::ExposeSecret;
        if !request.skip_analysis {
            builder = builder.upstage_api_key(api_key.expose_secret());
        }
    }

    // Apply skip flags
    if request.skip_embedding {
        builder = builder.skip_embedding();
    }

    if request.skip_analysis {
        builder = builder.skip_analysis();
    }

    // Ownership always comes from the authenticated principal, never the body.
    builder = builder.user_id(principal.effective_user_id());

    if request.is_public {
        builder = builder.public();
    }

    let pipeline = builder.build();

    // Create ingestion input
    let input = IngestionInput::from_text(request.title, request.content);

    // Process through the pipeline
    // Keep the error kind: rejected input (e.g. a blocked SSRF target) is a 4xx,
    // not a 500. `From<PipelineError> for AppError` maps InvalidInput -> Validation.
    let output = pipeline.process(input).await.map_err(|e| {
        tracing::error!("Pipeline processing failed: {}", e);
        AppError::from(e)
    })?;

    Ok(Json(output.into()))
}

/// Get pipeline job status
///
/// GET /api/pipeline/jobs/{job_id}
pub async fn get_pipeline_status(
    State(state): State<AppState>,
    _principal: AuthPrincipal,
    Path(job_id): Path<Uuid>,
) -> Result<Json<PipelineStatus>, AppError> {
    type PipelineJobRow = (String, Option<String>, Option<Uuid>, Option<String>);
    let job: Option<PipelineJobRow> = sqlx::query_as(
        r#"
        SELECT status, current_stage, document_id, error_message
        FROM pipeline_jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?;

    match job {
        Some((status, current_stage, document_id, error_message)) => Ok(Json(PipelineStatus {
            job_id,
            status,
            current_stage,
            document_id,
            error_message,
        })),
        None => Err(AppError::NotFound(format!(
            "Pipeline job not found: {}",
            job_id
        ))),
    }
}

/// Ingest document from URL
///
/// POST /api/pipeline/ingest/url
#[derive(Debug, Deserialize)]
pub struct IngestUrlRequest {
    /// URL to fetch and ingest
    pub url: String,

    /// Whether document should be public
    #[serde(default)]
    pub is_public: bool,
}

pub async fn ingest_from_url(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Json(request): Json<IngestUrlRequest>,
) -> Result<Json<IngestResponse>, AppError> {
    // Build the pipeline
    let mut builder = DocumentPipelineBuilder::new()
        .pool(state.db.clone())
        .app_config(state.config.clone())
        .semantic_chunking(512);

    // Configure embedding if API key is available
    if let Some(ref api_key) = state.config.openai_api_key {
        use secrecy::ExposeSecret;
        builder = builder.openai_embedding(api_key.expose_secret());
    }

    // Configure analysis if API key is available
    if let Some(ref api_key) = state.config.upstage_api_key {
        use secrecy::ExposeSecret;
        builder = builder.upstage_api_key(api_key.expose_secret());
    }

    // Ownership always comes from the authenticated principal, never the body.
    builder = builder.user_id(principal.effective_user_id());

    if request.is_public {
        builder = builder.public();
    }

    let pipeline = builder.build();

    // Create ingestion input from URL
    let input = IngestionInput::from_url(&request.url);

    // Process through the pipeline
    // Keep the error kind: rejected input (e.g. a blocked SSRF target) is a 4xx,
    // not a 500. `From<PipelineError> for AppError` maps InvalidInput -> Validation.
    let output = pipeline.process(input).await.map_err(|e| {
        tracing::error!("Pipeline processing failed: {}", e);
        AppError::from(e)
    })?;

    Ok(Json(output.into()))
}

/// Create pipeline routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ingest", post(ingest_document))
        .route("/ingest/url", post(ingest_from_url))
        .route("/jobs/{job_id}", get(get_pipeline_status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_request_defaults() {
        let json = r#"{"title": "Test", "content": "Content"}"#;
        let request: IngestRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.title, "Test");
        assert_eq!(request.content, "Content");
        assert!(!request.is_public);
        assert!(!request.skip_analysis);
        assert!(!request.skip_embedding);
    }

    #[test]
    fn test_ingest_response_from_output() {
        let output = PipelineOutput {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            chunks_count: 5,
            analyzed: true,
            topics: vec!["rust".to_string()],
            summary: Some("A test document.".to_string()),
            replaced_document_id: None,
        };

        let response: IngestResponse = output.into();
        assert_eq!(response.title, "Test");
        assert_eq!(response.chunks_count, 5);
        assert!(response.analyzed);
    }
}
