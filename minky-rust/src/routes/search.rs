use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    models::{AutocompleteSuggestion, SearchHit, SearchQuery, SearchResponse},
    services::{AIService, SearchService},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(search))
        .route("/autocomplete", get(autocomplete))
        .route("/reindex", post(reindex_all))
}

#[derive(Debug, Serialize)]
pub struct SearchResponseBody {
    pub success: bool,
    pub data: SearchResponse,
}

async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<SearchResponseBody>> {
    let search_service = SearchService::new(&state.config).await
        .map_err(|e| AppError::Internal(e))?;

    let response = search_service.search(query).await?;

    Ok(Json(SearchResponseBody {
        success: true,
        data: response,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SemanticSearchResponse {
    pub success: bool,
    pub data: Vec<SearchHit>,
}

async fn semantic_search(
    State(state): State<AppState>,
    Json(payload): Json<SemanticSearchRequest>,
) -> AppResult<Json<SemanticSearchResponse>> {
    let ai_service = AIService::new(state.config.clone());
    let search_service = SearchService::new(&state.config).await
        .map_err(|e| AppError::Internal(e))?;

    // Generate embedding for query
    let embedding_response = ai_service.generate_embedding(&payload.query).await?;

    // Search using embedding
    let limit = payload.limit.unwrap_or(10).min(50);
    let hits = search_service.semantic_search(embedding_response.embedding, limit).await?;

    Ok(Json(SemanticSearchResponse {
        success: true,
        data: hits,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AutocompleteQuery {
    pub q: String,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteResponse {
    pub success: bool,
    pub suggestions: Vec<AutocompleteSuggestion>,
}

async fn autocomplete(
    State(state): State<AppState>,
    Query(query): Query<AutocompleteQuery>,
) -> AppResult<Json<AutocompleteResponse>> {
    let search_service = SearchService::new(&state.config).await
        .map_err(|e| AppError::Internal(e))?;

    let limit = query.limit.unwrap_or(10).min(20);
    let suggestions = search_service.autocomplete(&query.q, limit).await
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(AutocompleteResponse {
        success: true,
        suggestions,
    }))
}

#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub success: bool,
    pub message: String,
    pub documents_indexed: usize,
}

async fn reindex_all(
    State(state): State<AppState>,
) -> AppResult<Json<ReindexResponse>> {
    // TODO: Only allow admin users
    let _user_id = 1;

    let search_service = SearchService::new(&state.config).await
        .map_err(|e| AppError::Internal(e))?;

    // Create index if not exists
    search_service.create_index().await
        .map_err(|e| AppError::Internal(e))?;

    // Fetch all documents from database and index them
    let documents: Vec<(String, String, String, Option<i32>, i32, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, i32)> = sqlx::query_as(
        r#"
        SELECT
            d.id::text,
            d.title,
            d.content,
            d.category_id,
            d.user_id,
            d.created_at,
            d.updated_at,
            d.view_count
        FROM documents d
        "#
    )
    .fetch_all(&state.db)
    .await?;

    let search_documents: Vec<crate::models::SearchDocument> = documents.into_iter()
        .map(|d| crate::models::SearchDocument {
            id: d.0,
            title: d.1,
            content: d.2,
            category_id: d.3,
            category_name: None,
            tags: vec![],
            user_id: d.4,
            author_name: String::new(),
            created_at: d.5,
            updated_at: d.6,
            view_count: d.7,
            embedding: None,
        })
        .collect();

    let count = search_service.bulk_index(search_documents).await
        .map_err(|e| AppError::Internal(e))?;

    Ok(Json(ReindexResponse {
        success: true,
        message: "Reindexing completed".to_string(),
        documents_indexed: count,
    }))
}
