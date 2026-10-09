//! Chat session persistence and synchronous message dispatch.
//!
//! Endpoints:
//! - Session CRUD: GET/POST /sessions, GET/DELETE /sessions/{id}
//! - Message append (record only): POST /sessions/{id}/messages
//! - Send message (RAG + record): POST /message
//!
//! POST /message calls RagService::ask() for non-streaming AI responses.
//! Streaming AI answers continue to use POST /search/ask/stream (rag.rs).
//! frontend/src/services/chatService.js is the consumer contract.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    middleware::AuthPrincipal,
    models::{RagAskRequest, RagSource},
    services::RagService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/{id}", get(get_session).delete(delete_session))
        .route("/sessions/{id}/messages", post(append_message))
        .route("/message", post(send_message))
}

#[derive(Debug, Serialize)]
pub struct Envelope<T> {
    pub success: bool,
    pub data: T,
}

fn ok<T>(data: T) -> Json<Envelope<T>> {
    Json(Envelope { success: true, data })
}

// ── Sessions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: Uuid,
    pub title: String,
    pub updated_at: DateTime<Utc>,
}

async fn list_sessions(
    State(state): State<AppState>,
    principal: AuthPrincipal,
) -> AppResult<Json<Envelope<Vec<SessionSummary>>>> {
    let sessions: Vec<SessionSummary> = sqlx::query_as(
        "SELECT id, title, updated_at FROM chat_sessions WHERE user_id = $1 ORDER BY updated_at DESC",
    )
    .bind(principal.effective_user_id())
    .fetch_all(&state.db)
    .await?;

    Ok(ok(sessions))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSessionRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be 1-255 characters"))]
    pub title: String,
}

async fn create_session(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Json(payload): Json<CreateSessionRequest>,
) -> AppResult<Json<Envelope<SessionSummary>>> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let session: SessionSummary = sqlx::query_as(
        "INSERT INTO chat_sessions (user_id, title) VALUES ($1, $2) RETURNING id, title, updated_at",
    )
    .bind(principal.effective_user_id())
    .bind(&payload.title)
    .fetch_one(&state.db)
    .await?;

    Ok(ok(session))
}

async fn delete_session(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Envelope<bool>>> {
    let result = sqlx::query("DELETE FROM chat_sessions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(principal.effective_user_id())
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Chat session not found".to_string()));
    }

    Ok(ok(true))
}

// ── Session detail (with messages) ───────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    role: String,
    content: String,
    sources: Option<sqlx::types::Json<serde_json::Value>>,
    tokens_used: Option<i32>,
    model: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOut {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl From<MessageRow> for MessageOut {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            role: row.role,
            content: row.content,
            sources: row.sources.map(|j| j.0),
            tokens_used: row.tokens_used,
            model: row.model,
            timestamp: row.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub id: Uuid,
    pub title: String,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageOut>,
}

async fn get_session(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Envelope<SessionDetail>>> {
    let session: Option<SessionSummary> = sqlx::query_as(
        "SELECT id, title, updated_at FROM chat_sessions WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(principal.effective_user_id())
    .fetch_optional(&state.db)
    .await?;

    let session = session.ok_or_else(|| AppError::NotFound("Chat session not found".to_string()))?;

    let rows: Vec<MessageRow> = sqlx::query_as(
        "SELECT id, role, content, sources, tokens_used, model, created_at
         FROM chat_messages WHERE session_id = $1 ORDER BY created_at ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(ok(SessionDetail {
        id: session.id,
        title: session.title,
        updated_at: session.updated_at,
        messages: rows.into_iter().map(MessageOut::from).collect(),
    }))
}

// ── Append message ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AppendMessageRequest {
    pub role: String,
    pub content: String,
    pub sources: Option<serde_json::Value>,
    pub tokens_used: Option<i32>,
    pub model: Option<String>,
}

fn validate_role(role: &str) -> AppResult<()> {
    if role == "user" || role == "assistant" {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "role must be 'user' or 'assistant', got '{role}'"
        )))
    }
}

async fn append_message(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Path(session_id): Path<Uuid>,
    Json(payload): Json<AppendMessageRequest>,
) -> AppResult<Json<Envelope<MessageOut>>> {
    validate_role(&payload.role)?;

    // Ownership check: 404 rather than leak existence of another user's session.
    let owned: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM chat_sessions WHERE id = $1 AND user_id = $2")
            .bind(session_id)
            .bind(principal.effective_user_id())
            .fetch_optional(&state.db)
            .await?;
    if owned.is_none() {
        return Err(AppError::NotFound("Chat session not found".to_string()));
    }

    let row: MessageRow = sqlx::query_as(
        "INSERT INTO chat_messages (session_id, role, content, sources, tokens_used, model)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, role, content, sources, tokens_used, model, created_at",
    )
    .bind(session_id)
    .bind(&payload.role)
    .bind(&payload.content)
    .bind(payload.sources.map(sqlx::types::Json))
    .bind(payload.tokens_used)
    .bind(&payload.model)
    .fetch_one(&state.db)
    .await?;

    sqlx::query("UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1")
        .bind(session_id)
        .execute(&state.db)
        .await?;

    Ok(ok(MessageOut::from(row)))
}

// ── Send message (RAG dispatch) ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub session_id: Uuid,
    pub content: String,
}

