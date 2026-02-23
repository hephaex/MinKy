//! RAG (Retrieval-Augmented Generation) API routes
//!
//! Endpoints:
//!
//! | Method | Path                  | Description                             |
//! |--------|-----------------------|-----------------------------------------|
//! | POST   | /search/ask           | Natural-language Q&A backed by RAG      |
//! | POST   | /search/ask/stream    | Streaming RAG response via SSE          |
//! | POST   | /search/semantic      | Vector similarity search only           |
//! | GET    | /search/history       | Retrieve paginated search history       |

use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::Stream;
use secrecy::ExposeSecret;
use serde::Serialize;
use std::convert::Infallible;
use tokio_stream::StreamExt;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        EmbeddingModel, RagAskRequest, RagAskResponse, RagSemanticSearchRequest,
        RagSemanticSearchResponse, RagSource, SearchHistoryEntry, SearchHistoryQuery,
        SemanticSearchRequest,
    },
    services::{
        anthropic_types::{AnthropicMessage, AnthropicStreamEvent, AnthropicStreamRequest},
        EmbeddingConfig, EmbeddingService, RagService,
    },
    AppState,
};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the router for RAG endpoints under `/search`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ask", post(ask))
        .route("/ask/stream", post(ask_stream))
        .route("/semantic", post(semantic_search))
        .route("/history", get(search_history))
}

// ---------------------------------------------------------------------------
// Response wrappers
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AskResponseBody {
    success: bool,
    data: RagAskResponse,
}

#[derive(Debug, Serialize)]
struct SemanticResponseBody {
    success: bool,
    data: RagSemanticSearchResponse,
}

#[derive(Debug, Serialize)]
struct HistoryResponseBody {
    success: bool,
    data: Vec<SearchHistoryEntry>,
    total: usize,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /search/ask
///
/// Accepts a natural-language question, retrieves relevant document chunks via
/// vector search, and generates a grounded answer using Claude.
///
/// # Request body
/// ```json
/// {
///   "question": "How does our authentication flow work?",
///   "top_k": 5,
///   "threshold": 0.7,
///   "include_sources": true
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "data": {
///     "answer": "...",
///     "sources": [...],
///     "tokens_used": 312,
///     "model": "claude-haiku-4-5-20251101"
///   }
/// }
/// ```
async fn ask(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<RagAskRequest>,
) -> AppResult<Json<AskResponseBody>> {
    if payload.question.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "question must not be empty".into(),
        ));
    }
    if payload.question.len() > 2000 {
        return Err(crate::error::AppError::Validation(
            "question must be at most 2000 characters".into(),
        ));
    }

    let service = RagService::new(state.db.clone(), state.config.clone());
    let response = service.ask(payload).await?;

    Ok(Json(AskResponseBody {
        success: true,
        data: response,
    }))
}

/// POST /search/semantic
///
/// Performs a vector similarity search and returns matching document chunks
/// without invoking an LLM for answer generation.
///
/// # Request body
/// ```json
/// {
///   "query": "authentication middleware",
///   "limit": 10,
///   "threshold": 0.6
/// }
/// ```
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "data": {
///     "results": [...],
///     "total": 3,
///     "query": "authentication middleware"
///   }
/// }
/// ```
async fn semantic_search(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<RagSemanticSearchRequest>,
) -> AppResult<Json<SemanticResponseBody>> {
    if payload.query.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "query must not be empty".into(),
        ));
    }

    let service = RagService::new(state.db.clone(), state.config.clone());
    let response = service.semantic_search(payload).await?;

    Ok(Json(SemanticResponseBody {
        success: true,
        data: response,
    }))
}

/// GET /search/history
///
/// Returns paginated search history entries for a given user.
///
/// # Query parameters
/// - `user_id` (optional) – filter by user
/// - `limit`   (optional, default 20, max 100)
/// - `offset`  (optional, default 0)
///
/// # Response
/// ```json
/// {
///   "success": true,
///   "data": [...],
///   "total": 5
/// }
/// ```
async fn search_history(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<SearchHistoryQuery>,
) -> AppResult<Json<HistoryResponseBody>> {
    let service = RagService::new(state.db.clone(), state.config.clone());
    let entries = service.get_history(query).await?;
    let total = entries.len();

    Ok(Json(HistoryResponseBody {
        success: true,
        data: entries,
        total,
    }))
}

// ---------------------------------------------------------------------------
// Streaming Handler
// ---------------------------------------------------------------------------

/// SSE event data for streaming responses
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    /// Sources retrieved before generation starts
    Sources { sources: Vec<RagSource> },
    /// Partial text delta
    Delta { text: String },
    /// Generation complete
    Done { tokens_used: u32, model: String },
    /// Error occurred
    Error { message: String },
}

