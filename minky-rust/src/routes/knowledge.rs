//! Knowledge graph API routes
//!
//! Endpoints:
//! - GET /api/knowledge/graph  – full knowledge graph (nodes + edges)
//! - GET /api/knowledge/team   – team expertise map

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::{
    middleware::AuthUser,
    models::knowledge_graph::{KnowledgeGraph, KnowledgeGraphQuery, TeamExpertiseMap},
    services::KnowledgeGraphService,
    AppState,
};

use super::common::{into_error_response, ApiResponse};

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/knowledge/graph
///
/// Build and return the full knowledge graph.
///
/// Query parameters:
/// - `threshold` – minimum cosine similarity for edges (default: 0.5)
/// - `max_edges` – maximum similar-document edges per node (default: 5, max: 20)
/// - `include_topics` – include topic nodes from AI analysis (default: true)
/// - `include_technologies` – include technology nodes (default: true)
/// - `include_insights` – include insight nodes (default: false)
/// - `max_documents` – maximum document nodes in graph (default: 100)
async fn get_knowledge_graph(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<KnowledgeGraphQuery>,
) -> Result<Json<ApiResponse<KnowledgeGraph>>, (StatusCode, Json<serde_json::Value>)> {
    let service = KnowledgeGraphService::new(state.db.clone());

    service
        .build_graph(&query)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/knowledge/team
///
/// Return the team expertise map derived from document authorship and AI analysis.
async fn get_team_expertise(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<ApiResponse<TeamExpertiseMap>>, (StatusCode, Json<serde_json::Value>)> {
    let service = KnowledgeGraphService::new(state.db.clone());

    service
        .build_team_expertise_map()
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/graph", get(get_knowledge_graph))
        .route("/team", get(get_team_expertise))
}
