//! Indexing stage - updates search indexes and creates document links
//!
//! Handles:
//! - Full-text search indexing
//! - Related document linking
//! - Tag creation/association

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

use super::storage::StoredDocument;
use super::PipelineOutput;

/// Configuration for indexing
#[derive(Debug, Clone)]
pub struct IndexingConfig {
    /// Whether to find and link related documents
    pub link_related: bool,

    /// Maximum number of related documents to link
    pub max_related: usize,

    /// Minimum similarity threshold for related documents
    pub similarity_threshold: f32,

    /// Whether to auto-create tags from analysis
    pub auto_tag: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            link_related: true,
            max_related: 5,
            similarity_threshold: 0.7,
            auto_tag: true,
        }
    }
}

/// Indexing stage - creates search indexes and links
#[derive(Debug, Clone)]
pub struct IndexingStage {
    pool: PgPool,
    config: IndexingConfig,
}

impl IndexingStage {
    /// Create a new indexing stage
    pub fn new(pool: PgPool, config: IndexingConfig) -> Self {
        Self { pool, config }
    }

    /// Create with default config
    pub fn with_pool(pool: PgPool) -> Self {
        Self::new(pool, IndexingConfig::default())
    }

    /// Find and link related documents based on embedding similarity
    async fn link_related_documents(&self, document_id: Uuid) -> PipelineResult<Vec<Uuid>> {
        // Find similar documents using vector similarity
        let related: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            WITH source_embedding AS (
                SELECT embedding, model FROM document_embeddings WHERE document_id = $1 LIMIT 1
            )
            SELECT de.document_id
            FROM document_embeddings de
            CROSS JOIN source_embedding se
            WHERE de.document_id != $1
              AND de.model = se.model
              AND de.embedding IS NOT NULL
              AND 1 - (de.embedding <=> se.embedding) >= $2
            ORDER BY de.embedding <=> se.embedding
            LIMIT $3
            "#,
        )
        .bind(document_id)
        .bind(self.config.similarity_threshold)
        .bind(self.config.max_related as i32)
        .fetch_all(&self.pool)
        .await?;

        let related_ids: Vec<Uuid> = related.into_iter().map(|(id,)| id).collect();

        // Update the document_understanding with related IDs
        if !related_ids.is_empty() {
            sqlx::query(
                r#"
                UPDATE document_understanding
                SET related_document_ids = $2
                WHERE document_id = $1
                "#,
            )
            .bind(document_id)
            .bind(&related_ids)
            .execute(&self.pool)
            .await?;
        }

        Ok(related_ids)
    }

    /// Create or find tags and associate with document
    async fn create_tags(&self, document_id: Uuid, topics: &[String]) -> PipelineResult<()> {
        for topic in topics {
            // Normalize topic as tag name (lowercase, trim)
            let tag_name = topic.to_lowercase().trim().to_string();
            if tag_name.is_empty() || tag_name.len() > 50 {
                continue;
            }

            // Find or create tag
            let tag_result: Option<(i32,)> =
                sqlx::query_as("SELECT id FROM tags WHERE name = $1")
                    .bind(&tag_name)
                    .fetch_optional(&self.pool)
                    .await?;

            let tag_id = if let Some((id,)) = tag_result {
                id
            } else {
                // Create new tag
                let (id,): (i32,) =
                    sqlx::query_as("INSERT INTO tags (name) VALUES ($1) RETURNING id")
                        .bind(&tag_name)
                        .fetch_one(&self.pool)
                        .await?;
                id
            };

            // Associate tag with document (ignore if already exists)
            sqlx::query(
                "INSERT INTO document_tags (document_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(document_id)
            .bind(tag_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Update pipeline job status
    async fn update_job_status(&self, job_id: Uuid, document_id: Uuid) -> PipelineResult<()> {
        sqlx::query(
            r#"
            UPDATE pipeline_jobs
            SET status = 'completed', document_id = $2, completed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(job_id)
        .bind(document_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl PipelineStage<StoredDocument, PipelineOutput> for IndexingStage {
    fn name(&self) -> &'static str {
        "indexing"
    }

    async fn process(
        &self,
        input: StoredDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<PipelineOutput> {
        let document_id = input.document_id;

        // Link related documents
        let related_count = if self.config.link_related && input.has_document_embedding {
            let related = self.link_related_documents(document_id).await.map_err(|e| {
                PipelineError::stage_failure(
                    "indexing",
                    format!("Failed to link related documents: {}", e),
                )
            })?;
            related.len()
        } else {
            0
        };

        // Create tags from topics
        let topics: Vec<String> = if self.config.auto_tag {
            if let Some(ref analysis) = input.analysis {
                // Create tags from topics
                self.create_tags(document_id, &analysis.topics)
                    .await
                    .map_err(|e| {
                        PipelineError::stage_failure("indexing", format!("Failed to create tags: {}", e))
                    })?;
                analysis.topics.clone()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Update pipeline job if tracking
        self.update_job_status(context.job_id, document_id)
            .await
            .ok(); // Ignore errors - job table might not exist

        context.set_metadata("related_documents_count", related_count);

        let summary = input
            .analysis
            .as_ref()
            .map(|a| a.summary.clone());

        Ok(PipelineOutput {
            document_id,
            title: input.title,
            chunks_count: input.chunks_count,
            analyzed: input.has_analysis,
            topics,
            summary,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexing_config_defaults() {
        let config = IndexingConfig::default();
        assert!(config.link_related);
        assert_eq!(config.max_related, 5);
        assert!((config.similarity_threshold - 0.7).abs() < 0.001);
        assert!(config.auto_tag);
    }

    #[test]
    fn test_pipeline_output() {
        let output = PipelineOutput {
            document_id: Uuid::new_v4(),
            title: "Test Doc".to_string(),
            chunks_count: 3,
            analyzed: true,
            topics: vec!["rust".to_string(), "programming".to_string()],
            summary: Some("A test document about Rust.".to_string()),
        };

        assert_eq!(output.chunks_count, 3);
        assert!(output.analyzed);
        assert_eq!(output.topics.len(), 2);
    }
}
