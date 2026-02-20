//! Slack / Teams integration API routes
//!
//! Endpoints:
//!   POST /api/slack/extract          – Extract knowledge from a conversation
//!   GET  /api/slack/extract/{id}     – Get extraction result by conversation ID
//!   POST /api/slack/confirm          – Confirm or reject extracted knowledge
//!   GET  /api/slack/summary          – Extraction activity summary
//!   GET  /api/slack/oauth/callback   – Slack OAuth 2.0 callback (full token exchange)
//!   POST /api/slack/webhook          – Receive Slack Events API payloads

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    error::AppError,
    models::{
        ConfirmKnowledgeRequest, ExtractedKnowledge, ExtractionStatus, ExtractionSummary,
        MessageFilter, PlatformMessage,
    },
    services::{
        conversation_extraction_service::{ConversationExtractionService, ExtractionConfig},
        slack_oauth_service::{SlackOAuthConfig, SlackOAuthService},
    },
    AppState,
};

type HmacSha256 = Hmac<Sha256>;

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

/// Slack Events API webhook payload (minimal).
///
/// Slack sends various shapes depending on `type`.  We capture only the
/// fields needed for routing (`type`, `challenge`, `team_id`).  The full
/// event is kept as raw JSON for downstream processing.
#[derive(Debug, Deserialize)]
pub struct SlackWebhookPayload {
    /// Top-level type: `url_verification` or `event_callback`.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Present only for `url_verification` challenges.
    pub challenge: Option<String>,

    /// Workspace/team ID (present for `event_callback`).
    pub team_id: Option<String>,

    /// Full event object (present for `event_callback`).
    pub event: Option<serde_json::Value>,
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
    #[allow(dead_code)]
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Webhook event routing decision for processing
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum WebhookAction {
    /// url_verification challenge echoed back
    UrlVerification,
    /// event_callback – knowledge extraction queued
    KnowledgeExtractionQueued { team_id: String, event_type: String },
    /// event_callback – event type not relevant for knowledge extraction
    EventIgnored { event_type: String },
    /// Unknown top-level webhook type
    UnknownType { webhook_type: String },
}

// ---------------------------------------------------------------------------
// Slack Signature Verification
// ---------------------------------------------------------------------------

/// Verify Slack request signature using HMAC-SHA256.
///
/// Slack sends two headers with each request:
/// - `X-Slack-Request-Timestamp`: Unix timestamp when request was sent
/// - `X-Slack-Signature`: HMAC-SHA256 signature in format `v0=<hex>`
///
/// The signature is computed over: `v0:{timestamp}:{body}`
///
/// Returns `Ok(())` if signature is valid, `Err` otherwise.
pub fn verify_slack_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &[u8],
    signature: &str,
) -> Result<(), AppError> {
    // Check timestamp is not too old (5 minutes)
    let ts: i64 = timestamp.parse().map_err(|_| {
        AppError::Validation("Invalid X-Slack-Request-Timestamp".to_string())
    })?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err(AppError::Validation(
            "Request timestamp is too old (replay attack prevention)".to_string(),
        ));
    }

    // Compute expected signature
    let sig_basestring = format!(
        "v0:{}:{}",
        timestamp,
        String::from_utf8_lossy(body)
    );

    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|_| AppError::Configuration("Invalid signing secret length".to_string()))?;
    mac.update(sig_basestring.as_bytes());
    let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

    // Constant-time comparison to prevent timing attacks
    if expected.len() != signature.len() {
        return Err(AppError::Unauthorized);
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(signature.bytes()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return Err(AppError::Unauthorized);
    }

    Ok(())
}

/// Extract Slack signature headers from request.
fn extract_slack_headers(headers: &HeaderMap) -> Result<(String, String), AppError> {
    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("Missing X-Slack-Request-Timestamp header".to_string()))?;

    let signature = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Validation("Missing X-Slack-Signature header".to_string()))?;

    Ok((timestamp, signature))
}

// ---------------------------------------------------------------------------
// Pure business logic helpers (no I/O – unit-testable)
// ---------------------------------------------------------------------------

/// Determine what action to take for an incoming Slack webhook payload.
///
/// Returns a `WebhookAction` that describes how the handler should respond.
/// This function is pure (no I/O) so it can be exhaustively unit-tested.
pub fn classify_webhook_action(payload: &SlackWebhookPayload) -> WebhookAction {
    match payload.event_type.as_str() {
        "url_verification" => WebhookAction::UrlVerification,

        "event_callback" => {
            let team_id = payload
                .team_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());

            // Determine the inner event type
            let inner_type = payload
                .event
                .as_ref()
                .and_then(|e| e.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Only queue extraction for message events
            if inner_type == "message" || inner_type == "app_mention" {
                WebhookAction::KnowledgeExtractionQueued {
                    team_id,
                    event_type: inner_type,
                }
            } else {
                WebhookAction::EventIgnored {
                    event_type: inner_type,
                }
            }
        }

        other => WebhookAction::UnknownType {
            webhook_type: other.to_string(),
        },
    }
}

