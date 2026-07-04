use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    middleware::{AdminUser, OptionalAuthUser},
    models::{AutocompleteSuggestion, SearchHit, SearchQuery, SearchResponse},
    services::{AIService, SearchService},
    AppState,
};

/// Raw DB row type for document reindex query.
/// Fields: id, title, content, category_id, user_id, created_at, updated_at,
///         view_count, sensitivity, is_public
type DocumentRow = (
    String,
    String,
    String,
    Option<i32>,
    i32,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    i32,
    String,
    bool,
);

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
    auth_user: OptionalAuthUser,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<SearchResponseBody>> {
    // Query text must be tokenized the same way as indexed documents (see
    // SearchService::create_index's whitespace "korean" analyzer applied to
    // title_tokenized/content_tokenized), or un-segmented Korean queries
    // never match the segmented morpheme index.
    let search_service =
        SearchService::with_korean_tokenizer(&state.config, state.korean_tokenizer.clone())
            .await
            .map_err(AppError::Internal)?;

    let user_id = auth_user.0.map(|u| u.id);
    let response = search_service.search(query, user_id).await?;

    Ok(Json(SearchResponseBody {
        success: true,
        data: response,
    }))
}

/// Semantic search request (future feature: OpenSearch integration)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub limit: Option<i32>,
}

/// Semantic search response (future feature: OpenSearch integration)
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SemanticSearchResponse {
    pub success: bool,
    pub data: Vec<SearchHit>,
}

