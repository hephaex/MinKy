//! RAG (Retrieval-Augmented Generation) API routes
//!
//! Endpoints:
//!
//! | Method | Path                  | Description                             |
//! |--------|-----------------------|-----------------------------------------|
//! | POST   | /search/ask           | Natural-language Q&A backed by RAG      |
//! | POST   | /search/semantic      | Vector similarity search only           |
//! | GET    | /search/history       | Retrieve paginated search history       |

use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        RagAskRequest, RagAskResponse, RagSemanticSearchRequest,
        RagSemanticSearchResponse, SearchHistoryEntry, SearchHistoryQuery,
    },
    services::RagService,
    AppState,
};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the router for RAG endpoints under `/search`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ask", post(ask))
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