/// POST /search/ask/stream
///
/// Streaming version of the RAG Q&A endpoint. Returns Server-Sent Events
/// with incremental text deltas as Claude generates the response.
///
/// # Request body
/// Same as `/search/ask`
///
/// # Response (SSE stream)
/// ```text
/// event: message
/// data: {"type":"sources","sources":[...]}
///
/// event: message
/// data: {"type":"delta","text":"The "}
///
/// event: message
/// data: {"type":"delta","text":"answer"}
///
/// event: message
/// data: {"type":"done","tokens_used":150,"model":"claude-haiku-4-5-20251101"}
/// ```
async fn ask_stream(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<RagAskRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        // Validate input
        if payload.question.trim().is_empty() {
            yield Ok(Event::default().data(
                serde_json::to_string(&StreamEvent::Error {
                    message: "question must not be empty".into(),
                }).unwrap_or_default()
            ));
            return;
        }

        if payload.question.len() > 2000 {
            yield Ok(Event::default().data(
                serde_json::to_string(&StreamEvent::Error {
                    message: "question must be at most 2000 characters".into(),
                }).unwrap_or_default()
            ));
            return;
        }

        // 1. Vector search for relevant chunks
        let openai_key = state.config.openai_api_key
            .as_ref()
            .map(|s| s.expose_secret().to_string());

        let embedding_config = EmbeddingConfig {
            openai_api_key: openai_key,
            voyage_api_key: None,
            default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
            chunk_size: 512,
            chunk_overlap: 50,
        };

        let embedding_service = EmbeddingService::new(state.db.clone(), embedding_config);

        let search_req = SemanticSearchRequest {
            query: payload.question.clone(),
            limit: payload.top_k.min(20) as i32,
            threshold: Some(payload.threshold),
            model: None,
            user_id: payload.user_id,
        };

        let chunks = match embedding_service.semantic_search(search_req).await {
            Ok(c) => c,
            Err(e) => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&StreamEvent::Error {
                        message: format!("Vector search failed: {}", e),
                    }).unwrap_or_default()
                ));
                return;
            }
        };

        // 2. Send sources first
        let sources: Vec<RagSource> = chunks.iter().map(|c| RagSource {
            document_id: c.document_id,
            document_title: c.document_title.clone(),
            chunk_text: c.chunk_text.clone().unwrap_or_default(),
            similarity: c.similarity,
        }).collect();

        if payload.include_sources {
            yield Ok(Event::default().data(
                serde_json::to_string(&StreamEvent::Sources { sources: sources.clone() })
                    .unwrap_or_default()
            ));
        }

        // 3. Build context
        let context = build_context_string(&chunks);

        // 4. Stream from Anthropic
        let api_key = match state.config.anthropic_api_key.as_ref() {
            Some(k) => k.expose_secret().to_string(),
            None => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&StreamEvent::Error {
                        message: "Anthropic API key not configured".into(),
                    }).unwrap_or_default()
                ));
                return;
            }
        };

        let system_prompt = "\
You are a helpful knowledge base assistant for a team. \
Your job is to answer questions accurately and concisely using ONLY the \
provided context. If the context does not contain enough information to \
answer the question, say so clearly rather than making up an answer. \
Always cite which source number(s) you used when possible.";

        let user_content = if context.is_empty() {
            format!("No relevant documents were found for this question.\n\nQuestion: {}", payload.question)
        } else {
            format!("Context from the knowledge base:\n\n{}\n\n---\n\nQuestion: {}", context, payload.question)
        };

        let request = AnthropicStreamRequest {
            model: "claude-haiku-4-5-20251101".to_string(),
            max_tokens: 1024,
            system: Some(system_prompt.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_content,
            }],
            stream: true,
        };

        let http_client = reqwest::Client::new();

        let response = match http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&StreamEvent::Error {
                        message: format!("Anthropic API request failed: {}", e),
                    }).unwrap_or_default()
                ));
                return;
            }
        };

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            yield Ok(Event::default().data(
                serde_json::to_string(&StreamEvent::Error {
                    message: format!("Anthropic API error: {}", error_text),
                }).unwrap_or_default()
            ));
            return;
        }

        // Parse SSE stream from Anthropic
        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut total_tokens: u32 = 0;
        let mut model_name = String::new();

        while let Some(chunk_result) = bytes_stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    yield Ok(Event::default().data(
                        serde_json::to_string(&StreamEvent::Error {
                            message: format!("Stream read error: {}", e),
                        }).unwrap_or_default()
                    ));
                    return;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events in buffer
            while let Some(pos) = buffer.find("\n\n") {
                let event_str = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                // Parse SSE event
                for line in event_str.lines() {
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        if json_str == "[DONE]" {
                            continue;
                        }

                        if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(json_str) {
                            match event {
                                AnthropicStreamEvent::MessageStart { message } => {
                                    model_name = message.model;
                                    if let Some(usage) = message.usage {
                                        total_tokens += usage.input_tokens;
                                    }
                                }
                                AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                    if let Some(text) = delta.text {
                                        yield Ok(Event::default().data(
                                            serde_json::to_string(&StreamEvent::Delta { text })
                                                .unwrap_or_default()
                                        ));
                                    }
                                }
                                AnthropicStreamEvent::MessageDelta { usage: Some(u), .. } => {
                                    total_tokens += u.output_tokens;
                                }
                                AnthropicStreamEvent::MessageDelta { usage: None, .. } => {}
                                AnthropicStreamEvent::MessageStop => {
                                    yield Ok(Event::default().data(
                                        serde_json::to_string(&StreamEvent::Done {
                                            tokens_used: total_tokens,
                                            model: model_name.clone(),
                                        }).unwrap_or_default()
                                    ));
                                }
                                AnthropicStreamEvent::Error { error } => {
                                    yield Ok(Event::default().data(
                                        serde_json::to_string(&StreamEvent::Error {
                                            message: error.message,
                                        }).unwrap_or_default()
                                    ));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Build context string from search results
fn build_context_string(chunks: &[crate::models::SemanticSearchResult]) -> String {
    if chunks.is_empty() {
        return String::new();
    }

    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let title = chunk.document_title.as_deref().unwrap_or("Untitled document");
            let text = chunk.chunk_text.as_deref().unwrap_or("");
            format!(
                "[Source {}] {} (similarity: {:.2})\n{}",
                i + 1,
                title,
                chunk.similarity,
                text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n")
}
