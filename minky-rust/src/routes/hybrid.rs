//! Hybrid Search Routes
//!
//! QMD-inspired hybrid search API endpoints.

use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::hierarchical_context::{Collection, CollectionContext, ContextAnnotation},
    services::hybrid_search::{
        HybridSearchRequest, HybridSearchResponse, HybridSearchService, SearchMode,
    },
    services::query_expansion::{QueryExpansionRequest, QueryExpansionResponse, QueryExpansionService},
    AppState,
};

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        // Search endpoints
        .route("/search", post(hybrid_search))
        .route("/search/keyword", post(keyword_search))
        .route("/search/vector", post(vector_search))
        .route("/search/deep", post(deep_search))
        // Query expansion
        .route("/expand", post(expand_query))
        // Collections
        .route("/collections", get(list_collections))
        .route("/collections", post(create_collection))
        .route("/collections/:id", get(get_collection))
        .route("/collections/:id/context", post(add_collection_context))
        // Context
        .route("/context/:path", get(get_context))
        .route("/context", post(add_context))
        // Status
        .route("/status", get(search_status))
}

// ---------------------------------------------------------------------------
// Search Endpoints
// ---------------------------------------------------------------------------

/// Full hybrid search (configurable mode)
async fn hybrid_search(
    State(state): State<AppState>,
    Json(req): Json<HybridSearchRequest>,
) -> AppResult<Json<HybridSearchResponse>> {
    let service = HybridSearchService::new(state.db.clone());
    let response = service.search(req).await?;
    Ok(Json(response))
}

/// Keyword-only search (BM25)
async fn keyword_search(
    State(state): State<AppState>,
    Json(req): Json<SimpleSearchRequest>,
) -> AppResult<Json<HybridSearchResponse>> {
    let service = HybridSearchService::new(state.db.clone());
    let response = service.search(HybridSearchRequest {
        query: req.query,
        mode: SearchMode::Keyword,
        limit: req.limit.unwrap_or(20),
        threshold: req.threshold.unwrap_or(0.0),
        expand_query: false,
        rerank: false,
        collection_id: req.collection_id,
        user_id: req.user_id,
    }).await?;
    Ok(Json(response))
}

/// Vector-only search (semantic)
async fn vector_search(
    State(state): State<AppState>,
    Json(req): Json<SimpleSearchRequest>,
) -> AppResult<Json<HybridSearchResponse>> {
    let service = HybridSearchService::new(state.db.clone());
    let response = service.search(HybridSearchRequest {
        query: req.query,
        mode: SearchMode::Vector,
        limit: req.limit.unwrap_or(20),
        threshold: req.threshold.unwrap_or(0.5),
        expand_query: false,
        rerank: false,
        collection_id: req.collection_id,
        user_id: req.user_id,
    }).await?;
    Ok(Json(response))
}

