use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        AnomalyType, ClusterAssignment, ClusteringAlgorithm, ClusteringMetrics, JobStatus,
        TimeSeriesPoint, TopicAlgorithm, TopicAssignment, TopicKeyword, TrendDirection,
        TrendingKeyword, TrendingTopic,
    };
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // TrendQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_trend_query_deserialization() {
        let json = r#"{"days": 14}"#;
        let query: TrendQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, Some(14));
    }

    #[test]
    fn test_trend_query_default() {
        let json = r#"{}"#;
        let query: TrendQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.days, None);
    }

    #[test]
    fn test_trend_query_default_logic() {
        // Test the default logic used in get_trends handler
        let test_cases = vec![
            (None, 30),    // default
            (Some(7), 7),  // specified
            (Some(90), 90),
        ];

        for (input, expected) in test_cases {
            let days = input.unwrap_or(30);
            assert_eq!(days, expected);
        }
    }

    // -------------------------------------------------------------------------
    // AnomalyQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_anomaly_query_deserialization() {
        let json = r#"{"category_id": 5}"#;
        let query: AnomalyQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.category_id, Some(5));
    }

    #[test]
    fn test_anomaly_query_without_category() {
        let json = r#"{}"#;
        let query: AnomalyQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.category_id, None);
    }

    // -------------------------------------------------------------------------
    // TopicModelingResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_topic_modeling_response_serialization() {
        let response = TopicModelingResponse {
            job_id: "job-123-abc".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"job_id\":\"job-123-abc\""));
    }

    // -------------------------------------------------------------------------
    // ClusteringRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clustering_request_deserialization() {
        let json = r#"{"num_clusters": 5, "algorithm": "kmeans"}"#;
        let request: ClusteringRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.num_clusters, Some(5));
    }

    #[test]
    fn test_clustering_request_minimal() {
        let json = r#"{}"#;
        let request: ClusteringRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.num_clusters, None);
        assert!(request.algorithm.is_none());
        assert!(request.category_id.is_none());
    }

    // -------------------------------------------------------------------------
    // ClusteringJob tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clustering_job_serialization() {
        let now = chrono::Utc::now();
        let job = ClusteringJob {
            id: "job-456".to_string(),
            status: JobStatus::Running,
            algorithm: ClusteringAlgorithm::KMeans,
            num_clusters: 5,
            documents_processed: 50,
            progress_percent: 50,
            created_at: now,
            completed_at: None,
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"id\":\"job-456\""));
        assert!(json.contains("\"status\":\"running\""));
        assert!(json.contains("\"algorithm\":\"kmeans\""));
        assert!(json.contains("\"progress_percent\":50"));
    }

    #[test]
    fn test_clustering_job_completed() {
        let now = chrono::Utc::now();
        let job = ClusteringJob {
            id: "job-789".to_string(),
            status: JobStatus::Completed,
            algorithm: ClusteringAlgorithm::DBSCAN,
            num_clusters: 3,
            documents_processed: 100,
            progress_percent: 100,
            created_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"progress_percent\":100"));
    }

    // -------------------------------------------------------------------------
    // ClusteringResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clustering_result_serialization() {
        let now = chrono::Utc::now();
        let result = ClusteringResult {
            job_id: "job-001".to_string(),
            clusters: vec![DocumentCluster {
                id: 1,
                name: "Technology".to_string(),
                description: Some("Tech related documents".to_string()),
                centroid: vec![0.1, 0.2, 0.3],
                document_count: 10,
                keywords: vec!["rust".to_string(), "web".to_string()],
                created_at: now,
                updated_at: now,
            }],
            assignments: vec![ClusterAssignment {
                document_id: Uuid::new_v4(),
                cluster_id: 1,
                similarity: 0.85,
            }],
            metrics: ClusteringMetrics {
                silhouette_score: 0.7,
                inertia: 100.0,
                num_clusters: 5,
                total_documents: 100,
            },
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"job_id\":\"job-001\""));
        assert!(json.contains("\"name\":\"Technology\""));
        assert!(json.contains("\"silhouette_score\":0.7"));
    }

    // -------------------------------------------------------------------------
    // DocumentCluster tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_document_cluster_serialization() {
        let now = chrono::Utc::now();
        let cluster = DocumentCluster {
            id: 1,
            name: "AI/ML".to_string(),
            description: None,
            centroid: vec![0.5, 0.5],
            document_count: 25,
            keywords: vec!["machine learning".to_string()],
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&cluster).unwrap();
        assert!(json.contains("\"name\":\"AI/ML\""));
        assert!(json.contains("\"document_count\":25"));
    }

    // -------------------------------------------------------------------------
    // ClusteredDocument tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clustered_document_serialization() {
        let doc = ClusteredDocument {
            document_id: Uuid::new_v4(),
            title: "Rust Guide".to_string(),
            cluster_id: 2,
            cluster_name: "Programming".to_string(),
            similarity_score: 0.92,
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"title\":\"Rust Guide\""));
        assert!(json.contains("\"similarity_score\":0.92"));
    }

    // -------------------------------------------------------------------------
    // DocumentSimilarity tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_document_similarity_serialization() {
        let sim = DocumentSimilarity {
            document_id: Uuid::new_v4(),
            title: "Similar Document".to_string(),
            similarity_score: 0.88,
            shared_keywords: vec!["api".to_string(), "rest".to_string()],
        };
        let json = serde_json::to_string(&sim).unwrap();
        assert!(json.contains("\"similarity_score\":0.88"));
        assert!(json.contains("\"api\""));
    }

    // -------------------------------------------------------------------------
    // SimilarDocumentsRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_similar_documents_request_deserialization() {
        let doc_id = Uuid::new_v4();
        let json = format!(r#"{{"document_id": "{}", "limit": 10, "min_similarity": 0.5}}"#, doc_id);
        let request: SimilarDocumentsRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.document_id, doc_id);
        assert_eq!(request.limit, Some(10));
        assert_eq!(request.min_similarity, Some(0.5));
    }

    // -------------------------------------------------------------------------
    // Topic tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_topic_serialization() {
        let topic = Topic {
            id: 1,
            name: "Web Development".to_string(),
            keywords: vec![
                TopicKeyword { word: "javascript".to_string(), weight: 0.9 },
                TopicKeyword { word: "react".to_string(), weight: 0.8 },
            ],
            document_count: 50,
            coherence_score: 0.75,
        };
        let json = serde_json::to_string(&topic).unwrap();
        assert!(json.contains("\"name\":\"Web Development\""));
        assert!(json.contains("\"coherence_score\":0.75"));
    }

    // -------------------------------------------------------------------------
    // TopicModelingRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_topic_modeling_request_deserialization() {
        let json = r#"{"num_topics": 10, "algorithm": "lda"}"#;
        let request: TopicModelingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.num_topics, Some(10));
    }

    #[test]
    fn test_topic_modeling_request_bertopic() {
        let json = r#"{"num_topics": 5, "algorithm": "bertopic"}"#;
        let request: TopicModelingRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(request.algorithm, Some(TopicAlgorithm::BERTopic)));
    }

    // -------------------------------------------------------------------------
    // DocumentTopics tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_document_topics_serialization() {
        let doc_topics = DocumentTopics {
            document_id: Uuid::new_v4(),
            title: "My Document".to_string(),
            topics: vec![
                TopicAssignment {
                    topic_id: 1,
                    topic_name: "Tech".to_string(),
                    weight: 0.7,
                },
                TopicAssignment {
                    topic_id: 2,
                    topic_name: "Business".to_string(),
                    weight: 0.3,
                },
            ],
        };
        let json = serde_json::to_string(&doc_topics).unwrap();
        assert!(json.contains("\"title\":\"My Document\""));
        assert!(json.contains("\"topic_name\":\"Tech\""));
    }

    // -------------------------------------------------------------------------
    // TrendAnalysis tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_trend_analysis_serialization() {
        let now = chrono::Utc::now();
        let analysis = TrendAnalysis {
            period_start: now,
            period_end: now,
            trending_topics: vec![TrendingTopic {
                topic: "AI".to_string(),
                count: 100,
                growth_rate: 0.5,
                trend_direction: TrendDirection::Up,
            }],
            trending_keywords: vec![TrendingKeyword {
                keyword: "rust".to_string(),
                count: 50,
                growth_rate: 0.3,
            }],
            document_volume: vec![TimeSeriesPoint {
                timestamp: now,
                value: 25,
            }],
        };
        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("\"trending_topics\""));
        assert!(json.contains("\"topic\":\"AI\""));
    }

    // -------------------------------------------------------------------------
    // AnomalyResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_anomaly_result_serialization() {
        let result = AnomalyResult {
            document_id: Uuid::new_v4(),
            title: "Outlier Doc".to_string(),
            anomaly_score: 0.95,
            anomaly_type: AnomalyType::ContentOutlier,
            explanation: "This document has unusual content".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"anomaly_score\":0.95"));
        assert!(json.contains("\"anomaly_type\":\"content_outlier\""));
    }

    #[test]
    fn test_anomaly_types() {
        let types = vec![
            (AnomalyType::ContentOutlier, "content_outlier"),
            (AnomalyType::LengthAnomaly, "length_anomaly"),
            (AnomalyType::TopicMismatch, "topic_mismatch"),
            (AnomalyType::StyleDeviation, "style_deviation"),
        ];

        for (anomaly_type, expected_str) in types {
            let result = AnomalyResult {
                document_id: Uuid::new_v4(),
                title: "Test".to_string(),
                anomaly_score: 0.5,
                anomaly_type,
                explanation: "Test".to_string(),
            };
            let json = serde_json::to_string(&result).unwrap();
            assert!(json.contains(expected_str), "Expected {} in {}", expected_str, json);
        }
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_router_creation() {
        let _router: Router<AppState> = router();
        // Should be creatable without panicking
    }
}
