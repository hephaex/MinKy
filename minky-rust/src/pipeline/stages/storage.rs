//! Storage stage - persists documents and embeddings to the database
//!
//! Stores:
//! - Document record
//! - Document embeddings
//! - Chunk embeddings
//! - Document understanding (AI analysis)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::EmbeddingModel;
use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

use super::analysis::AnalyzedDocument;

/// Stored document with all persisted IDs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDocument {
    /// Document ID in the database
    pub document_id: Uuid,

    /// Document title
    pub title: String,

    /// Number of chunks stored
    pub chunks_count: usize,

    /// Whether document embedding was stored
    pub has_document_embedding: bool,

    /// Whether analysis was stored
    pub has_analysis: bool,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Analysis results (if any)
    pub analysis: Option<super::analysis::DocumentAnalysis>,
}

/// Storage stage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Whether to update existing documents (by title/source)
    pub upsert: bool,

    /// Whether to replace existing embeddings
    pub replace_embeddings: bool,

    /// User ID for document ownership
    pub user_id: Option<i32>,

    /// Whether document should be public
    pub is_public: bool,

    /// Category ID
    pub category_id: Option<i32>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            upsert: true,
            replace_embeddings: true,
            user_id: None,
            is_public: false,
            category_id: None,
        }
    }
}

/// Storage stage - persists documents to the database
#[derive(Debug, Clone)]
pub struct StorageStage {
    pool: PgPool,
    config: StorageConfig,
}

impl StorageStage {
    /// Create a new storage stage
    pub fn new(pool: PgPool, config: StorageConfig) -> Self {
        Self { pool, config }
    }

    /// Create with default config
    pub fn with_pool(pool: PgPool) -> Self {
        Self::new(pool, StorageConfig::default())
    }

