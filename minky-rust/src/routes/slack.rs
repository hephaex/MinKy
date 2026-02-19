//! Slack / Teams integration API routes
//!
//! Endpoints:
//!   POST /api/slack/extract          – Extract knowledge from a conversation
//!   GET  /api/slack/extract/{id}     – Get extraction result by conversation ID
//!   POST /api/slack/confirm          – Confirm or reject extracted knowledge
//!   GET  /api/slack/summary          – Extraction activity summary
//!   GET  /api/slack/oauth/callback   – Slack OAuth 2.0 callback

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    models::{
        ConfirmKnowledgeRequest, ExtractedKnowledge, ExtractionStatus, ExtractionSummary,
        MessageFilter, PlatformMessage,
    },
    services::conversation_extraction_service::{
        ConversationExtractionService, ExtractionConfig,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Response wrapper
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: T,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data,
        })
    }
}

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
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for knowledge extraction from a conversation thread.
#[derive(Debug, Deserialize)]
pub struct ExtractKnowledgeRequest {
    /// Unique identifier for the conversation thread.
    /// Format: `<channel_id>/<thread_ts>`, e.g. `C01ABC123/1700000000.000`
    pub conversation_id: String,

    /// All messages in the thread (root + replies).
    pub messages: Vec<PlatformMessage>,

    /// Optional filter to apply before extraction.
    #[serde(default)]
    pub filter: MessageFilter,
}

/// Response for a knowledge extraction request.
#[derive(Debug, Serialize)]
pub struct ExtractKnowledgeResponse {
    pub conversation_id: String,
    pub status: ExtractionStatus,
    pub knowledge: Option<ExtractedKnowledge>,
    pub message: String,
}

/// Query parameters for the summary endpoint.
#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    /// Only include items since this ISO-8601 datetime.
    #[allow(dead_code)]
    pub since: Option<chrono::DateTime<chrono::Utc>>,
}

/// Slack OAuth callback query parameters.
#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/slack/extract
///
/// Extract knowledge from a conversation thread.
/// Requires `ANTHROPIC_API_KEY` to be configured in the environment.
async fn extract_knowledge(
    State(state): State<AppState>,
    Json(req): Json<ExtractKnowledgeRequest>,
) -> Result<Json<ApiResponse<ExtractKnowledgeResponse>>, (StatusCode, Json<serde_json::Value>)> {
    use secrecy::ExposeSecret;

    let api_key = state
        .config
        .anthropic_api_key
        .as_ref()
        .map(|k| k.expose_secret().to_owned());

    let config = ExtractionConfig {
        anthropic_api_key: api_key,
        ..Default::default()
    };

    let service = ConversationExtractionService::new(config);

    match service.extract(&req.conversation_id, &req.messages, &req.filter).await {
        Ok(result) => {
            let response = ExtractKnowledgeResponse {
                conversation_id: req.conversation_id,
                status: result.status,
                knowledge: Some(result.knowledge),
                message: format!(
                    "Extraction completed. {} messages analysed across {} threads.",
                    result.stats.total_messages, result.stats.thread_count
                ),
            };
            Ok(ApiResponse::ok(response))
        }
        Err(AppError::Validation(msg)) => {
            // Thread did not meet quality criteria – return 200 with skipped status
            let response = ExtractKnowledgeResponse {
                conversation_id: req.conversation_id,
                status: ExtractionStatus::Skipped,
                knowledge: None,
                message: msg,
            };
            Ok(ApiResponse::ok(response))
        }
        Err(e) => Err(into_error_response(e)),
    }
}

/// GET /api/slack/summary
///
/// Return aggregated extraction statistics.
/// Currently returns a stub summary until DB persistence is wired.
async fn get_extraction_summary(
    State(_state): State<AppState>,
    Query(_query): Query<SummaryQuery>,
) -> Result<Json<ApiResponse<ExtractionSummary>>, (StatusCode, Json<serde_json::Value>)> {
    // TODO: query extraction_jobs table once DB schema is added.
    // For now return a placeholder so the endpoint is callable.
    let summary = ExtractionSummary {
        total_conversations: 0,
        total_messages: 0,
        knowledge_items_extracted: 0,
        high_quality_items: 0,
        pending_confirmation: 0,
        last_extraction_at: None,
    };

    Ok(ApiResponse::ok(summary))
}

