use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::models::{
    AnomalyResult, AnomalyType, ClusteredDocument,
    ClusteringJob, ClusteringRequest, ClusteringResult, DocumentCluster,
    DocumentSimilarity, DocumentTopics, JobStatus, SimilarDocumentsRequest, Topic, TopicAssignment, TopicKeyword, TopicModelingRequest, TrendAnalysis, TrendingKeyword, TrendingTopic,
};

/// Raw DB row type for document cluster queries
type DocumentClusterRow = (
    i32,
    String,
    Option<String>,
    Vec<f32>,
    i64,
    Vec<String>,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// ML Analytics service
pub struct MlService {
    db: PgPool,
}

impl MlService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Start document clustering job
    pub async fn start_clustering(&self, request: ClusteringRequest) -> Result<ClusteringJob> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let algorithm = request.algorithm.unwrap_or_default();
        let num_clusters = request.num_clusters.unwrap_or(5);

        Ok(ClusteringJob {
            id: job_id,
            status: JobStatus::Pending,
            algorithm,
            num_clusters,
            documents_processed: 0,
            progress_percent: 0,
            created_at: Utc::now(),
            completed_at: None,
        })
    }

    /// Get clustering results
    pub async fn get_clustering_result(&self, _job_id: &str) -> Result<Option<ClusteringResult>> {
        // TODO: Implement actual clustering with embeddings
        Ok(None)
    }

    /// Get document clusters
    pub async fn get_clusters(&self) -> Result<Vec<DocumentCluster>> {
        let rows: Vec<DocumentClusterRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, centroid, document_count, keywords, created_at, updated_at
            FROM document_clusters
            ORDER BY document_count DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DocumentCluster {
                id: r.0,
                name: r.1,
                description: r.2,
                centroid: r.3,
                document_count: r.4,
                keywords: r.5,
                created_at: r.6,
                updated_at: r.7,
            })
            .collect())
    }

    /// Get documents in cluster
    pub async fn get_cluster_documents(&self, cluster_id: i32) -> Result<Vec<ClusteredDocument>> {
        let rows: Vec<(uuid::Uuid, String, i32, String, f32)> = sqlx::query_as(
            r#"
            SELECT d.id, d.title, dc.cluster_id, c.name, dc.similarity_score
            FROM documents d
            JOIN document_cluster_assignments dc ON d.id = dc.document_id
            JOIN document_clusters c ON dc.cluster_id = c.id
            WHERE dc.cluster_id = $1
            ORDER BY dc.similarity_score DESC
            "#,
        )
        .bind(cluster_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ClusteredDocument {
                document_id: r.0,
                title: r.1,
                cluster_id: r.2,
                cluster_name: r.3,
                similarity_score: r.4,
            })
            .collect())
    }

    /// Find similar documents
    pub async fn find_similar_documents(
        &self,
        request: SimilarDocumentsRequest,
    ) -> Result<Vec<DocumentSimilarity>> {
        let limit = request.limit.unwrap_or(10).min(50);
        let min_similarity = request.min_similarity.unwrap_or(0.5);

        // Get document embedding
        let doc_embedding: Option<(Vec<f32>,)> = sqlx::query_as(
            "SELECT embedding FROM document_embeddings WHERE document_id = $1",
        )
        .bind(request.document_id)
        .fetch_optional(&self.db)
        .await?;

        let embedding = match doc_embedding {
            Some((e,)) => e,
            None => return Ok(vec![]),
        };

        // Find similar documents using cosine similarity
        let rows: Vec<(uuid::Uuid, String, f32)> = sqlx::query_as(
            r#"
            SELECT d.id, d.title,
                   1 - (de.embedding <=> $1::vector) as similarity
            FROM documents d
            JOIN document_embeddings de ON d.id = de.document_id
            WHERE d.id != $2
              AND 1 - (de.embedding <=> $1::vector) >= $3
            ORDER BY similarity DESC
            LIMIT $4
            "#,
        )
        .bind(&embedding)
        .bind(request.document_id)
        .bind(min_similarity)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| DocumentSimilarity {
                document_id: r.0,
                title: r.1,
                similarity_score: r.2,
                shared_keywords: vec![], // TODO: Calculate shared keywords
            })
            .collect())
    }

    /// Start topic modeling
    pub async fn start_topic_modeling(&self, _request: TopicModelingRequest) -> Result<String> {
        let job_id = uuid::Uuid::new_v4().to_string();
        // TODO: Queue topic modeling job
        Ok(job_id)
    }

    /// Get topics
    pub async fn get_topics(&self) -> Result<Vec<Topic>> {
        let rows: Vec<(i32, String, serde_json::Value, i64, f32)> = sqlx::query_as(
            r#"
            SELECT id, name, keywords, document_count, coherence_score
            FROM topics
            ORDER BY document_count DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let keywords: Vec<TopicKeyword> =
                    serde_json::from_value(r.2).unwrap_or_default();

                Topic {
                    id: r.0,
                    name: r.1,
                    keywords,
                    document_count: r.3,
                    coherence_score: r.4,
                }
            })
            .collect())
    }

    /// Get document topics
    pub async fn get_document_topics(&self, document_id: uuid::Uuid) -> Result<DocumentTopics> {
        let title: (String,) = sqlx::query_as("SELECT title FROM documents WHERE id = $1")
            .bind(document_id)
            .fetch_one(&self.db)
            .await?;

        let rows: Vec<(i32, String, f32)> = sqlx::query_as(
            r#"
            SELECT t.id, t.name, dt.weight
            FROM document_topics dt
            JOIN topics t ON dt.topic_id = t.id
            WHERE dt.document_id = $1
            ORDER BY dt.weight DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await?;

        let topics: Vec<TopicAssignment> = rows
            .into_iter()
            .map(|r| TopicAssignment {
                topic_id: r.0,
                topic_name: r.1,
                weight: r.2,
            })
            .collect();

        Ok(DocumentTopics {
            document_id,
            title: title.0,
            topics,
        })
    }

    /// Get trend analysis
    pub async fn get_trend_analysis(&self, days: i32) -> Result<TrendAnalysis> {
        let period_start = Utc::now() - chrono::Duration::days(days as i64);
        let period_end = Utc::now();

        // Get trending topics
        let trending_topics: Vec<TrendingTopic> = vec![]; // TODO: Calculate from data

        // Get trending keywords
        let trending_keywords: Vec<TrendingKeyword> = vec![]; // TODO: Extract from documents

        // Get document volume time series
        let rows: Vec<(chrono::DateTime<chrono::Utc>, i64)> = sqlx::query_as(
            r#"
            SELECT DATE_TRUNC('day', created_at) as day, COUNT(*)::bigint
            FROM documents
            WHERE created_at >= $1
            GROUP BY day
            ORDER BY day
            "#,
        )
        .bind(period_start)
        .fetch_all(&self.db)
        .await?;

        let document_volume: Vec<crate::models::TimeSeriesPoint> = rows
            .into_iter()
            .map(|r| crate::models::TimeSeriesPoint {
                timestamp: r.0,
                value: r.1,
            })
            .collect();

        Ok(TrendAnalysis {
            period_start,
            period_end,
            trending_topics,
            trending_keywords,
            document_volume,
        })
    }

    /// Detect anomalies in documents
    pub async fn detect_anomalies(&self, category_id: Option<i32>) -> Result<Vec<AnomalyResult>> {
        // Simple anomaly detection based on document length
        let rows: Vec<(uuid::Uuid, String, i64)> = sqlx::query_as(
            r#"
            SELECT d.id, d.title, LENGTH(d.content) as content_length
            FROM documents d
            WHERE ($1::int IS NULL OR d.category_id = $1)
            ORDER BY content_length DESC
            LIMIT 10
            "#,
        )
        .bind(category_id)
        .fetch_all(&self.db)
        .await?;

        // Calculate mean and std for anomaly detection
        let lengths: Vec<i64> = rows.iter().map(|r| r.2).collect();
        let mean = lengths.iter().sum::<i64>() as f32 / lengths.len().max(1) as f32;
        let variance = lengths
            .iter()
            .map(|l| (*l as f32 - mean).powi(2))
            .sum::<f32>()
            / lengths.len().max(1) as f32;
        let std = variance.sqrt();

        let anomalies: Vec<AnomalyResult> = rows
            .into_iter()
            .filter_map(|r| {
                let z_score = (r.2 as f32 - mean) / std.max(1.0);
                if z_score.abs() > 2.0 {
                    Some(AnomalyResult {
                        document_id: r.0,
                        title: r.1,
                        anomaly_score: z_score.abs(),
                        anomaly_type: if r.2 as f32 > mean {
                            AnomalyType::LengthAnomaly
                        } else {
                            AnomalyType::ContentOutlier
                        },
                        explanation: format!(
                            "Document length ({}) deviates significantly from average ({})",
                            r.2, mean as i64
                        ),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(anomalies)
    }
}

/// Pure statistical helper functions (testable without DB)
/// Compute the mean of a slice of i64 values.
/// Returns 0.0 for an empty slice.
pub fn compute_mean(values: &[i64]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<i64>() as f32 / values.len() as f32
}

/// Compute the population standard deviation of a slice of i64 values.
/// Returns 0.0 for an empty or single-element slice.
pub fn compute_std(values: &[i64]) -> f32 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = compute_mean(values);
    let variance = values
        .iter()
        .map(|v| (*v as f32 - mean).powi(2))
        .sum::<f32>()
        / values.len() as f32;
    variance.sqrt()
}

/// Compute the z-score for a single value given mean and std.
/// If std is 0, returns 0 to avoid division by zero.
pub fn compute_z_score(value: i64, mean: f32, std: f32) -> f32 {
    let denominator = std.max(1.0);
    (value as f32 - mean) / denominator
}

/// Determine if a z-score represents an anomaly (|z| > threshold).
pub fn is_anomaly(z_score: f32, threshold: f32) -> bool {
    z_score.abs() > threshold
}

/// Clamp a similarity score to [0.0, 1.0].
pub fn clamp_similarity(score: f32) -> f32 {
    score.clamp(0.0, 1.0)
}

/// Limit the result count to at most `max`.
pub fn clamp_result_limit(limit: i32, max: i32) -> i32 {
    limit.min(max).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_mean_empty() {
        assert_eq!(compute_mean(&[]), 0.0);
    }

    #[test]
    fn test_compute_mean_single() {
        assert!((compute_mean(&[10]) - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_mean_multiple() {
        let values = [2, 4, 6, 8];
        assert!((compute_mean(&values) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_mean_negative_values() {
        let values = [-4, -2, 0, 2, 4];
        assert!((compute_mean(&values) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_std_empty() {
        assert_eq!(compute_std(&[]), 0.0);
    }

    #[test]
    fn test_compute_std_single() {
        assert_eq!(compute_std(&[42]), 0.0);
    }

    #[test]
    fn test_compute_std_identical_values() {
        let values = [5, 5, 5, 5];
        assert!((compute_std(&values) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_std_known_values() {
        // [2, 4, 4, 4, 5, 5, 7, 9] -> mean=5, variance=4, std=2
        let values = [2, 4, 4, 4, 5, 5, 7, 9];
        assert!((compute_std(&values) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_z_score_at_mean() {
        assert!((compute_z_score(5, 5.0, 2.0) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_z_score_one_std_above() {
        assert!((compute_z_score(7, 5.0, 2.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_compute_z_score_zero_std_does_not_panic() {
        let z = compute_z_score(10, 5.0, 0.0);
        // std clamped to 1.0, so z = (10 - 5) / 1 = 5
        assert!((z - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_is_anomaly_above_threshold() {
        assert!(is_anomaly(2.5, 2.0));
    }

    #[test]
    fn test_is_anomaly_at_threshold_not_anomaly() {
        assert!(!is_anomaly(2.0, 2.0));
    }

    #[test]
    fn test_is_anomaly_negative_z_score() {
        assert!(is_anomaly(-3.0, 2.0));
    }

    #[test]
    fn test_clamp_similarity_normal() {
        assert!((clamp_similarity(0.75) - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_clamp_similarity_below_zero() {
        assert!((clamp_similarity(-0.5) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_clamp_similarity_above_one() {
        assert!((clamp_similarity(1.5) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_clamp_result_limit_normal() {
        assert_eq!(clamp_result_limit(10, 50), 10);
    }

    #[test]
    fn test_clamp_result_limit_above_max() {
        assert_eq!(clamp_result_limit(100, 50), 50);
    }

    #[test]
    fn test_clamp_result_limit_zero_becomes_one() {
        assert_eq!(clamp_result_limit(0, 50), 1);
    }
}