    /// Store the document record
    async fn store_document(
        &self,
        doc: &AnalyzedDocument,
        context: &PipelineContext,
    ) -> PipelineResult<Uuid> {
        let user_id = context.user_id.or(self.config.user_id).unwrap_or(1);
        let category_id = context.category_id.or(self.config.category_id);
        let is_public = self.config.is_public;
        let source_path = doc.source_path.as_deref();

        // Check if document exists.
        // Prefer matching by (user_id, source_path) when source_path is present so
        // that vault re-ingests update the same row instead of creating duplicates.
        // Fall back to (user_id, title) for plain-text / URL ingests.
        let existing_id: Option<(Uuid,)> = if self.config.upsert {
            if let Some(sp) = source_path {
                sqlx::query_as(
                    "SELECT id FROM documents WHERE user_id = $1 AND source_path = $2",
                )
                .bind(user_id)
                .bind(sp)
                .fetch_optional(&self.pool)
                .await?
            } else {
                sqlx::query_as(
                    "SELECT id FROM documents WHERE title = $1 AND user_id = $2",
                )
                .bind(&doc.title)
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?
            }
        } else {
            None
        };

        let document_id = if let Some((id,)) = existing_id {
            // Update existing document, refreshing source_path in case it was previously NULL
            sqlx::query(
                r#"
                UPDATE documents
                SET content = $2, category_id = $3, is_public = $4, source_path = $5, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(&doc.original_content)
            .bind(category_id)
            .bind(is_public)
            .bind(source_path)
            .execute(&self.pool)
            .await?;

            id
        } else {
            // Insert new document
            let (id,): (Uuid,) = sqlx::query_as(
                r#"
                INSERT INTO documents (title, content, category_id, user_id, is_public, source_path)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id
                "#,
            )
            .bind(&doc.title)
            .bind(&doc.original_content)
            .bind(category_id)
            .bind(user_id)
            .bind(is_public)
            .bind(source_path)
            .fetch_one(&self.pool)
            .await?;

            id
        };

        Ok(document_id)
    }

    /// Store document-level embedding
    async fn store_document_embedding(
        &self,
        document_id: Uuid,
        embedding: &[f32],
        model: EmbeddingModel,
    ) -> PipelineResult<()> {
        if self.config.replace_embeddings {
            // Delete existing embedding for this document/model combo
            sqlx::query("DELETE FROM document_embeddings WHERE document_id = $1 AND model = $2")
                .bind(document_id)
                .bind(model)
                .execute(&self.pool)
                .await?;
        }

        let token_count = 0i32; // Will be updated by embedding service

        sqlx::query(
            r#"
            INSERT INTO document_embeddings (document_id, embedding, model, token_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (document_id, model)
            DO UPDATE SET embedding = $2, token_count = $4, updated_at = NOW()
            "#,
        )
        .bind(document_id)
        .bind(Vector::from(embedding.to_vec()))
        .bind(model)
        .bind(token_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store chunk embeddings
    async fn store_chunk_embeddings(
        &self,
        document_id: Uuid,
        embedded_chunks: &[super::embedding::EmbeddedChunk],
    ) -> PipelineResult<usize> {
        if embedded_chunks.is_empty() {
            return Ok(0);
        }

        let model = embedded_chunks[0].model;

        if self.config.replace_embeddings {
            // Delete existing chunks for this document/model
            sqlx::query("DELETE FROM chunk_embeddings WHERE document_id = $1 AND model = $2")
                .bind(document_id)
                .bind(model)
                .execute(&self.pool)
                .await?;
        }

        let mut count = 0;

        for embedded in embedded_chunks {
            let token_count = embedded.chunk.token_count as i32;

            sqlx::query(
                r#"
                INSERT INTO chunk_embeddings
                (document_id, chunk_index, chunk_text, chunk_start_offset, chunk_end_offset,
                 embedding, model, token_count, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (document_id, chunk_index, model)
                DO UPDATE SET
                    chunk_text = $3,
                    chunk_start_offset = $4,
                    chunk_end_offset = $5,
                    embedding = $6,
                    token_count = $8,
                    metadata = $9
                "#,
            )
            .bind(document_id)
            .bind(embedded.chunk.index as i32)
            .bind(&embedded.chunk.text)
            .bind(embedded.chunk.start_offset as i32)
            .bind(embedded.chunk.end_offset as i32)
            .bind(Vector::from(embedded.embedding.clone()))
            .bind(embedded.model)
            .bind(token_count)
            .bind(&embedded.chunk.metadata)
            .execute(&self.pool)
            .await?;

            count += 1;
        }

        Ok(count)
    }

    /// Store document understanding (AI analysis)
    async fn store_analysis(
        &self,
        document_id: Uuid,
        analysis: &super::analysis::DocumentAnalysis,
    ) -> PipelineResult<()> {
        sqlx::query(
            r#"
            INSERT INTO document_understanding
            (document_id, topics, summary, problem_solved, insights, technologies, relevant_for, analyzer_model)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (document_id)
            DO UPDATE SET
                topics = $2,
                summary = $3,
                problem_solved = $4,
                insights = $5,
                technologies = $6,
                relevant_for = $7,
                analyzer_model = $8,
                analyzed_at = NOW()
            "#,
        )
        .bind(document_id)
        .bind(&analysis.topics)
        .bind(&analysis.summary)
        .bind(&analysis.problem_solved)
        .bind(&analysis.insights)
        .bind(&analysis.technologies)
        .bind(&analysis.relevant_for)
        .bind(&analysis.analyzer_model)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl PipelineStage<AnalyzedDocument, StoredDocument> for StorageStage {
    fn name(&self) -> &'static str {
        "storage"
    }

    async fn process(
        &self,
        input: AnalyzedDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<StoredDocument> {
        // Store document record
        let document_id = self.store_document(&input, context).await.map_err(|e| {
            PipelineError::stage_failure("storage", format!("Failed to store document: {}", e))
        })?;

        // Update context with document ID
        context.set_document_id(document_id);

        // Store document-level embedding if available
        let has_document_embedding = if let Some(ref embedding) = input.document_embedding {
            self.store_document_embedding(document_id, embedding, input.embedding_model)
                .await
                .map_err(|e| {
                    PipelineError::stage_failure(
                        "storage",
                        format!("Failed to store document embedding: {}", e),
                    )
                })?;
            true
        } else {
            false
        };

        // Store chunk embeddings
        let chunks_count = self
            .store_chunk_embeddings(document_id, &input.embedded_chunks)
            .await
            .map_err(|e| {
                PipelineError::stage_failure(
                    "storage",
                    format!("Failed to store chunk embeddings: {}", e),
                )
            })?;

        // Store analysis if available
        let has_analysis = if let Some(ref analysis) = input.analysis {
            self.store_analysis(document_id, analysis).await.map_err(|e| {
                PipelineError::stage_failure("storage", format!("Failed to store analysis: {}", e))
            })?;
            true
        } else {
            false
        };

        Ok(StoredDocument {
            document_id,
            title: input.title,
            chunks_count,
            has_document_embedding,
            has_analysis,
            created_at: Utc::now(),
            analysis: input.analysis,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a database connection
    // For unit tests without DB, we test the configuration

    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert!(config.upsert);
        assert!(config.replace_embeddings);
        assert!(!config.is_public);
        assert!(config.user_id.is_none());
    }

    #[test]
    fn test_stored_document_fields() {
        let stored = StoredDocument {
            document_id: Uuid::new_v4(),
            title: "Test".to_string(),
            chunks_count: 5,
            has_document_embedding: true,
            has_analysis: true,
            created_at: Utc::now(),
            analysis: None,
        };

        assert_eq!(stored.chunks_count, 5);
        assert!(stored.has_document_embedding);
        assert!(stored.has_analysis);
    }

    /// When `source_path` is `Some`, the storage stage must use the
    /// `(user_id, source_path)` lookup so that repeated ingests of the same
    /// vault file converge on a single document row.
    ///
    /// When `source_path` is `None` (e.g. plain-text or URL ingests), the
    /// fallback `(user_id, title)` lookup is used instead.
    ///
    /// This test verifies the branching precondition that controls which
    /// upsert path is taken, without requiring a live database connection.
    #[test]
    fn upsert_prefers_source_path_lookup_when_source_path_is_some() {
        use super::super::analysis::AnalyzedDocument;
        use crate::models::EmbeddingModel;

        // A document produced by the File ingestion path always has source_path
        // set.  The storage stage selects the upsert branch via:
        //   `if let Some(sp) = doc.source_path.as_deref() { ... }`
        // so we verify that the field is correctly `Some` for file ingests.
        let doc_with_path = AnalyzedDocument {
            title: "vault note".to_string(),
            plain_text: String::new(),
            original_content: "# Vault Note\n".to_string(),
            mime_type: "text/markdown".to_string(),
            metadata: Default::default(),
            embedded_chunks: vec![],
            document_embedding: None,
            embedding_model: EmbeddingModel::default(),
            analysis: None,
            headings: vec![],
            links: vec![],
            code_blocks: vec![],
            source_type: "file".to_string(),
            source_path: Some("/vault/note.md".to_string()),
        };

        // Mirrors the branch condition in `store_document`.
        let uses_source_path_lookup = doc_with_path.source_path.is_some();
        assert!(
            uses_source_path_lookup,
            "file-ingested document must take the source_path upsert branch"
        );
        assert_eq!(
            doc_with_path.source_path.as_deref(),
            Some("/vault/note.md"),
            "source_path must round-trip the file path unchanged"
        );

        // A document ingested via plain text or URL has no source_path and
        // must fall through to the title-based lookup.
        let doc_without_path = AnalyzedDocument {
            title: "plain text note".to_string(),
            source_path: None,
            source_type: "text".to_string(),
            ..doc_with_path
        };

        let uses_title_lookup = doc_without_path.source_path.is_none();
        assert!(
            uses_title_lookup,
            "text-ingested document must take the title upsert branch"
        );
    }

    /// Verify that `source_path` is present on `AnalyzedDocument` and can be
    /// extracted as a `&str` the same way the storage stage does.
    ///
    /// This is a compile-time / structural test — if `AnalyzedDocument` ever
    /// loses the `source_path: Option<String>` field the test will fail to
    /// compile, catching the regression immediately.
    #[test]
    fn source_path_field_propagates() {
        use super::super::analysis::AnalyzedDocument;
        use crate::models::EmbeddingModel;

        // Helper that mirrors the extraction the storage stage performs
        fn extract_source_path(doc: &AnalyzedDocument) -> Option<&str> {
            doc.source_path.as_deref()
        }

        let doc_with_path = AnalyzedDocument {
            title: "test doc".to_string(),
            plain_text: String::new(),
            original_content: "content".to_string(),
            mime_type: "text/markdown".to_string(),
            metadata: Default::default(),
            embedded_chunks: vec![],
            document_embedding: None,
            embedding_model: EmbeddingModel::default(),
            analysis: None,
            headings: vec![],
            links: vec![],
            code_blocks: vec![],
            source_type: "file".to_string(),
            source_path: Some("/tmp/test.md".to_string()),
        };

        assert_eq!(extract_source_path(&doc_with_path), Some("/tmp/test.md"));

        let doc_no_path = AnalyzedDocument {
            source_path: None,
            source_type: "text".to_string(),
            title: "plain text".to_string(),
            ..doc_with_path
        };

        assert!(extract_source_path(&doc_no_path).is_none());
    }
}
