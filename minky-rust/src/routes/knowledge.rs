//! Knowledge graph API routes
//!
//! Endpoints:
//! - GET /api/knowledge/graph    – full knowledge graph (nodes + edges)
//! - GET /api/knowledge/team     – team expertise map
//! - GET /api/knowledge/path     – find shortest path between two nodes
//! - GET /api/knowledge/clusters – detect and return clusters
//! - GET /api/knowledge/export   – export graph data (JSON or CSV)

use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use crate::{
    middleware::AuthUser,
    models::knowledge_graph::{
        ClusterQuery, ClusterResult, ExportFormat, ExportQuery, GraphExport, GraphPath,
        KnowledgeGraph, KnowledgeGraphQuery, PathQuery, TeamExpertiseMap,
    },
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

/// GET /api/knowledge/path
///
/// Find the shortest path between two nodes in the knowledge graph.
///
/// Query parameters:
/// - `from` – source node ID (required)
/// - `to` – target node ID (required)
/// - `max_depth` – maximum path length (default: 5, max: 10)
async fn get_graph_path(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<PathQuery>,
) -> Result<Json<ApiResponse<GraphPath>>, (StatusCode, Json<serde_json::Value>)> {
    let service = KnowledgeGraphService::new(state.db.clone());

    service
        .find_path(&query)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/knowledge/clusters
///
/// Detect and return clusters (communities) in the knowledge graph.
///
/// Query parameters:
/// - `max_iterations` – maximum label propagation iterations (default: 10, max: 100)
/// - `min_cluster_size` – minimum nodes per cluster (default: 2)
async fn get_graph_clusters(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<ClusterQuery>,
) -> Result<Json<ApiResponse<ClusterResult>>, (StatusCode, Json<serde_json::Value>)> {
    let service = KnowledgeGraphService::new(state.db.clone());

    service
        .analyze_clusters(query.max_iterations, query.min_cluster_size)
        .await
        .map(ApiResponse::ok)
        .map_err(into_error_response)
}

/// GET /api/knowledge/export
///
/// Export the knowledge graph data in JSON or CSV format.
///
/// Query parameters:
/// - `format` – export format: json (default) or csv
/// - `include_details` – include summary fields (default: true)
async fn get_graph_export(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let service = KnowledgeGraphService::new(state.db.clone());

    let export = service
        .export_graph(query.include_details)
        .await
        .map_err(into_error_response)?;

    match query.format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&export)
                .map_err(|e| into_error_response(crate::error::AppError::Internal(e.into())))?;

            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"knowledge-graph.json\"",
                    ),
                ],
                json,
            ))
        }
        ExportFormat::Csv => {
            let csv = export_to_csv(&export);

            Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"knowledge-graph.csv\"",
                    ),
                ],
                csv,
            ))
        }
    }
}

/// Convert graph export to CSV format.
fn export_to_csv(export: &GraphExport) -> String {
    let mut csv = String::new();

    // Nodes section
    csv.push_str("# Nodes\n");
    csv.push_str("id,label,type,document_count,created_at\n");
    for node in &export.nodes {
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",{},{}\n",
            escape_csv(&node.id),
            escape_csv(&node.label),
            escape_csv(&node.node_type),
            node.document_count,
            node.created_at.as_deref().unwrap_or("")
        ));
    }

    csv.push('\n');

    // Edges section
    csv.push_str("# Edges\n");
    csv.push_str("source,target,weight\n");
    for edge in &export.edges {
        csv.push_str(&format!(
            "\"{}\",\"{}\",{:.4}\n",
            escape_csv(&edge.source),
            escape_csv(&edge.target),
            edge.weight
        ));
    }

    csv
}

/// Escape double quotes in CSV values.
fn escape_csv(s: &str) -> String {
    s.replace('"', "\"\"")
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/graph", get(get_knowledge_graph))
        .route("/team", get(get_team_expertise))
        .route("/path", get(get_graph_path))
        .route("/clusters", get(get_graph_clusters))
        .route("/export", get(get_graph_export))
}
