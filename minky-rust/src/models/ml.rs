use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Document cluster
#[derive(Debug, Serialize)]
pub struct DocumentCluster {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub centroid: Vec<f32>,
    pub document_count: i64,
    pub keywords: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Document with cluster assignment
#[derive(Debug, Serialize)]
pub struct ClusteredDocument {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub cluster_id: i32,
    pub cluster_name: String,
    pub similarity_score: f32,
}

/// Clustering request
#[derive(Debug, Deserialize)]
pub struct ClusteringRequest {
    pub num_clusters: Option<i32>,
    pub algorithm: Option<ClusteringAlgorithm>,
    pub category_id: Option<i32>,
    pub min_documents: Option<i32>,
}

/// Clustering algorithm
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClusteringAlgorithm {
    #[default]
    KMeans,
    DBSCAN,
    Hierarchical,
    Spectral,
}

/// Clustering job
#[derive(Debug, Serialize)]
pub struct ClusteringJob {
    pub id: String,
    pub status: JobStatus,
    pub algorithm: ClusteringAlgorithm,
    pub num_clusters: i32,
    pub documents_processed: i32,
    pub progress_percent: i32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
}

/// Clustering result
#[derive(Debug, Serialize)]
pub struct ClusteringResult {
    pub job_id: String,
    pub clusters: Vec<DocumentCluster>,
    pub assignments: Vec<ClusterAssignment>,
    pub metrics: ClusteringMetrics,
}

/// Cluster assignment
#[derive(Debug, Serialize)]
pub struct ClusterAssignment {
    pub document_id: uuid::Uuid,
    pub cluster_id: i32,
    pub similarity: f32,
}

/// Clustering quality metrics
#[derive(Debug, Serialize)]
pub struct ClusteringMetrics {
    pub silhouette_score: f32,
    pub inertia: f32,
    pub num_clusters: i32,
    pub total_documents: i32,
}

/// Document similarity
#[derive(Debug, Serialize)]
pub struct DocumentSimilarity {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub similarity_score: f32,
    pub shared_keywords: Vec<String>,
}

/// Similar documents request
#[derive(Debug, Deserialize)]
pub struct SimilarDocumentsRequest {
    pub document_id: uuid::Uuid,
    pub limit: Option<i32>,
    pub min_similarity: Option<f32>,
}

/// Topic model
#[derive(Debug, Serialize)]
pub struct Topic {
    pub id: i32,
    pub name: String,
    pub keywords: Vec<TopicKeyword>,
    pub document_count: i64,
    pub coherence_score: f32,
}

/// Topic keyword with weight
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopicKeyword {
    pub word: String,
    pub weight: f32,
}

/// Topic modeling request
#[derive(Debug, Deserialize)]
pub struct TopicModelingRequest {
    pub num_topics: Option<i32>,
    pub algorithm: Option<TopicAlgorithm>,
    pub category_id: Option<i32>,
}

/// Topic modeling algorithm
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TopicAlgorithm {
    #[default]
    LDA,
    NMF,
    BERTopic,
}

/// Document topics
#[derive(Debug, Serialize)]
pub struct DocumentTopics {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub topics: Vec<TopicAssignment>,
}

/// Topic assignment
#[derive(Debug, Clone, Serialize)]
pub struct TopicAssignment {
    pub topic_id: i32,
    pub topic_name: String,
    pub weight: f32,
}

use crate::models::analytics::{TimeSeriesPoint, TrendDirection};

/// Trend analysis result
#[derive(Debug, Serialize)]
pub struct TrendAnalysis {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub trending_topics: Vec<TrendingTopic>,
    pub trending_keywords: Vec<TrendingKeyword>,
    pub document_volume: Vec<TimeSeriesPoint>,
}

/// Trending topic
#[derive(Debug, Serialize)]
pub struct TrendingTopic {
    pub topic: String,
    pub count: i64,
    pub growth_rate: f32,
    pub trend_direction: TrendDirection,
}

/// Trending keyword
#[derive(Debug, Serialize)]
pub struct TrendingKeyword {
    pub keyword: String,
    pub count: i64,
    pub growth_rate: f32,
}

/// Anomaly detection result
#[derive(Debug, Serialize)]
pub struct AnomalyResult {
    pub document_id: uuid::Uuid,
    pub title: String,
    pub anomaly_score: f32,
    pub anomaly_type: AnomalyType,
    pub explanation: String,
}

/// Anomaly type
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyType {
    ContentOutlier,
    LengthAnomaly,
    TopicMismatch,
    StyleDeviation,
    TemporalAnomaly,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clustering_algorithm_default_is_kmeans() {
        assert!(matches!(ClusteringAlgorithm::default(), ClusteringAlgorithm::KMeans));
    }

    #[test]
    fn test_clustering_algorithm_serde_roundtrip() {
        let algorithms = [
            ClusteringAlgorithm::KMeans,
            ClusteringAlgorithm::DBSCAN,
            ClusteringAlgorithm::Hierarchical,
            ClusteringAlgorithm::Spectral,
        ];
        for alg in &algorithms {
            let json = serde_json::to_string(alg).unwrap();
            let back: ClusteringAlgorithm = serde_json::from_str(&json).unwrap();
            assert!(matches!((alg, &back),
                (ClusteringAlgorithm::KMeans, ClusteringAlgorithm::KMeans) |
                (ClusteringAlgorithm::DBSCAN, ClusteringAlgorithm::DBSCAN) |
                (ClusteringAlgorithm::Hierarchical, ClusteringAlgorithm::Hierarchical) |
                (ClusteringAlgorithm::Spectral, ClusteringAlgorithm::Spectral)
            ));
        }
    }

    #[test]
    fn test_job_status_default_is_pending() {
        assert!(matches!(JobStatus::default(), JobStatus::Pending));
    }

    #[test]
    fn test_job_status_serde_roundtrip() {
        let statuses = [
            JobStatus::Pending,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
        ];
        for status in &statuses {
            let json = serde_json::to_string(status).unwrap();
            let back: JobStatus = serde_json::from_str(&json).unwrap();
            assert!(matches!((status, &back),
                (JobStatus::Pending, JobStatus::Pending) |
                (JobStatus::Running, JobStatus::Running) |
                (JobStatus::Completed, JobStatus::Completed) |
                (JobStatus::Failed, JobStatus::Failed)
            ));
        }
    }

    #[test]
    fn test_topic_algorithm_default_is_lda() {
        assert!(matches!(TopicAlgorithm::default(), TopicAlgorithm::LDA));
    }

    #[test]
    fn test_topic_algorithm_serde_roundtrip() {
        let algorithms = [
            TopicAlgorithm::LDA,
            TopicAlgorithm::NMF,
            TopicAlgorithm::BERTopic,
        ];
        for alg in &algorithms {
            let json = serde_json::to_string(alg).unwrap();
            let back: TopicAlgorithm = serde_json::from_str(&json).unwrap();
            assert!(matches!((alg, &back),
                (TopicAlgorithm::LDA, TopicAlgorithm::LDA) |
                (TopicAlgorithm::NMF, TopicAlgorithm::NMF) |
                (TopicAlgorithm::BERTopic, TopicAlgorithm::BERTopic)
            ));
        }
    }

    #[test]
    fn test_topic_keyword_default_values() {
        let kw = TopicKeyword::default();
        assert!(kw.word.is_empty());
        assert_eq!(kw.weight, 0.0);
    }

    #[test]
    fn test_anomaly_type_serializes_snake_case() {
        let json = serde_json::to_string(&AnomalyType::ContentOutlier).unwrap();
        assert_eq!(json, "\"content_outlier\"");
        let json2 = serde_json::to_string(&AnomalyType::LengthAnomaly).unwrap();
        assert_eq!(json2, "\"length_anomaly\"");
    }
}
