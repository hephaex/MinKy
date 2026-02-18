use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::models::{
    AnomalyResult, AnomalyType, ClusterAssignment, ClusteredDocument, ClusteringAlgorithm,
    ClusteringJob, ClusteringMetrics, ClusteringRequest, ClusteringResult, DocumentCluster,
    DocumentSimilarity, DocumentTopics, JobStatus, SimilarDocumentsRequest, Topic,
    TopicAlgorithm, TopicAssignment, TopicKeyword, TopicModelingRequest, TrendAnalysis,
    TrendDirection, TrendingKeyword, TrendingTopic,
};

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
        let rows: Vec<(
            i32,
            String,
            Option<String>,
            Vec<f32>,
            i64,
            Vec<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
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
    pub async fn start_topic_modeling(&self, request: TopicModelingRequest) -> Result<String> {
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