/// Deep search (full hybrid with re-ranking)
async fn deep_search(
    State(state): State<AppState>,
    Json(req): Json<SimpleSearchRequest>,
) -> AppResult<Json<HybridSearchResponse>> {
    let service = HybridSearchService::new(state.db.clone());
    let response = service.search(HybridSearchRequest {
        query: req.query,
        mode: SearchMode::Deep,
        limit: req.limit.unwrap_or(20),
        threshold: req.threshold.unwrap_or(0.2),
        expand_query: true,
        rerank: true,
        collection_id: req.collection_id,
        user_id: req.user_id,
    }).await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct SimpleSearchRequest {
    query: String,
    limit: Option<usize>,
    threshold: Option<f32>,
    collection_id: Option<Uuid>,
    user_id: Option<i32>,
}

// ---------------------------------------------------------------------------
// Query Expansion
// ---------------------------------------------------------------------------

/// Expand a query into alternative phrasings
async fn expand_query(
    State(state): State<AppState>,
    Json(req): Json<QueryExpansionRequest>,
) -> AppResult<Json<QueryExpansionResponse>> {
    let service = QueryExpansionService::new(state.config.clone());
    let response = service.expand(req).await?;
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

/// List all collections
async fn list_collections(
    State(state): State<AppState>,
    Query(params): Query<ListCollectionsParams>,
) -> AppResult<Json<Vec<CollectionSummary>>> {
    let limit = params.limit.unwrap_or(50);

    let rows = sqlx::query_as::<_, CollectionRow>(
        r#"
        SELECT id, name, description, patterns, parent_id, context, document_count, created_at, updated_at
        FROM collections
        ORDER BY name
        LIMIT $1
        "#
    )
    .bind(limit as i32)
    .fetch_all(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?;

    let collections: Vec<CollectionSummary> = rows.into_iter().map(|r| CollectionSummary {
        id: r.id,
        name: r.name,
        description: r.description,
        document_count: r.document_count,
        has_children: false, // TODO: compute from query
    }).collect();

    Ok(Json(collections))
}

#[derive(Debug, Deserialize)]
struct ListCollectionsParams {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct CollectionSummary {
    id: Uuid,
    name: String,
    description: Option<String>,
    document_count: i64,
    has_children: bool,
}

#[derive(sqlx::FromRow)]
struct CollectionRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    patterns: serde_json::Value,
    parent_id: Option<Uuid>,
    context: serde_json::Value,
    document_count: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Create a new collection
async fn create_collection(
    State(state): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> AppResult<Json<Collection>> {
    let id = Uuid::new_v4();
    let patterns = serde_json::to_value(&req.patterns).unwrap_or_default();
    let context = serde_json::to_value(&req.context.clone().unwrap_or_default()).unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO collections (id, name, description, patterns, parent_id, context)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#
    )
    .bind(id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&patterns)
    .bind(req.parent_id)
    .bind(&context)
    .execute(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?;

    let collection = Collection {
        id,
        name: req.name,
        description: req.description,
        patterns: req.patterns,
        parent_id: req.parent_id,
        context: req.context.unwrap_or_default(),
        document_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    Ok(Json(collection))
}

#[derive(Debug, Deserialize)]
struct CreateCollectionRequest {
    name: String,
    description: Option<String>,
    #[serde(default)]
    patterns: Vec<String>,
    parent_id: Option<Uuid>,
    context: Option<CollectionContext>,
}

/// Get a collection by ID
async fn get_collection(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Collection>> {
    let row = sqlx::query_as::<_, CollectionRow>(
        r#"
        SELECT id, name, description, patterns, parent_id, context, document_count, created_at, updated_at
        FROM collections
        WHERE id = $1
        "#
    )
    .bind(id)
    .fetch_optional(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?
    .ok_or_else(|| crate::error::AppError::NotFound("Collection not found".into()))?;

    let patterns: Vec<String> = serde_json::from_value(row.patterns).unwrap_or_default();
    let context: CollectionContext = serde_json::from_value(row.context).unwrap_or_default();

    let collection = Collection {
        id: row.id,
        name: row.name,
        description: row.description,
        patterns,
        parent_id: row.parent_id,
        context,
        document_count: row.document_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };

    Ok(Json(collection))
}

/// Add context annotation to a collection
async fn add_collection_context(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddContextRequest>,
) -> AppResult<Json<ContextAnnotation>> {
    let annotation_id = Uuid::new_v4();
    let context_path = format!("minky://{}", req.path.unwrap_or_else(|| id.to_string()));

    sqlx::query(
        r#"
        INSERT INTO context_annotations (id, target_type, target_id, context_path, context_text, context_type, priority)
        VALUES ($1, 'collection', $2, $3, $4, $5, $6)
        "#
    )
    .bind(annotation_id)
    .bind(id)
    .bind(&context_path)
    .bind(&req.text)
    .bind(&req.context_type)
    .bind(req.priority.unwrap_or(0))
    .execute(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?;

    let annotation = ContextAnnotation {
        id: annotation_id,
        target_type: crate::models::hierarchical_context::ContextTargetType::Collection,
        target_id: id,
        context_path,
        context_text: req.text,
        context_type: parse_context_type(&req.context_type),
        priority: req.priority.unwrap_or(0),
        created_at: chrono::Utc::now(),
    };

    Ok(Json(annotation))
}

#[derive(Debug, Deserialize)]
struct AddContextRequest {
    text: String,
    path: Option<String>,
    #[serde(default = "default_context_type")]
    context_type: String,
    priority: Option<i32>,
}

fn default_context_type() -> String { "description".to_string() }

fn parse_context_type(s: &str) -> crate::models::hierarchical_context::ContextType {
    match s {
        "purpose" => crate::models::hierarchical_context::ContextType::Purpose,
        "audience" => crate::models::hierarchical_context::ContextType::Audience,
        "topic" => crate::models::hierarchical_context::ContextType::Topic,
        "temporal" => crate::models::hierarchical_context::ContextType::Temporal,
        "source" => crate::models::hierarchical_context::ContextType::Source,
        "custom" => crate::models::hierarchical_context::ContextType::Custom,
        _ => crate::models::hierarchical_context::ContextType::Description,
    }
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Get context for a path
async fn get_context(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> AppResult<Json<Vec<ContextAnnotation>>> {
    let full_path = format!("minky://{}", path);

    let rows = sqlx::query_as::<_, ContextRow>(
        r#"
        SELECT id, target_type, target_id, context_path, context_text, context_type, priority, created_at
        FROM context_annotations
        WHERE context_path LIKE $1 || '%'
        ORDER BY priority DESC
        "#
    )
    .bind(&full_path)
    .fetch_all(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?;

    let annotations: Vec<ContextAnnotation> = rows.into_iter().map(|r| ContextAnnotation {
        id: r.id,
        target_type: if r.target_type == "collection" {
            crate::models::hierarchical_context::ContextTargetType::Collection
        } else {
            crate::models::hierarchical_context::ContextTargetType::Document
        },
        target_id: r.target_id,
        context_path: r.context_path,
        context_text: r.context_text,
        context_type: parse_context_type(&r.context_type),
        priority: r.priority,
        created_at: r.created_at,
    }).collect();

    Ok(Json(annotations))
}

#[derive(sqlx::FromRow)]
struct ContextRow {
    id: Uuid,
    target_type: String,
    target_id: Uuid,
    context_path: String,
    context_text: String,
    context_type: String,
    priority: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Add a context annotation
async fn add_context(
    State(state): State<AppState>,
    Json(req): Json<CreateContextRequest>,
) -> AppResult<Json<ContextAnnotation>> {
    let id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO context_annotations (id, target_type, target_id, context_path, context_text, context_type, priority)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#
    )
    .bind(id)
    .bind(&req.target_type)
    .bind(req.target_id)
    .bind(&req.context_path)
    .bind(&req.text)
    .bind(&req.context_type)
    .bind(req.priority.unwrap_or(0))
    .execute(&state.db.clone())
    .await
    .map_err(crate::error::AppError::Database)?;

    let annotation = ContextAnnotation {
        id,
        target_type: if req.target_type == "collection" {
            crate::models::hierarchical_context::ContextTargetType::Collection
        } else {
            crate::models::hierarchical_context::ContextTargetType::Document
        },
        target_id: req.target_id,
        context_path: req.context_path,
        context_text: req.text,
        context_type: parse_context_type(&req.context_type),
        priority: req.priority.unwrap_or(0),
        created_at: chrono::Utc::now(),
    };

    Ok(Json(annotation))
}

#[derive(Debug, Deserialize)]
struct CreateContextRequest {
    target_type: String,
    target_id: Uuid,
    context_path: String,
    text: String,
    #[serde(default = "default_context_type")]
    context_type: String,
    priority: Option<i32>,
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Get search service status
async fn search_status(
    State(state): State<AppState>,
) -> AppResult<Json<SearchStatus>> {
    let doc_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(&state.db.clone())
        .await
        .map_err(crate::error::AppError::Database)?;

    let collection_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM collections")
        .fetch_one(&state.db.clone())
        .await
        .map_err(crate::error::AppError::Database)?;

    let embedding_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM document_embeddings")
        .fetch_one(&state.db.clone())
        .await
        .unwrap_or((0,));

    Ok(Json(SearchStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        document_count: doc_count.0,
        collection_count: collection_count.0,
        embedding_count: embedding_count.0,
        search_modes: vec![
            "keyword".to_string(),
            "vector".to_string(),
            "hybrid".to_string(),
            "deep".to_string(),
        ],
        features: SearchFeatures {
            fts_enabled: true,
            vector_enabled: true,
            query_expansion: true,
            llm_reranking: true,
            mcp_server: true,
        },
    }))
}

#[derive(Debug, Serialize)]
struct SearchStatus {
    version: String,
    document_count: i64,
    collection_count: i64,
    embedding_count: i64,
    search_modes: Vec<String>,
    features: SearchFeatures,
}

#[derive(Debug, Serialize)]
struct SearchFeatures {
    fts_enabled: bool,
    vector_enabled: bool,
    query_expansion: bool,
    llm_reranking: bool,
    mcp_server: bool,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_context_type() {
        assert_eq!(
            parse_context_type("purpose"),
            crate::models::hierarchical_context::ContextType::Purpose
        );
        assert_eq!(
            parse_context_type("unknown"),
            crate::models::hierarchical_context::ContextType::Description
        );
    }

    #[test]
    fn test_default_context_type() {
        assert_eq!(default_context_type(), "description");
    }
}