/// Extract conversation messages from a Slack `event_callback` payload.
///
/// Returns `None` if the event does not contain message data.
pub fn extract_messages_from_event(
    payload: &SlackWebhookPayload,
) -> Option<(String, Vec<PlatformMessage>)> {
    use crate::models::{MessagingPlatform, MessageReaction};
    use chrono::Utc;

    let event = payload.event.as_ref()?;
    let team_id = payload.team_id.as_deref().unwrap_or("unknown");

    let channel_id = event.get("channel")?.as_str()?.to_string();
    let user_id = event.get("user")?.as_str().unwrap_or("unknown").to_string();
    let text = event.get("text")?.as_str().unwrap_or("").to_string();
    let ts = event.get("ts")?.as_str().unwrap_or("0").to_string();
    let thread_ts = event
        .get("thread_ts")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let conversation_id = format!(
        "{}/{}",
        channel_id,
        thread_ts.as_deref().unwrap_or(&ts)
    );

    let message = PlatformMessage {
        id: ts.clone(),
        platform: MessagingPlatform::Slack,
        workspace_id: team_id.to_string(),
        channel_id: channel_id.clone(),
        channel_name: None,
        user_id,
        username: None,
        text,
        thread_ts,
        reply_count: 0,
        reactions: Vec::<MessageReaction>::new(),
        attachments: Vec::new(),
        posted_at: Utc::now(),
        captured_at: Utc::now(),
    };

    Some((conversation_id, vec![message]))
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
/// temporary `code`.  We exchange it for a bot token and persist the
/// workspace credentials to `platform_configs`.
async fn oauth_callback(
    State(state): State<AppState>,
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

    // Build OAuth config from application settings
    let (client_id, client_secret) = match (
        state.config.slack_client_id.as_deref(),
        state.config.slack_client_secret.as_ref(),
    ) {
        (Some(id), Some(secret)) => {
            use secrecy::ExposeSecret;
            (id.to_string(), secret.expose_secret().to_string())
        }
        _ => {
            return Err(into_error_response(AppError::Configuration(
                "SLACK_CLIENT_ID and SLACK_CLIENT_SECRET must be configured".to_string(),
            )));
        }
    };

    let mut oauth_config = SlackOAuthConfig::new(client_id, client_secret);
    if let Some(redirect_uri) = state.config.slack_redirect_uri.as_deref() {
        oauth_config = oauth_config.with_redirect_uri(redirect_uri);
    }

    let oauth_service = SlackOAuthService::new(oauth_config);

    // Exchange code for bot token
    let oauth_resp = oauth_service
        .exchange_code(&code)
        .await
        .map_err(into_error_response)?;

    // Persist workspace credentials
    let credentials =
        SlackOAuthService::save_workspace_credentials(&state.db, &oauth_resp)
            .await
            .map_err(into_error_response)?;

    let response = serde_json::json!({
        "message": "Slack workspace connected successfully.",
        "workspace_id": credentials.workspace_id,
        "workspace_name": credentials.workspace_name,
        "platform_config_id": credentials.platform_config_id,
    });

    Ok(ApiResponse::ok(response))
}

/// POST /api/slack/webhook
///
/// Receive Slack Events API payloads.
///
/// Slack sends two types of HTTP POST requests to this endpoint:
/// 1. `url_verification` – Slack verifies the endpoint by sending a challenge.
///    We must echo back the `challenge` value.
/// 2. `event_callback`   – An actual event (message posted, reaction added, etc.)
///    For `message` and `app_mention` events, spawns a background task to
///    attempt knowledge extraction.
///
/// Security: All requests are verified using HMAC-SHA256 signature validation
/// with the `X-Slack-Signature` header (except url_verification which happens
/// before signing secret is configured).
async fn slack_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    use secrecy::ExposeSecret;

    // Parse the JSON body
    let payload: SlackWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| into_error_response(AppError::Validation(format!("Invalid JSON: {e}"))))?;

    // For url_verification, skip signature check (signing secret not yet configured)
    // For all other events, verify the signature
    if payload.event_type != "url_verification" {
        if let Some(ref signing_secret) = state.config.slack_signing_secret {
            let (timestamp, signature) = extract_slack_headers(&headers)
                .map_err(into_error_response)?;

            verify_slack_signature(
                signing_secret.expose_secret(),
                &timestamp,
                &body,
                &signature,
            )
            .map_err(|e| {
                tracing::warn!(error = %e, "Slack webhook signature verification failed");
                into_error_response(e)
            })?;
        } else {
            tracing::warn!(
                "SLACK_SIGNING_SECRET not configured - webhook signature verification skipped. \
                 This is insecure for production use."
            );
        }
    }

    match classify_webhook_action(&payload) {
        WebhookAction::UrlVerification => {
            let challenge = payload.challenge.unwrap_or_default();
            Ok(Json(serde_json::json!({ "challenge": challenge })))
        }

        WebhookAction::KnowledgeExtractionQueued { team_id, event_type } => {
            tracing::info!(
                team_id = %team_id,
                event_type = %event_type,
                "Received Slack event_callback – queuing knowledge extraction"
            );

            // Extract messages from the event payload
            if let Some((conversation_id, messages)) = extract_messages_from_event(&payload) {
                let api_key = state
                    .config
                    .anthropic_api_key
                    .as_ref()
                    .map(|k| k.expose_secret().to_owned());

                // Spawn background task – acknowledge Slack immediately
                tokio::spawn(async move {
                    let config = ExtractionConfig {
                        anthropic_api_key: api_key,
                        ..Default::default()
                    };
                    let service = ConversationExtractionService::new(config);
                    let filter = MessageFilter::default();

                    match service.extract(&conversation_id, &messages, &filter).await {
                        Ok(result) => {
                            tracing::info!(
                                conversation_id = %conversation_id,
                                status = %result.status,
                                confidence = result.knowledge.confidence,
                                "Knowledge extraction completed"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                conversation_id = %conversation_id,
                                error = %e,
                                "Knowledge extraction skipped or failed"
                            );
                        }
                    }
                });
            }

            // Acknowledge immediately (Slack requires < 3s response)
            Ok(Json(serde_json::json!({ "ok": true, "queued": true })))
        }

        WebhookAction::EventIgnored { event_type } => {
            tracing::debug!(event_type = %event_type, "Ignoring non-message Slack event");
            Ok(Json(serde_json::json!({ "ok": true, "queued": false })))
        }

        WebhookAction::UnknownType { webhook_type } => {
            tracing::warn!(webhook_type = %webhook_type, "Unknown Slack webhook type");
            Ok(Json(serde_json::json!({ "ok": true })))
        }
    }
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
        .route("/webhook", post(slack_webhook))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Deserialization tests ----

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

    #[test]
    fn test_slack_webhook_payload_url_verification() {
        let json = r#"{"type":"url_verification","challenge":"abc123"}"#;
        let payload: SlackWebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.event_type, "url_verification");
        assert_eq!(payload.challenge.as_deref(), Some("abc123"));
        assert!(payload.team_id.is_none());
    }

    #[test]
    fn test_slack_webhook_payload_event_callback() {
        let json = r#"{
            "type": "event_callback",
            "team_id": "T01ABC",
            "event": {"type": "message", "text": "hello"}
        }"#;
        let payload: SlackWebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.event_type, "event_callback");
        assert_eq!(payload.team_id.as_deref(), Some("T01ABC"));
        assert!(payload.event.is_some());
    }

    #[test]
    fn test_slack_webhook_payload_missing_challenge_defaults_to_none() {
        let json = r#"{"type":"event_callback","team_id":"T1"}"#;
        let payload: SlackWebhookPayload = serde_json::from_str(json).unwrap();
        assert!(payload.challenge.is_none());
    }

    // ---- classify_webhook_action tests ----

    #[test]
    fn test_classify_url_verification() {
        let payload = SlackWebhookPayload {
            event_type: "url_verification".to_string(),
            challenge: Some("xyz".to_string()),
            team_id: None,
            event: None,
        };
        assert_eq!(classify_webhook_action(&payload), WebhookAction::UrlVerification);
    }

    #[test]
    fn test_classify_event_callback_message_queued() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T123".to_string()),
            event: Some(serde_json::json!({
                "type": "message",
                "channel": "C001",
                "user": "U001",
                "text": "Hello team",
                "ts": "1700000000.000"
            })),
        };
        let action = classify_webhook_action(&payload);
        assert!(matches!(
            action,
            WebhookAction::KnowledgeExtractionQueued { .. }
        ));
    }

    #[test]
    fn test_classify_event_callback_app_mention_queued() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T123".to_string()),
            event: Some(serde_json::json!({"type": "app_mention", "text": "@bot help"})),
        };
        let action = classify_webhook_action(&payload);
        assert!(matches!(
            action,
            WebhookAction::KnowledgeExtractionQueued { .. }
        ));
    }

    #[test]
    fn test_classify_event_callback_reaction_ignored() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T123".to_string()),
            event: Some(serde_json::json!({"type": "reaction_added", "reaction": "thumbsup"})),
        };
        let action = classify_webhook_action(&payload);
        assert!(matches!(action, WebhookAction::EventIgnored { .. }));
    }

    #[test]
    fn test_classify_unknown_webhook_type() {
        let payload = SlackWebhookPayload {
            event_type: "block_actions".to_string(),
            challenge: None,
            team_id: None,
            event: None,
        };
        let action = classify_webhook_action(&payload);
        assert!(matches!(action, WebhookAction::UnknownType { .. }));
    }

    #[test]
    fn test_classify_event_callback_no_team_id_still_queued() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: None,
            event: Some(serde_json::json!({"type": "message", "text": "hi"})),
        };
        let action = classify_webhook_action(&payload);
        if let WebhookAction::KnowledgeExtractionQueued { team_id, .. } = action {
            assert_eq!(team_id, "unknown");
        } else {
            panic!("Expected KnowledgeExtractionQueued");
        }
    }

    // ---- extract_messages_from_event tests ----

    #[test]
    fn test_extract_messages_from_event_basic() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T001".to_string()),
            event: Some(serde_json::json!({
                "type": "message",
                "channel": "C001",
                "user": "U001",
                "text": "Hello world",
                "ts": "1700000000.000"
            })),
        };
        let result = extract_messages_from_event(&payload);
        assert!(result.is_some());
        let (conv_id, messages) = result.unwrap();
        assert!(conv_id.starts_with("C001/"));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello world");
    }

    #[test]
    fn test_extract_messages_from_event_with_thread_ts() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T001".to_string()),
            event: Some(serde_json::json!({
                "type": "message",
                "channel": "C001",
                "user": "U002",
                "text": "Reply in thread",
                "ts": "1700000001.000",
                "thread_ts": "1700000000.000"
            })),
        };
        let result = extract_messages_from_event(&payload);
        assert!(result.is_some());
        let (conv_id, messages) = result.unwrap();
        assert_eq!(conv_id, "C001/1700000000.000");
        assert!(messages[0].thread_ts.is_some());
    }

    #[test]
    fn test_extract_messages_from_event_no_event_returns_none() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T001".to_string()),
            event: None,
        };
        let result = extract_messages_from_event(&payload);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_messages_from_event_no_channel_returns_none() {
        let payload = SlackWebhookPayload {
            event_type: "event_callback".to_string(),
            challenge: None,
            team_id: Some("T001".to_string()),
            event: Some(serde_json::json!({"type": "message", "text": "no channel field"})),
        };
        let result = extract_messages_from_event(&payload);
        assert!(result.is_none());
    }

    // ---- verify_slack_signature tests ----

    #[test]
    fn test_verify_slack_signature_valid() {
        // Test vector from Slack documentation
        let signing_secret = "8f742231b10e8888abcd99yyyzzz85a5";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = b"token=xyzz0WbapA4vBCDEFasx0q6G&team_id=T1DC2JH3J&api_app_id=A1DE&event=...";

        // Compute expected signature
        let sig_basestring = format!("v0:{}:{}", timestamp, String::from_utf8_lossy(body));
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes()).unwrap();
        mac.update(sig_basestring.as_bytes());
        let signature = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let result = verify_slack_signature(signing_secret, timestamp, body, &signature);
        assert!(result.is_ok(), "Valid signature should pass verification");
    }

    #[test]
    fn test_verify_slack_signature_invalid() {
        let signing_secret = "8f742231b10e8888abcd99yyyzzz85a5";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = b"token=xyzz0WbapA4vBCDEFasx0q6G";
        let wrong_signature = "v0=abc123wrongsignature";

        let result = verify_slack_signature(signing_secret, timestamp, body, wrong_signature);
        assert!(result.is_err(), "Invalid signature should fail verification");
    }

    #[test]
    fn test_verify_slack_signature_old_timestamp() {
        let signing_secret = "test-secret";
        let old_timestamp = (chrono::Utc::now().timestamp() - 600).to_string(); // 10 minutes ago
        let body = b"test body";
        let signature = "v0=doesnotmatter";

        let result = verify_slack_signature(signing_secret, &old_timestamp, body, signature);
        assert!(result.is_err(), "Old timestamp should fail (replay attack prevention)");
    }

    #[test]
    fn test_verify_slack_signature_invalid_timestamp() {
        let signing_secret = "test-secret";
        let invalid_timestamp = "not-a-number";
        let body = b"test body";
        let signature = "v0=doesnotmatter";

        let result = verify_slack_signature(signing_secret, invalid_timestamp, body, signature);
        assert!(result.is_err(), "Invalid timestamp should fail");
    }

    #[test]
    fn test_verify_slack_signature_length_mismatch() {
        let signing_secret = "test-secret";
        let timestamp = &chrono::Utc::now().timestamp().to_string();
        let body = b"test body";
        let short_signature = "v0=abc"; // Too short

        let result = verify_slack_signature(signing_secret, timestamp, body, short_signature);
        assert!(result.is_err(), "Signature with wrong length should fail");
    }
}
