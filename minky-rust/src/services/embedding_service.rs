//! Embedding service for vector search and RAG
//!
//! This service handles:
//! - Generating embeddings via OpenAI/Voyage API
//! - Storing embeddings in pgvector
//! - Semantic similarity search
//! - Document understanding with Claude

use crate::error::{AppError, Result};
use crate::models::{
    ChunkData, ChunkEmbedding, CreateChunkEmbeddingsRequest, CreateDocumentEmbeddingRequest,
    DocumentEmbedding, DocumentUnderstanding,
    DocumentUnderstandingResponse, EmbeddingModel, EmbeddingQueueEntry, EmbeddingStats,
    SemanticSearchRequest, SemanticSearchResult,
};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Configuration for the embedding service
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub openai_api_key: Option<String>,
    pub voyage_api_key: Option<String>,
    pub default_model: EmbeddingModel,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            voyage_api_key: None,
            default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
            chunk_size: 512,
            chunk_overlap: 50,
        }
    }
}

/// Embedding service for vector operations
pub struct EmbeddingService {
    pool: PgPool,
    config: EmbeddingConfig,
    http_client: reqwest::Client,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(pool: PgPool, config: EmbeddingConfig) -> Self {
        Self {
            pool,
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generate embedding for text using OpenAI API
    pub async fn generate_embedding(
        &self,
        text: &str,
        model: EmbeddingModel,
    ) -> Result<Vec<f32>> {
        let api_key = self
            .config
            .openai_api_key
            .as_ref()
            .ok_or_else(|| AppError::Configuration("OpenAI API key not configured".into()))?;

        let request = OpenAIEmbeddingRequest {
            input: text.to_string(),
            model: model.api_model_id().to_string(),
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse response: {}", e)))?;

        response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| AppError::ExternalService("No embedding returned".into()))
    }

    /// Generate embeddings for multiple texts (batch)
    pub async fn generate_embeddings_batch(
        &self,
        texts: &[String],
        model: EmbeddingModel,
    ) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        // Process in batches of 100 (OpenAI limit)
        for chunk in texts.chunks(100) {
            for text in chunk {
                let embedding = self.generate_embedding(text, model).await?;
                embeddings.push(embedding);
            }
        }

        Ok(embeddings)
    }

    /// Create document embedding
    pub async fn create_document_embedding(
        &self,
        req: CreateDocumentEmbeddingRequest,
    ) -> Result<DocumentEmbedding> {
        let model = req.model.unwrap_or(self.config.default_model);

        // Get document content
        let document: (String, String) = sqlx::query_as(
            "SELECT title, content FROM documents WHERE id = $1",
        )
        .bind(req.document_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::NotFound("Document not found".into()))?;

        let text = format!("{}\n\n{}", document.0, document.1);

        // Generate embedding
        let embedding = self.generate_embedding(&text, model).await?;
        let token_count = text.split_whitespace().count() as i32; // Approximate

        // Insert or update
        let result = sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            INSERT INTO document_embeddings (document_id, embedding, model, token_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (document_id, model)
            DO UPDATE SET embedding = $2, token_count = $4, updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(req.document_id)
        .bind(Vector::from(embedding))
        .bind(model)
        .bind(token_count)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Create chunk embeddings for RAG
    pub async fn create_chunk_embeddings(
        &self,
        req: CreateChunkEmbeddingsRequest,
    ) -> Result<Vec<ChunkEmbedding>> {
        let model = req.model.unwrap_or(self.config.default_model);

        // Generate embeddings for all chunks
        let texts: Vec<String> = req.chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = self.generate_embeddings_batch(&texts, model).await?;

        let mut results = Vec::new();

        for (idx, (chunk, embedding)) in req.chunks.iter().zip(embeddings.iter()).enumerate() {
            let token_count = chunk.text.split_whitespace().count() as i32;

            let result = sqlx::query_as::<_, ChunkEmbedding>(
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
                RETURNING *
                "#,
            )
            .bind(req.document_id)
            .bind(idx as i32)
            .bind(&chunk.text)
            .bind(chunk.start_offset)
            .bind(chunk.end_offset)
            .bind(Vector::from(embedding.clone()))
            .bind(model)
            .bind(token_count)
            .bind(&chunk.metadata)
            .fetch_one(&self.pool)
            .await?;

            results.push(result);
        }

        Ok(results)
    }

    /// Semantic search using vector similarity
    pub async fn semantic_search(
        &self,
        req: SemanticSearchRequest,
    ) -> Result<Vec<SemanticSearchResult>> {
        let model = req.model.unwrap_or(self.config.default_model);

        // Generate query embedding
        let query_embedding = self.generate_embedding(&req.query, model).await?;
        let threshold = req.threshold.unwrap_or(0.7);

        // Search in chunk embeddings with cosine similarity
        let results = sqlx::query_as::<_, SemanticSearchRow>(
            r#"
            SELECT
                ce.document_id,
                ce.id as chunk_id,
                ce.chunk_text,
                1 - (ce.embedding <=> $1) as similarity,
                d.title as document_title
            FROM chunk_embeddings ce
            JOIN documents d ON d.id = ce.document_id
            WHERE ce.model = $2
              AND ce.embedding IS NOT NULL
              AND ($4::integer IS NULL OR d.user_id = $4)
              AND 1 - (ce.embedding <=> $1) >= $3
            ORDER BY ce.embedding <=> $1
            LIMIT $5
            "#,
        )
        .bind(Vector::from(query_embedding))
        .bind(model)
        .bind(threshold)
        .bind(req.user_id)
        .bind(req.limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| SemanticSearchResult {
                document_id: r.document_id,
                chunk_id: r.chunk_id,
                chunk_text: r.chunk_text,
                similarity: r.similarity,
                document_title: r.document_title,
            })
            .collect())
    }

    /// Find similar documents
    pub async fn find_similar_documents(
        &self,
        document_id: Uuid,
        limit: i32,
    ) -> Result<Vec<SemanticSearchResult>> {
        let results = sqlx::query_as::<_, SemanticSearchRow>(
            r#"
            WITH source_embedding AS (
                SELECT embedding, model FROM document_embeddings WHERE document_id = $1 LIMIT 1
            )
            SELECT
                de.document_id,
                NULL::uuid as chunk_id,
                NULL as chunk_text,
                1 - (de.embedding <=> se.embedding) as similarity,
                d.title as document_title
            FROM document_embeddings de
            CROSS JOIN source_embedding se
            JOIN documents d ON d.id = de.document_id
            WHERE de.document_id != $1
              AND de.model = se.model
              AND de.embedding IS NOT NULL
            ORDER BY de.embedding <=> se.embedding
            LIMIT $2
            "#,
        )
        .bind(document_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|r| SemanticSearchResult {
                document_id: r.document_id,
                chunk_id: r.chunk_id,
                chunk_text: r.chunk_text,
                similarity: r.similarity,
                document_title: r.document_title,
            })
            .collect())
    }

    /// Get document understanding
    pub async fn get_document_understanding(
        &self,
        document_id: Uuid,
    ) -> Result<Option<DocumentUnderstanding>> {
        let result = sqlx::query_as::<_, DocumentUnderstanding>(
            "SELECT * FROM document_understanding WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Save document understanding
    pub async fn save_document_understanding(
        &self,
        document_id: Uuid,
        understanding: DocumentUnderstandingResponse,
    ) -> Result<DocumentUnderstanding> {
        let result = sqlx::query_as::<_, DocumentUnderstanding>(
            r#"
            INSERT INTO document_understanding
            (document_id, topics, summary, problem_solved, insights, technologies, relevant_for)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (document_id)
            DO UPDATE SET
                topics = $2,
                summary = $3,
                problem_solved = $4,
                insights = $5,
                technologies = $6,
                relevant_for = $7,
                analyzed_at = NOW()
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(&understanding.topics)
        .bind(&understanding.summary)
        .bind(&understanding.problem_solved)
        .bind(&understanding.insights)
        .bind(&understanding.technologies)
        .bind(&understanding.relevant_for)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Add document to embedding queue
    pub async fn queue_document(&self, document_id: Uuid, priority: i32) -> Result<EmbeddingQueueEntry> {
        let result = sqlx::query_as::<_, EmbeddingQueueEntry>(
            r#"
            INSERT INTO embedding_queue (document_id, priority)
            VALUES ($1, $2)
            ON CONFLICT (document_id) WHERE status = 'pending'
            DO UPDATE SET priority = GREATEST(embedding_queue.priority, $2)
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get next document from queue
    pub async fn get_next_from_queue(&self) -> Result<Option<EmbeddingQueueEntry>> {
        let result = sqlx::query_as::<_, EmbeddingQueueEntry>(
            r#"
            UPDATE embedding_queue
            SET status = 'processing', started_at = NOW(), attempts = attempts + 1
            WHERE id = (
                SELECT id FROM embedding_queue
                WHERE status = 'pending' AND attempts < max_attempts
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING *
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Mark queue entry as completed
    pub async fn complete_queue_entry(&self, entry_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE embedding_queue SET status = 'completed', completed_at = NOW() WHERE id = $1",
        )
        .bind(entry_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark queue entry as failed
    pub async fn fail_queue_entry(&self, entry_id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE embedding_queue
            SET status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'pending' END,
                error_message = $2
            WHERE id = $1
            "#,
        )
        .bind(entry_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get embedding statistics
    pub async fn get_stats(&self) -> Result<EmbeddingStats> {
        let stats = sqlx::query_as::<_, EmbeddingStatsRow>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM documents) as total_documents,
                (SELECT COUNT(DISTINCT document_id) FROM document_embeddings) as documents_with_embeddings,
                (SELECT COUNT(*) FROM chunk_embeddings) as total_chunks,
                (SELECT COUNT(*) FROM embedding_queue WHERE status = 'pending') as pending_queue,
                (SELECT COUNT(*) FROM embedding_queue WHERE status = 'failed') as failed_queue
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(EmbeddingStats {
            total_documents: stats.total_documents.unwrap_or(0),
            documents_with_embeddings: stats.documents_with_embeddings.unwrap_or(0),
            total_chunks: stats.total_chunks.unwrap_or(0),
            pending_queue: stats.pending_queue.unwrap_or(0),
            failed_queue: stats.failed_queue.unwrap_or(0),
        })
    }

    /// Split text into chunks for embedding
    pub fn chunk_text(&self, text: &str) -> Vec<ChunkData> {
        chunk_text_with_config(text, self.config.chunk_size, self.config.chunk_overlap)
    }
}

/// Split text into overlapping word chunks.
/// Pure function with no external dependencies, extracting it enables unit testing.
pub fn chunk_text_with_config(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<ChunkData> {
    if text.is_empty() || chunk_size == 0 {
        return Vec::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current_offset: i32 = 0;
    let step = if chunk_overlap < chunk_size { chunk_size - chunk_overlap } else { 1 };

    let mut i = 0;
    while i < words.len() {
        let end = (i + chunk_size).min(words.len());
        let chunk_words = &words[i..end];
        let chunk_text = chunk_words.join(" ");

        let start_offset = current_offset;
        let end_offset = start_offset + chunk_text.len() as i32;

        chunks.push(ChunkData {
            text: chunk_text.clone(),
            start_offset,
            end_offset,
            metadata: None,
        });

        // Approximate overlap in characters (5 chars/word average)
        current_offset = end_offset - (chunk_overlap * 5) as i32;
        i += step;
    }

    chunks
}

// Internal types for database queries

#[derive(sqlx::FromRow)]
struct SemanticSearchRow {
    document_id: Uuid,
    chunk_id: Option<Uuid>,
    chunk_text: Option<String>,
    similarity: f32,
    document_title: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EmbeddingStatsRow {
    total_documents: Option<i64>,
    documents_with_embeddings: Option<i64>,
    total_chunks: Option<i64>,
    pending_queue: Option<i64>,
    failed_queue: Option<i64>,
}

// OpenAI API types

#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let text = "This is a test sentence with multiple words for chunking purposes.";
        let chunks = chunk_text_with_config(text, 10, 2);

        assert!(!chunks.is_empty());
        assert!(!chunks[0].text.is_empty());
    }

    #[test]
    fn test_chunk_text_empty() {
        let chunks = chunk_text_with_config("", 10, 2);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_text_single_word() {
        let chunks = chunk_text_with_config("hello", 10, 2);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "hello");
    }

    #[test]
    fn test_chunk_text_respects_chunk_size() {
        // chunk_size=3 words, no overlap
        let text = "one two three four five six";
        let chunks = chunk_text_with_config(text, 3, 0);

        // With chunk_size=3 and 6 words and no overlap, expect 2 chunks
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "one two three");
        assert_eq!(chunks[1].text, "four five six");
    }

    #[test]
    fn test_chunk_text_offsets_are_sequential() {
        let text = "one two three four five six";
        let chunks = chunk_text_with_config(text, 3, 0);

        assert!(chunks[0].start_offset >= 0);
        assert!(chunks[0].end_offset > chunks[0].start_offset);
    }

    #[test]
    fn test_chunk_text_whitespace_only() {
        let chunks = chunk_text_with_config("   ", 10, 2);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_text_zero_chunk_size_returns_empty() {
        // chunk_size=0 is a degenerate case: function returns empty
        let chunks = chunk_text_with_config("hello world", 0, 0);
        assert!(chunks.is_empty(), "chunk_size=0 should return no chunks");
    }

    #[test]
    fn test_chunk_text_overlap_produces_more_chunks() {
        // Without overlap: 6 words / 3 chunk_size = 2 chunks
        // With overlap=1: step = 3-1 = 2, so more chunks
        let text = "one two three four five six";
        let no_overlap = chunk_text_with_config(text, 3, 0);
        let with_overlap = chunk_text_with_config(text, 3, 1);
        assert!(
            with_overlap.len() >= no_overlap.len(),
            "Overlap should produce at least as many chunks"
        );
    }

    #[test]
    fn test_chunk_text_each_chunk_has_expected_word_count() {
        // chunk_size=2 words, no overlap, 4 words -> 2 chunks of 2 words each
        let text = "alpha beta gamma delta";
        let chunks = chunk_text_with_config(text, 2, 0);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].text, "alpha beta");
        assert_eq!(chunks[1].text, "gamma delta");
    }

    #[test]
    fn test_chunk_text_last_chunk_may_be_smaller() {
        // 5 words with chunk_size=3, no overlap: [0..3], [3..5]
        let text = "one two three four five";
        let chunks = chunk_text_with_config(text, 3, 0);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[1].text, "four five", "Last chunk should have remaining words");
    }
}