/// POST /api/slack/confirm
///
/// Human-in-the-loop confirmation of extracted knowledge.
/// Accepts or rejects an extraction, optionally overriding title/summary.
async fn confirm_knowledge(
    State(_state): State<AppState>,
    Json(req): Json<ConfirmKnowledgeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    // TODO: persist confirmation to database once extraction_jobs table exists.
    let action = if req.confirmed { "confirmed" } else { "rejected" };
    let response = serde_json::json!({
        "extraction_id": req.extraction_id,
        "action": action,
    });
    Ok(ApiResponse::ok(response))
}

/// GET /api/slack/extract/{conversation_id}
///
/// Retrieve a previously extracted knowledge item by conversation ID.
async fn get_extraction(
    State(_state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    // TODO: look up extraction_jobs table.
    Err(into_error_response(AppError::NotFound(format!(
        "No extraction found for conversation '{conversation_id}'"
    ))))
}

/// GET /api/slack/oauth/callback
///
/// Slack OAuth 2.0 redirect handler.
///
/// After the user approves the Slack app, Slack redirects here with a
/// temporary `code`.  We exchange it for a bot token and store the workspace
/// credentials.
async fn oauth_callback(
    State(_state): State<AppState>,
    Query(params): Query<OAuthCallbackParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(error) = params.error {
        return Err(into_error_response(AppError::ExternalService(format!(
            "Slack OAuth error: {error}"
        ))));
    }

    let code = params.code.ok_or_else(|| {
        into_error_response(AppError::Validation(
            "Missing 'code' parameter from Slack OAuth callback".to_string(),
        ))
    })?;

    // TODO: exchange `code` for a bot token via https://slack.com/api/oauth.v2.access
    // and persist the workspace credentials to the database.
    //
    // For now, acknowledge the callback so the flow can be tested end-to-end
    // without real Slack credentials.
    let response = serde_json::json!({
        "message": "OAuth callback received. Token exchange not yet implemented.",
        "code_received": !code.is_empty(),
        "state": params.state,
    });

    Ok(ApiResponse::ok(response))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/extract", post(extract_knowledge))
        .route("/extract/{conversation_id}", get(get_extraction))
        .route("/confirm", post(confirm_knowledge))
        .route("/summary", get(get_extraction_summary))
        .route("/oauth/callback", get(oauth_callback))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_knowledge_request_default_filter() {
        let json = r#"{
            "conversation_id": "C1/ts1",
            "messages": []
        }"#;
        let req: ExtractKnowledgeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.conversation_id, "C1/ts1");
        // MessageFilter should default to no filters
        assert!(req.filter.platform.is_none());
        assert!(req.filter.channel_id.is_none());
    }

    #[test]
    fn test_oauth_callback_params_deserialise_with_error() {
        let json = r#"{"error": "access_denied"}"#;
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.error.as_deref(), Some("access_denied"));
        assert!(params.code.is_none());
    }

    #[test]
    fn test_oauth_callback_params_deserialise_with_code() {
        let json = r#"{"code": "abc123", "state": "random_state"}"#;
        let params: OAuthCallbackParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.code.as_deref(), Some("abc123"));
        assert_eq!(params.state.as_deref(), Some("random_state"));
    }

    #[test]
    fn test_confirm_knowledge_request_confirmed_true() {
        let json = r#"{"extraction_id": "ext-001", "confirmed": true}"#;
        let req: ConfirmKnowledgeRequest = serde_json::from_str(json).unwrap();
        assert!(req.confirmed);
        assert_eq!(req.extraction_id, "ext-001");
    }
}
