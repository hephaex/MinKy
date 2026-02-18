use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::models::{
    AnomalyResult, ClusteredDocument, ClusteringJob, ClusteringRequest, ClusteringResult,
    DocumentCluster, DocumentSimilarity, DocumentTopics, SimilarDocumentsRequest, Topic,
    TopicModelingRequest, TrendAnalysis,
};
use crate::services::MlService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct TrendQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AnomalyQuery {
    pub category_id: Option<i32>,
}

/// Start clustering job
async fn start_clustering(
    State(state): State<AppState>,
    Json(request): Json<ClusteringRequest>,
) -> Result<Json<ClusteringJob>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .start_clustering(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get clustering result
async fn get_clustering_result(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<ClusteringResult>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .get_clustering_result(&job_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Result not found".to_string()))
}

/// Get all clusters
async fn get_clusters(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentCluster>>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .get_clusters()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get documents in cluster
async fn get_cluster_documents(
    State(state): State<AppState>,
    Path(cluster_id): Path<i32>,
) -> Result<Json<Vec<ClusteredDocument>>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .get_cluster_documents(cluster_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Find similar documents
async fn find_similar_documents(
    State(state): State<AppState>,
    Json(request): Json<SimilarDocumentsRequest>,
) -> Result<Json<Vec<DocumentSimilarity>>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .find_similar_documents(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Start topic modeling
async fn start_topic_modeling(
    State(state): State<AppState>,
    Json(request): Json<TopicModelingRequest>,
) -> Result<Json<TopicModelingResponse>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .start_topic_modeling(request)
        .await
        .map(|job_id| Json(TopicModelingResponse { job_id }))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get all topics
async fn get_topics(
    State(state): State<AppState>,
) -> Result<Json<Vec<Topic>>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .get_topics()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get document topics
async fn get_document_topics(
    State(state): State<AppState>,
    Path(document_id): Path<uuid::Uuid>,
) -> Result<Json<DocumentTopics>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .get_document_topics(document_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get trend analysis
async fn get_trends(
    State(state): State<AppState>,
    Query(query): Query<TrendQuery>,
) -> Result<Json<TrendAnalysis>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());
    let days = query.days.unwrap_or(30);

    service
        .get_trend_analysis(days)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Detect anomalies
async fn detect_anomalies(
    State(state): State<AppState>,
    Query(query): Query<AnomalyQuery>,
) -> Result<Json<Vec<AnomalyResult>>, (StatusCode, String)> {
    let service = MlService::new(state.db.clone());

    service
        .detect_anomalies(query.category_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, serde::Serialize)]
pub struct TopicModelingResponse {
    pub job_id: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/clustering", post(start_clustering))
        .route("/clustering/{job_id}", get(get_clustering_result))
        .route("/clusters", get(get_clusters))
        .route("/clusters/{cluster_id}/documents", get(get_cluster_documents))
        .route("/similar", post(find_similar_documents))
        .route("/topics", get(get_topics).post(start_topic_modeling))
        .route("/documents/{document_id}/topics", get(get_document_topics))
        .route("/trends", get(get_trends))
        .route("/anomalies", get(detect_anomalies))
}