#[allow(dead_code)]
async fn semantic_search(
    State(state): State<AppState>,
    Json(payload): Json<SemanticSearchRequest>,
) -> AppResult<Json<SemanticSearchResponse>> {
    let ai_service = AIService::new(state.config.clone());
    let search_service = SearchService::new(&state.config).await
        .map_err(AppError::Internal)?;

    // Generate embedding for query
    let embedding_response = ai_service.generate_embedding(&payload.query).await?;

    // Search using embedding.
    // This handler is currently dead code; when wired up it should extract
    // OptionalAuthUser and pass the real user_id instead of None.
    let limit = payload.limit.unwrap_or(10).min(50);
    let hits = search_service.semantic_search(embedding_response.embedding, limit, None).await?;

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
    auth_user: OptionalAuthUser,
    Query(query): Query<AutocompleteQuery>,
) -> AppResult<Json<AutocompleteResponse>> {
    let search_service = SearchService::new(&state.config).await
        .map_err(AppError::Internal)?;

    let user_id = auth_user.0.map(|u| u.id);
    let limit = query.limit.unwrap_or(10).min(20);
    let suggestions = search_service.autocomplete(&query.q, limit, user_id).await
        .map_err(AppError::Internal)?;

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
    _admin: AdminUser,
) -> AppResult<Json<ReindexResponse>> {

    let search_service = SearchService::with_korean_tokenizer(&state.config, state.korean_tokenizer.clone())
        .await
        .map_err(AppError::Internal)?;

    // Full reindex is a bounded destructive cutover: delete the existing
    // index (if any) before recreating it, so a mapping change (e.g. the
    // mecab-ko whitespace analyzer replacing nori, or the removed
    // knn_vector field) actually takes effect. `create_index` alone
    // silently no-ops via the 400 resource_already_exists_exception path
    // when the index already exists with a stale mapping — correct for the
    // common "index already correct" case, but reindex_all is specifically
    // the operator-triggered rebuild path, so it must force a real rebuild.
    // OpenSearch is a rebuildable lexical projection of Postgres (Postgres
    // is the source of truth), so this delete+recreate is safe: worst case
    // is a brief search-unavailability window until bulk_index below
    // repopulates it.
    search_service
        .delete_index()
        .await
        .map_err(AppError::Internal)?;
    search_service.create_index().await
        .map_err(AppError::Internal)?;

    // Fetch all documents from database and index them.
    // sensitivity and is_public are fetched so the index correctly reflects
    // each document's access classification from the start.
    let documents: Vec<DocumentRow> = sqlx::query_as(
        r#"
        SELECT
            d.id::text,
            d.title,
            d.content,
            d.category_id,
            d.user_id,
            d.created_at,
            d.updated_at,
            d.view_count,
            d.sensitivity,
            d.is_public
        FROM documents d
        WHERE d.archived = false
        "#
    )
    .fetch_all(&state.db)
    .await?;

    // Pre-tokenize title/content with the shared mecab-ko tokenizer (falls
    // back to whitespace splitting when no dictionary is configured) into
    // separate *_tokenized fields analyzed by the whitespace "korean"
    // OpenSearch analyzer (see SearchService::create_index). "title"/
    // "content" themselves keep the original readable text unmodified so
    // _source retrieval never returns tokenized/corrupted display text —
    // only the *_tokenized copies are used for lexical matching. This is
    // the same tokenizer used for /api/korean/analyze (mechanism B2: one
    // shared pipeline for lexical indexing and, per the embedding
    // preprocessing step, pgvector embeddings).
    //
    // Tokenization is CPU-bound (mecab-ko Viterbi search over the whole
    // corpus), so it runs inside spawn_blocking rather than on the async
    // worker thread, matching the fastembed/EmbeddingService::local_model
    // discipline used elsewhere in this codebase.
    let korean_tokenizer = state.korean_tokenizer.clone();
    let search_documents: Vec<crate::models::SearchDocument> = tokio::task::spawn_blocking(move || {
        documents
            .into_iter()
            .map(|d| {
                let title_tokenized =
                    crate::services::tokenize_for_lexical_index_with(korean_tokenizer.as_ref(), &d.1);
                let content_tokenized =
                    crate::services::tokenize_for_lexical_index_with(korean_tokenizer.as_ref(), &d.2);
                crate::models::SearchDocument {
                    id: d.0,
                    title: d.1,
                    content: d.2,
                    title_tokenized,
                    content_tokenized,
                    category_id: d.3,
                    category_name: None,
                    tags: vec![],
                    user_id: d.4,
                    author_name: String::new(),
                    created_at: d.5,
                    updated_at: d.6,
                    view_count: d.7,
                    embedding: None,
                    sensitivity: d.8,
                    is_public: d.9,
                }
            })
            .collect()
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("reindex tokenization task panicked: {}", e)))?;

    let count = search_service.bulk_index(search_documents).await
        .map_err(AppError::Internal)?;

    Ok(Json(ReindexResponse {
        success: true,
        message: "Reindexing completed".to_string(),
        documents_indexed: count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FacetCount, SearchFacets, SearchHit, SearchResponse};
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // SearchResponseBody tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_search_response_body_serialization() {
        let body = SearchResponseBody {
            success: true,
            data: SearchResponse {
                hits: vec![],
                total: 0,
                page: 1,
                limit: 10,
                took_ms: 5,
                facets: SearchFacets {
                    categories: vec![],
                    tags: vec![],
                    date_ranges: vec![],
                },
            },
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"page\":1"));
    }

    #[test]
    fn test_search_response_body_with_hits() {
        let now = chrono::Utc::now();
        let body = SearchResponseBody {
            success: true,
            data: SearchResponse {
                hits: vec![SearchHit {
                    id: Uuid::new_v4(),
                    title: "Test Document".to_string(),
                    content_snippet: "This is a preview...".to_string(),
                    highlights: vec!["rust".to_string()],
                    score: 0.95,
                    category_name: Some("Tech".to_string()),
                    tags: vec!["rust".to_string()],
                    created_at: now,
                    updated_at: now,
                }],
                total: 1,
                page: 1,
                limit: 10,
                took_ms: 10,
                facets: SearchFacets {
                    categories: vec![FacetCount {
                        value: "Tech".to_string(),
                        count: 1,
                    }],
                    tags: vec![],
                    date_ranges: vec![],
                },
            },
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"score\":0.95"));
        assert!(json.contains("\"title\":\"Test Document\""));
    }

    // -------------------------------------------------------------------------
    // SemanticSearchRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_semantic_search_request_deserialization() {
        let json = r#"{"query": "find rust tutorials", "limit": 20}"#;
        let request: SemanticSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "find rust tutorials");
        assert_eq!(request.limit, Some(20));
    }

    #[test]
    fn test_semantic_search_request_without_limit() {
        let json = r#"{"query": "kubernetes basics"}"#;
        let request: SemanticSearchRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.query, "kubernetes basics");
        assert_eq!(request.limit, None);
    }

    // -------------------------------------------------------------------------
    // SemanticSearchResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_semantic_search_response_empty() {
        let response = SemanticSearchResponse {
            success: true,
            data: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":[]"));
    }

    // -------------------------------------------------------------------------
    // AutocompleteQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_autocomplete_query_deserialization() {
        let json = r#"{"q": "rus", "limit": 5}"#;
        let query: AutocompleteQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "rus");
        assert_eq!(query.limit, Some(5));
    }

    #[test]
    fn test_autocomplete_query_without_limit() {
        let json = r#"{"q": "doc"}"#;
        let query: AutocompleteQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.q, "doc");
        assert_eq!(query.limit, None);
    }

    #[test]
    fn test_autocomplete_limit_clamping_logic() {
        // Test the limit clamping logic used in autocomplete handler
        let test_cases = vec![
            (None, 10),    // default
            (Some(5), 5),  // below max
            (Some(20), 20), // at max
            (Some(50), 20), // above max, clamped to 20
        ];

        for (input, expected) in test_cases {
            let limit = input.unwrap_or(10).min(20);
            assert_eq!(limit, expected, "Failed for input {:?}", input);
        }
    }

    // -------------------------------------------------------------------------
    // AutocompleteResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_autocomplete_response_serialization() {
        let response = AutocompleteResponse {
            success: true,
            suggestions: vec![
                AutocompleteSuggestion {
                    text: "rust programming".to_string(),
                    score: 0.9,
                    document_count: 5,
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"text\":\"rust programming\""));
        assert!(json.contains("\"score\":0.9"));
        assert!(json.contains("\"document_count\":5"));
    }

    #[test]
    fn test_autocomplete_response_empty() {
        let response = AutocompleteResponse {
            success: true,
            suggestions: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"suggestions\":[]"));
    }

    // -------------------------------------------------------------------------
    // ReindexResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_reindex_response_serialization() {
        let response = ReindexResponse {
            success: true,
            message: "Reindexing completed".to_string(),
            documents_indexed: 150,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"documents_indexed\":150"));
        assert!(json.contains("\"message\":\"Reindexing completed\""));
    }

    #[test]
    fn test_reindex_response_zero_documents() {
        let response = ReindexResponse {
            success: true,
            message: "Reindexing completed".to_string(),
            documents_indexed: 0,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"documents_indexed\":0"));
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