/// Flat response — chatService.sendMessage() returns response.data directly
/// without the .data.data unwrap other chatService methods perform,
/// so this MUST NOT be wrapped in Envelope<T>.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub content: String,         // RagAskResponse.answer
    pub sources: Vec<RagSource>, // RagAskResponse.sources
    pub tokens_used: u32,
    pub model: String,
}

async fn send_message(
    State(state): State<AppState>,
    principal: AuthPrincipal,
    Json(payload): Json<SendMessageRequest>,
) -> AppResult<Json<SendMessageResponse>> {
    // ── 입력 검증 (rag.rs::ask 핸들러와 동일 패턴) ──────────────────────
    if payload.content.trim().is_empty() {
        return Err(AppError::Validation("content must not be empty".into()));
    }
    if payload.content.len() > 2000 {
        return Err(AppError::Validation(
            "content must be at most 2000 characters".into(),
        ));
    }

    // ── 소유권 체크 ───────────────────────────────────────────────────────
    let owned: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM chat_sessions WHERE id = $1 AND user_id = $2")
            .bind(payload.session_id)
            .bind(principal.effective_user_id())
            .fetch_optional(&state.db)
            .await?;
    if owned.is_none() {
        return Err(AppError::NotFound("Chat session not found".to_string()));
    }

    // ── 사용자 메시지 DB 기록 (fire-and-forget, observable) ──────────────
    let session_id = payload.session_id;
    let user_content = payload.content.clone();
    let db = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = sqlx::query(
            "INSERT INTO chat_messages (session_id, role, content) VALUES ($1, 'user', $2)",
        )
        .bind(session_id)
        .bind(&user_content)
        .execute(&db)
        .await
        {
            tracing::warn!(error = %e, %session_id, "chat user message DB write failed");
        }
    });

    // ── RAG 호출 (non-streaming) ─────────────────────────────────────────
    // NOTE: RagService::ask() internally calls persist_history(), writing a
    // search_history row for this chat turn. This is accepted behavior;
    // chat turns will appear in search history attributed to the principal's
    // effective user (the JWT user, or api_keys.user_id for a service key).
    let rag = RagService::new(state.db.clone(), state.config.clone());
    let rag_resp = rag
        .ask(RagAskRequest {
            question: payload.content.clone(),
            top_k: 5,
            threshold: 0.7,
            user_id: Some(principal.effective_user_id()),
            include_sources: true,
        })
        .await?;

    // ── 어시스턴트 메시지 DB 기록 (fire-and-forget, observable) ──────────
    let assistant_content = rag_resp.answer.clone();
    let sources_json = serde_json::to_value(&rag_resp.sources).ok();
    let tokens = rag_resp.tokens_used as i32;
    let model_name = rag_resp.model.clone();
    let db2 = state.db.clone();
    tokio::spawn(async move {
        if let Err(e) = sqlx::query(
            "INSERT INTO chat_messages (session_id, role, content, sources, tokens_used, model)
             VALUES ($1, 'assistant', $2, $3, $4, $5)",
        )
        .bind(session_id)
        .bind(&assistant_content)
        .bind(sources_json.map(sqlx::types::Json))
        .bind(tokens)
        .bind(&model_name)
        .execute(&db2)
        .await
        {
            tracing::warn!(error = %e, %session_id, "chat assistant message DB write failed");
        }

        if let Err(e) = sqlx::query(
            "UPDATE chat_sessions SET updated_at = NOW() WHERE id = $1",
        )
        .bind(session_id)
        .execute(&db2)
        .await
        {
            tracing::warn!(error = %e, %session_id, "chat session updated_at refresh failed");
        }
    });

    // ── 플랫 응답 반환 (Envelope 미사용) ─────────────────────────────────
    Ok(Json(SendMessageResponse {
        content: rag_resp.answer,
        sources: rag_resp.sources,
        tokens_used: rag_resp.tokens_used,
        model: rag_resp.model,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_role_accepts_user_and_assistant() {
        assert!(validate_role("user").is_ok());
        assert!(validate_role("assistant").is_ok());
    }

    #[test]
    fn validate_role_rejects_anything_else() {
        assert!(validate_role("system").is_err());
        assert!(validate_role("").is_err());
        assert!(validate_role("User").is_err());
    }

    #[test]
    fn session_summary_serializes_camel_case_updated_at() {
        let s = SessionSummary { id: Uuid::nil(), title: "Hi".into(), updated_at: Utc::now() };
        let json = serde_json::to_value(&s).unwrap();
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("updated_at").is_none());
    }

    #[test]
    fn message_out_serializes_camel_case_tokens_used_and_timestamp_key() {
        let row = MessageRow {
            id: Uuid::nil(),
            role: "assistant".into(),
            content: "hi".into(),
            sources: None,
            tokens_used: Some(42),
            model: Some("solar-open2".into()),
            created_at: Utc::now(),
        };
        let out = MessageOut::from(row);
        let json = serde_json::to_value(&out).unwrap();
        assert!(json.get("tokensUsed").is_some(), "tokens_used must serialize as tokensUsed");
        assert!(json.get("timestamp").is_some(), "created_at must serialize under the `timestamp` key the frontend reads");
        assert!(json.get("created_at").is_none());
    }

    #[test]
    fn message_out_omits_null_optional_fields() {
        let row = MessageRow {
            id: Uuid::nil(),
            role: "user".into(),
            content: "hi".into(),
            sources: None,
            tokens_used: None,
            model: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_value(MessageOut::from(row)).unwrap();
        assert!(json.get("sources").is_none());
        assert!(json.get("tokensUsed").is_none());
        assert!(json.get("model").is_none());
    }

    #[test]
    fn envelope_wraps_success_and_data() {
        let json = serde_json::to_value(Envelope { success: true, data: 5 }).unwrap();
        assert_eq!(json["success"], true);
        assert_eq!(json["data"], 5);
    }
    #[test]
    fn send_message_response_serializes_flat_content_field() {
        let resp = SendMessageResponse {
            content: "42".into(),
            sources: vec![],
            tokens_used: 10,
            model: "solar-open2".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        // chatService.sendMessage reads response.content at the top level
        assert!(json.get("content").is_some(), "content must be top-level");
        assert!(json.get("sources").is_some());
        assert_eq!(json["tokensUsed"], 10, "tokens_used must serialize as camelCase tokensUsed");
        // Must NOT be wrapped in Envelope
        assert!(json.get("success").is_none(), "must not have Envelope.success");
        assert!(json.get("data").is_none(), "must not have Envelope.data");
    }
}
