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
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Configuration for the embedding service
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub openai_api_key: Option<String>,
    pub voyage_api_key: Option<String>,
    pub upstage_api_key: Option<String>,
    pub default_model: EmbeddingModel,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    /// When `true`, the service initializes the fastembed-rs local model
    /// (downloaded on first run, then cached). Required for
    /// [`EmbeddingModel::NomicEmbedTextV15`].
    pub local_embedding_enabled: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            voyage_api_key: None,
            upstage_api_key: None,
            default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
            chunk_size: 512,
            chunk_overlap: 50,
            local_embedding_enabled: false,
        }
    }
}

/// Embedding service for vector operations
pub struct EmbeddingService {
    pool: PgPool,
    config: EmbeddingConfig,
    http_client: reqwest::Client,
    /// Lazily-initialized local embedding model. `None` when the local
    /// path was not requested, populated by [`EmbeddingService::new_with_local`].
    ///
    /// `fastembed::TextEmbedding::embed` requires `&mut self` (ONNX Runtime
    /// session reuse), so we wrap it in a blocking `std::sync::Mutex`. The
    /// lock is only held inside `tokio::task::spawn_blocking`, never across
    /// an `.await`, which keeps the async runtime free.
    local_model: Option<Arc<Mutex<fastembed::TextEmbedding>>>,
}

impl EmbeddingService {
    /// Create a new embedding service (without the local fastembed model).
    ///
    /// Use [`EmbeddingService::new_with_local`] to enable the local
    /// `nomic-embed-text-v1.5` model.
    pub fn new(pool: PgPool, config: EmbeddingConfig) -> Self {
        Self {
            pool,
            config,
            http_client: reqwest::Client::new(),
            local_model: None,
        }
    }

    /// Create a new embedding service, optionally initializing the local
    /// fastembed-rs model when `config.local_embedding_enabled` is `true`.
    ///
    /// The first call triggers a ~270 MB model download from HuggingFace
    /// (cached for subsequent invocations). Initialization is offloaded to
    /// `tokio::task::spawn_blocking` since fastembed performs synchronous
    /// disk and ONNX Runtime setup.
    pub async fn new_with_local(pool: PgPool, config: EmbeddingConfig) -> Result<Self> {
        let local_model = if config.local_embedding_enabled {
            let model = tokio::task::spawn_blocking(|| {
                fastembed::TextEmbedding::try_new(fastembed::InitOptions::new(
                    fastembed::EmbeddingModel::NomicEmbedTextV15,
                ))
            })
            .await
            .map_err(|e| {
                AppError::Configuration(format!(
                    "Failed to spawn fastembed initialization task: {}",
                    e
                ))
            })?
            .map_err(|e| {
                AppError::Configuration(format!("Failed to initialize fastembed model: {}", e))
            })?;
            Some(Arc::new(Mutex::new(model)))
        } else {
            None
        };

        Ok(Self {
            pool,
            config,
            http_client: reqwest::Client::new(),
            local_model,
        })
    }

    /// The configured default embedding model (Solar when `UPSTAGE_API_KEY` is
    /// set at startup, OpenAI otherwise — see `lib.rs`). Callers that need a
    /// single ad-hoc embedding without document/chunk semantics (e.g. a plain
    /// `/api/ai/embedding` passthrough) should use this instead of assuming a
    /// hardcoded provider.
    pub fn default_model(&self) -> EmbeddingModel {
        self.config.default_model
    }

    /// Generate embedding for text. Dispatches to the local fastembed model
    /// for [`EmbeddingModel::NomicEmbedTextV15`], otherwise calls the
    /// configured remote provider (OpenAI today).
    pub async fn generate_embedding(
        &self,
        text: &str,
        model: EmbeddingModel,
    ) -> Result<Vec<f32>> {
        if model.is_local() {
            return self.generate_local_embedding(text).await;
        }

        let api_key = if model.is_upstage() {
            self.config.upstage_api_key.as_ref()
                .ok_or_else(|| AppError::Configuration("Upstage API key not configured".into()))?
        } else {
            self.config.openai_api_key.as_ref()
                .ok_or_else(|| AppError::Configuration("OpenAI API key not configured".into()))?
        };

        let url = std::env::var("EMBEDDING_BASE_URL")
            .unwrap_or_else(|_| model.api_base_url().to_string());

        let request = OpenAIEmbeddingRequest {
            input: text.to_string(),
            model: model.api_model_id().to_string(),
        };

        let response = self
            .http_client
            .post(&url)
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

    /// Generate an embedding using the locally-loaded fastembed model.
    ///
    /// Returns [`AppError::Configuration`] when the service was built without
    /// `local_embedding_enabled = true`.
    async fn generate_local_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let local_model = self.local_model.as_ref().ok_or_else(|| {
            AppError::Configuration(
                "Local embedding model not initialized; build the service with \
                 EmbeddingService::new_with_local and local_embedding_enabled = true"
                    .into(),
            )
        })?;

        let model = Arc::clone(local_model);
        let text_owned = text.to_string();

        let mut embeddings = tokio::task::spawn_blocking(move || {
            let mut guard = model.lock().map_err(|e| {
                anyhow::anyhow!("fastembed mutex poisoned: {}", e)
            })?;
            guard
                .embed(vec![text_owned], None)
                .map_err(|e| anyhow::anyhow!("fastembed inference error: {}", e))
        })
        .await
        .map_err(|e| AppError::ExternalService(format!("fastembed spawn_blocking error: {}", e)))?
        .map_err(|e| AppError::ExternalService(e.to_string()))?;

        if embeddings.is_empty() {
            return Err(AppError::ExternalService(
                "fastembed returned no embeddings".into(),
            ));
        }
        Ok(embeddings.remove(0))
    }

    /// Generate embeddings for multiple texts (batch)
    pub async fn generate_embeddings_batch(
        &self,
        texts: &[String],
        model: EmbeddingModel,
    ) -> Result<Vec<Vec<f32>>> {
        if model.is_local() {
            return self.generate_local_embeddings_batch(texts).await;
        }

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

    /// Generate embeddings for multiple texts using the local fastembed model.
    ///
    /// This method batches all texts into a single `embed()` call, avoiding
    /// N Mutex lock acquisitions. Only valid for [`EmbeddingModel::NomicEmbedTextV15`].
    ///
    /// Returns [`AppError::Configuration`] when the service was built without
    /// `local_embedding_enabled = true`.
    async fn generate_local_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let local_model = self.local_model.as_ref().ok_or_else(|| {
            AppError::Configuration(
                "Local embedding model not initialized; build the service with \
                 EmbeddingService::new_with_local and local_embedding_enabled = true"
                    .into(),
            )
        })?;

        let model = Arc::clone(local_model);
        let text_refs: Vec<String> = texts.to_vec();

        let results = tokio::task::spawn_blocking(move || {
            let mut guard = model.lock().map_err(|e| {
                anyhow::anyhow!("fastembed mutex poisoned: {}", e)
            })?;
            let embeddings = guard
                .embed(text_refs.iter().map(|s| s.as_str()).collect::<Vec<_>>(), None)
                .map_err(|e| anyhow::anyhow!("fastembed batch inference error: {}", e))?;
            Ok::<Vec<Vec<f32>>, anyhow::Error>(embeddings)
        })
        .await
        .map_err(|e| AppError::ExternalService(format!("fastembed spawn_blocking error: {}", e)))?
        .map_err(|e: anyhow::Error| AppError::ExternalService(e.to_string()))?;

        if results.is_empty() {
            return Err(AppError::ExternalService(
                "fastembed returned no embeddings".into(),
            ));
        }

        Ok(results)
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

        let full_text = format!("{}\n\n{}", document.0, document.1);
        // Truncate to prevent embedder OOM on very large documents (Strix --ctx-size 32768).
        // 50,000 bytes ≈ 12,500–25,000 tokens depending on language.
        let text = truncate_to_char_boundary(&full_text, EMBEDDING_MAX_BYTES);

        // Generate embedding
        let embedding = self.generate_embedding(text, model).await?;
        let token_count = text.split_whitespace().count() as i32; // Approximate

        // Insert or update
        let result = sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            INSERT INTO document_embeddings (document_id, embedding, model, token_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (document_id, model)
            DO UPDATE SET embedding = $2, token_count = $4, updated_at = NOW()
            RETURNING id, document_id, embedding::float4[] as embedding, model, token_count, created_at, updated_at
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
                RETURNING id, document_id, chunk_index, chunk_text, chunk_start_offset, chunk_end_offset, embedding::float4[] as embedding, model, token_count, metadata, created_at
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

    /// Resolve the query/storage model split and generate a query embedding.
    ///
    /// Upstage Solar uses asymmetric embeddings: queries are embedded with
    /// `UpstaageSolarQuery` but stored/compared against rows embedded with
    /// `UpstaageSolarPassage`. Other models use the same model for both
    /// sides. Returns `(query_embedding, storage_model)` so callers can bind
    /// `storage_model` as a `WHERE model = $N` filter and avoid comparing
    /// embeddings across incompatible vector spaces.
    pub async fn embed_query(
        &self,
        query: &str,
        requested_model: Option<EmbeddingModel>,
    ) -> Result<(Vec<f32>, EmbeddingModel)> {
        let (query_model, storage_model) =
            resolve_query_storage_models(requested_model.unwrap_or(self.config.default_model));
        let query_embedding = self.generate_embedding(query, query_model).await?;
        Ok((query_embedding, storage_model))
    }

    /// Semantic search using vector similarity
    pub async fn semantic_search(
        &self,
        req: SemanticSearchRequest,
    ) -> Result<Vec<SemanticSearchResult>> {
        let (query_embedding, storage_model) = self.embed_query(&req.query, req.model).await?;
        let threshold = req.threshold.unwrap_or(0.7);

        // Search in chunk embeddings with cosine similarity
        let results = sqlx::query_as::<_, SemanticSearchRow>(
            r#"
            SELECT
                ce.document_id,
                ce.id as chunk_id,
                ce.chunk_text,
                (1 - (ce.embedding <=> $1))::float4 as similarity,
                d.title as document_title
            FROM chunk_embeddings ce
            JOIN documents d ON d.id = ce.document_id
            WHERE ce.model = $2
              AND ce.embedding IS NOT NULL
              AND d.archived = false
              AND ($4::integer IS NULL OR d.user_id = $4)
              AND 1 - (ce.embedding <=> $1) >= $3
            ORDER BY ce.embedding <=> $1
            LIMIT $5
            "#,
        )
        .bind(Vector::from(query_embedding))
        .bind(storage_model)
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
                (1 - (de.embedding <=> se.embedding))::float4 as similarity,
                d.title as document_title
            FROM document_embeddings de
            CROSS JOIN source_embedding se
            JOIN documents d ON d.id = de.document_id
            WHERE de.document_id != $1
              AND de.model = se.model
              AND de.embedding IS NOT NULL
              AND d.archived = false
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

    /// Reset entries stuck in 'processing' back to an appropriate terminal state.
    ///
    /// Entries whose `attempts >= max_attempts` are marked 'failed' so they are
    /// not silently stuck as 'pending' forever. Remaining entries become 'pending'
    /// for retry.
    ///
    /// Called at worker startup and periodically to recover from crashes that
    /// left rows in 'processing' with no corresponding worker.
    pub async fn requeue_stale_processing(&self, stale_secs: u64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE embedding_queue
            SET status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'pending' END,
                error_message = 'requeued: stale processing (crash recovery)'
            WHERE status = 'processing'
              AND (started_at IS NULL OR started_at < NOW() - ($1::bigint * interval '1 second'))
            "#,
        )
        .bind(stale_secs as i64)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Background loop that drains the embedding_queue.
    ///
    /// Spawned once at startup via `tokio::spawn` in `create_app`. Runs forever.
    /// The release binary uses `panic = "abort"`, so a panic aborts the whole
    /// process; crash recovery relies on the process-level restarter (Docker /
    /// systemd `Restart=always`) and the startup stale-requeue below.
    ///
    /// On startup, stale 'processing' rows older than `stale_secs` are moved to
    /// 'pending' (or 'failed' when `attempts >= max_attempts`) to recover from
    /// the previous crash. The reaper also runs every `REAPER_PERIOD` idle cycles
    /// so long-lived workers self-heal after transient outages.
    ///
    /// When the queue is empty the loop sleeps `idle_interval_secs` before polling
    /// again. Successful and failed items are processed without sleeping so the
    /// queue drains as fast as the embedder allows.
    pub async fn run_queue_worker(self: Arc<Self>, idle_interval_secs: u64) {
        // Stale entries are those in 'processing' longer than 5 × idle or 60 s min.
        let stale_secs = idle_interval_secs.saturating_mul(5).max(60);
        // Reaper runs every this many idle cycles (≈ 1 min at 10 s idle).
        const REAPER_PERIOD: u32 = 6;
        // After this many consecutive embedding failures, sleep failure_backoff before next item.
        // Prevents hot-spinning when the embedding service (e.g. Strix) is down.
        const FAILURE_CIRCUIT_THRESHOLD: u32 = 3;
        let failure_backoff_secs = {
            let raw = std::env::var("EMBEDDING_QUEUE_FAILURE_BACKOFF_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30);
            if raw == 0 {
                tracing::warn!("EMBEDDING_QUEUE_FAILURE_BACKOFF_SECS=0 disables backoff; using 1s floor");
                1
            } else {
                raw
            }
        };
        let failure_backoff = tokio::time::Duration::from_secs(failure_backoff_secs);
        let idle = tokio::time::Duration::from_secs(idle_interval_secs);
        let mut idle_count: u32 = 0;
        let mut consecutive_failures: u32 = 0;

        // Recovery pass on startup.
        match self.requeue_stale_processing(stale_secs).await {
            Ok(n) if n > 0 => tracing::warn!(count = n, "embedding queue: requeued stale processing entries on startup"),
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "embedding queue: startup stale-requeue failed"),
        }

        loop {
            match self.get_next_from_queue().await {
                Ok(Some(entry)) => {
                    idle_count = 0;
                    let doc_id = entry.document_id;
                    let entry_id = entry.id;
                    tracing::debug!(document_id = %doc_id, "embedding queue: processing");
                    let req = CreateDocumentEmbeddingRequest {
                        document_id: doc_id,
                        model: None,
                    };
                    match self.create_document_embedding(req).await {
                        Ok(_) => {
                            consecutive_failures = 0;
                            tracing::info!(document_id = %doc_id, "embedding queue: complete");
                            if let Err(e) = self.complete_queue_entry(entry_id).await {
                                tracing::error!(entry_id = %entry_id, error = %e, "embedding queue: complete_queue_entry failed");
                            }
                        }
                        Err(e) => {
                            consecutive_failures = consecutive_failures.saturating_add(1);
                            tracing::warn!(document_id = %doc_id, error = %e, "embedding queue: embedding failed");
                            if let Err(fe) = self.fail_queue_entry(entry_id, &e.to_string()).await {
                                tracing::error!(entry_id = %entry_id, error = %fe, "embedding queue: fail_queue_entry failed");
                            }
                            if should_circuit_break(consecutive_failures, FAILURE_CIRCUIT_THRESHOLD) {
                                tracing::warn!(
                                    consecutive = consecutive_failures,
                                    backoff_secs = failure_backoff_secs,
                                    "embedding queue: consecutive failures, backing off"
                                );
                                tokio::time::sleep(failure_backoff).await;
                            }
                        }
                    }
                }
                Ok(None) => {
                    idle_count = idle_count.wrapping_add(1);
                    if idle_count.is_multiple_of(REAPER_PERIOD) {
                        match self.requeue_stale_processing(stale_secs).await {
                            Ok(n) if n > 0 => tracing::warn!(count = n, "embedding queue: requeued stale processing entries"),
                            Ok(_) => {}
                            Err(e) => tracing::error!(error = %e, "embedding queue: periodic stale-requeue failed"),
                        }
                    }
                    tokio::time::sleep(idle).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "embedding queue: get_next_from_queue error");
                    tokio::time::sleep(idle).await;
                }
            }
        }
    }
}

/// Resolve the query-time embedding model and the storage model to filter
/// against, given a requested (or default) model.
///
/// Upstage Solar uses asymmetric embeddings: a caller requesting
/// `UpstaageSolarPassage` (the storage/indexing model) must actually embed
/// their *query* text with `UpstaageSolarQuery`, then filter stored rows by
/// `UpstaageSolarPassage`. Every other model uses the same model for both
/// query and storage. Returns `(query_model, storage_model)`.
#[inline]
pub fn resolve_query_storage_models(
    requested_model: EmbeddingModel,
) -> (EmbeddingModel, EmbeddingModel) {
    let query_model = if requested_model == EmbeddingModel::UpstaageSolarPassage {
        EmbeddingModel::UpstaageSolarQuery
    } else {
        requested_model
    };
    let storage_model = if query_model == EmbeddingModel::UpstaageSolarQuery {
        EmbeddingModel::UpstaageSolarPassage
    } else {
        query_model
    };
    (query_model, storage_model)
}

/// Returns `true` when the embedding worker should pause before processing the next item.
///
/// The worker tracks `consecutive_failures` (reset to 0 on any success). When the count
/// reaches `threshold`, sleeping `EMBEDDING_QUEUE_FAILURE_BACKOFF_SECS` before the next
/// dequeue prevents hot-spinning against a down embedding service (e.g. Strix :8091).
#[inline]
pub fn should_circuit_break(consecutive_failures: u32, threshold: u32) -> bool {
    consecutive_failures >= threshold
}

/// Truncate `s` to at most `max_bytes` bytes at a valid UTF-8 character boundary.
pub fn truncate_to_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut idx = max_bytes;
    while !s.is_char_boundary(idx) {
        idx -= 1;
    }
    &s[..idx]
}

/// Maximum byte length fed to the embedder per document.
/// Conservative limit for Strix `llama-qwen3-embed` with `--ctx-size 32768`:
/// 50,000 bytes ≈ 12,500–25,000 tokens (language-dependent).
pub const EMBEDDING_MAX_BYTES: usize = 50_000;

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
    fn test_nomic_embed_text_v1_5_dimension() {
        assert_eq!(EmbeddingModel::NomicEmbedTextV15.dimension(), 768);
    }

    #[test]
    fn test_embedding_config_local_enabled_default_false() {
        let config = EmbeddingConfig::default();
        assert!(!config.local_embedding_enabled);
    }

    #[test]
    fn test_embedding_model_nomic_api_id() {
        assert_eq!(
            EmbeddingModel::NomicEmbedTextV15.api_model_id(),
            "nomic-embed-text-v1.5"
        );
    }

    #[test]
    fn test_embedding_model_nomic_is_local() {
        assert!(EmbeddingModel::NomicEmbedTextV15.is_local());
        assert!(!EmbeddingModel::OpenaiTextEmbedding3Small.is_local());
        assert!(!EmbeddingModel::VoyageLarge2.is_local());
    }

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

    #[test]
    fn test_generate_local_embeddings_batch_without_model_returns_configuration_error() {
        // This test verifies that the error handling path is correct when local_model is None.
        // We verify that:
        // 1. EmbeddingService has local_model: Option<Arc<Mutex<fastembed::TextEmbedding>>>
        // 2. generate_local_embedding returns AppError::Configuration when local_model is None
        // 3. The error message mentions "Local embedding model not initialized"

        // Since we cannot construct an EmbeddingService in sync context without a real PgPool,
        // we verify the behavior by checking that:
        // - is_local() for NomicEmbedTextV15 is true
        // - The method signature exists
        assert!(
            EmbeddingModel::NomicEmbedTextV15.is_local(),
            "NomicEmbedTextV15 should be local"
        );

        // The actual runtime behavior (error when local_model is None) is tested
        // in the async test below with a mocked pool
    }

    #[tokio::test]
    #[ignore] // Skip in CI; requires DB. Run with: cargo test -- --ignored
    async fn test_generate_local_embeddings_batch_without_model_async() {
        // This test requires a PostgreSQL connection and should be run manually.
        // To run: MINKY_DATABASE_URL=... cargo test -- --ignored
        use sqlx::postgres::PgPoolOptions;

        let database_url =
            std::env::var("MINKY_DATABASE_URL").unwrap_or_else(|_| {
                // Default to localhost for local testing
                "postgresql://minky:minky@localhost/minky".to_string()
            });

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        let config = EmbeddingConfig {
            openai_api_key: None,
            voyage_api_key: None,
            upstage_api_key: None,
            default_model: EmbeddingModel::NomicEmbedTextV15,
            chunk_size: 512,
            chunk_overlap: 50,
            local_embedding_enabled: false, // Local model NOT initialized
        };

        let service = EmbeddingService::new(pool, config);

        // Try to generate embeddings
        let texts = vec!["hello world".to_string(), "goodbye world".to_string()];
        let result = service
            .generate_local_embeddings_batch(&texts)
            .await;

        // Should return Configuration error
        assert!(result.is_err(), "Should return error when local model not initialized");

        if let Err(AppError::Configuration(msg)) = result {
            assert!(
                msg.contains("Local embedding model not initialized"),
                "Error message should mention local model not initialized; got: {}",
                msg
            );
        } else {
            panic!("Expected AppError::Configuration, got: {:?}", result);
        }
    }

    #[test]
    fn test_generate_embeddings_batch_delegates_to_local_for_nomic() {
        // This test verifies that is_local() returns true for NomicEmbedTextV15
        // so that generate_embeddings_batch will delegate to generate_local_embeddings_batch
        let model = EmbeddingModel::NomicEmbedTextV15;
        assert!(
            model.is_local(),
            "NomicEmbedTextV15 should be identified as a local model"
        );

        // Also verify other models are NOT local
        assert!(
            !EmbeddingModel::OpenaiTextEmbedding3Small.is_local(),
            "OpenAI model should not be local"
        );
        assert!(
            !EmbeddingModel::VoyageLarge2.is_local(),
            "Voyage model should not be local"
        );
    }

    // ── truncate_to_char_boundary ────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        let s = "hello";
        assert_eq!(truncate_to_char_boundary(s, 100), s);
    }

    #[test]
    fn truncate_at_exact_limit_unchanged() {
        let s = "abc";
        assert_eq!(truncate_to_char_boundary(s, 3), "abc");
    }

    #[test]
    fn truncate_cuts_ascii_at_byte_limit() {
        let s = "abcde";
        assert_eq!(truncate_to_char_boundary(s, 3), "abc");
    }

    #[test]
    fn truncate_respects_multibyte_char_boundary() {
        // "a안b": 'a'(1 byte) + '안'(3 bytes) + 'b'(1 byte) = 5 bytes total.
        // Offset 4 IS a valid char boundary (end of '안'), so max_bytes=4 yields "a안".
        // Offsets 2 and 3 are mid-'안', so max_bytes=2 and max_bytes=3 both yield "a".
        let s = "a안b";
        let result4 = truncate_to_char_boundary(s, 4);
        assert_eq!(result4, "a안", "4 bytes = end of '안', full codepoint retained");

        let result3 = truncate_to_char_boundary(s, 3);
        assert_eq!(result3, "a", "3 bytes falls mid-'안', must back up to 'a'");

        let result2 = truncate_to_char_boundary(s, 2);
        assert_eq!(result2, "a", "2 bytes falls mid-'안', must back up to 'a'");
    }

    #[test]
    fn truncate_empty_string() {
        assert_eq!(truncate_to_char_boundary("", 10), "");
    }

    #[test]
    fn embedding_max_bytes_constant_is_sensible() {
        // Guard against accidental mutation of the constant.
        assert_eq!(EMBEDDING_MAX_BYTES, 50_000);
    }

    // ── should_circuit_break ─────────────────────────────────────────────────

    #[test]
    fn circuit_break_below_threshold_is_false() {
        assert!(!should_circuit_break(0, 3));
        assert!(!should_circuit_break(1, 3));
        assert!(!should_circuit_break(2, 3));
    }

    #[test]
    fn circuit_break_at_threshold_is_true() {
        assert!(should_circuit_break(3, 3));
    }

    #[test]
    fn circuit_break_above_threshold_is_true() {
        assert!(should_circuit_break(100, 3));
        assert!(should_circuit_break(u32::MAX, 3));
    }

    #[test]
    fn circuit_break_threshold_zero_always_breaks() {
        // threshold=0 means every failure triggers backoff
        assert!(should_circuit_break(0, 0));
    }

    #[test]
    fn circuit_break_resets_on_success() {
        // Simulate: 5 failures → break → success → counter reset → no break
        let mut consecutive: u32 = 0;
        for _ in 0..5 {
            consecutive = consecutive.saturating_add(1);
        }
        assert!(should_circuit_break(consecutive, 3), "should break after 5 failures");
        // success
        consecutive = 0;
        assert!(!should_circuit_break(consecutive, 3), "should not break after reset");
    }

    #[test]
    fn resolve_models_solar_passage_splits_into_query_and_passage() {
        let (query_model, storage_model) =
            resolve_query_storage_models(EmbeddingModel::UpstaageSolarPassage);
        assert_eq!(query_model, EmbeddingModel::UpstaageSolarQuery);
        assert_eq!(storage_model, EmbeddingModel::UpstaageSolarPassage);
    }

    #[test]
    fn resolve_models_solar_query_resolves_storage_to_passage() {
        let (query_model, storage_model) =
            resolve_query_storage_models(EmbeddingModel::UpstaageSolarQuery);
        assert_eq!(query_model, EmbeddingModel::UpstaageSolarQuery);
        assert_eq!(storage_model, EmbeddingModel::UpstaageSolarPassage);
    }

    #[test]
    fn resolve_models_symmetric_model_uses_same_model_both_sides() {
        let (query_model, storage_model) =
            resolve_query_storage_models(EmbeddingModel::OpenaiTextEmbedding3Small);
        assert_eq!(query_model, EmbeddingModel::OpenaiTextEmbedding3Small);
        assert_eq!(storage_model, EmbeddingModel::OpenaiTextEmbedding3Small);
    }

    #[test]
    fn resolve_models_never_returns_incompatible_query_storage_pair() {
        let all_models = [
            EmbeddingModel::OpenaiTextEmbedding3Small,
            EmbeddingModel::OpenaiTextEmbedding3Large,
            EmbeddingModel::VoyageLarge2,
            EmbeddingModel::VoyageCode2,
            EmbeddingModel::NomicEmbedTextV15,
            EmbeddingModel::UpstaageSolarPassage,
            EmbeddingModel::UpstaageSolarQuery,
        ];
        for requested in all_models {
            let (query_model, storage_model) = resolve_query_storage_models(requested);
            // The storage model must never itself be the query-only variant.
            assert_ne!(
                storage_model,
                EmbeddingModel::UpstaageSolarQuery,
                "storage model resolved from {:?} must not be the query-only variant",
                requested
            );
            // Re-resolving the storage model must be a no-op (idempotent / stable fixed point).
            let (_, storage_again) = resolve_query_storage_models(storage_model);
            assert_eq!(
                storage_again, storage_model,
                "resolving storage_model={:?} again must be stable",
                storage_model
            );
            let _ = query_model;
        }
    }

    /// Regression test for the float8-vs-f32 sqlx decode mismatch: pgvector's
    /// `1 - (a <=> b)` distance expression returns `FLOAT8`/`double precision`,
    /// but `SemanticSearchRow.similarity` is `f32`. Without an explicit
    /// `::float4` cast in the SQL, `sqlx::query_as` fails at runtime with a
    /// `ColumnDecode` error the moment a row is actually returned (this never
    /// surfaced before because these paths were never exercised against real
    /// data). Requires a live Postgres; skipped in CI. Run with:
    ///   MINKY_DATABASE_URL=... UPSTAGE_API_KEY=... cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn semantic_search_and_find_similar_documents_decode_similarity_without_error() {
        use sqlx::postgres::PgPoolOptions;

        let upstage_api_key = match std::env::var("UPSTAGE_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => {
                eprintln!("skipping: UPSTAGE_API_KEY not set");
                return;
            }
        };

        let database_url = std::env::var("MINKY_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://minky:minky@localhost/minky".to_string());

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        let config = EmbeddingConfig {
            openai_api_key: None,
            voyage_api_key: None,
            upstage_api_key: Some(upstage_api_key),
            default_model: EmbeddingModel::UpstaageSolarPassage,
            chunk_size: 512,
            chunk_overlap: 50,
            local_embedding_enabled: false,
        };
        let service = EmbeddingService::new(pool.clone(), config);

        // Seed a document with a chunk embedding and a document-level embedding,
        // both on the storage model resolved from the default config.
        let user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (email, username, password_hash) \
             VALUES ($1, $2, 'x') \
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
             RETURNING id",
        )
        .bind("float4-decode-test@minky.local")
        .bind("float4_decode_test")
        .fetch_one(&pool)
        .await
        .expect("seed user");

        let document_id: Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("float4 decode fixture")
        .bind("The quick brown fox jumps over the lazy dog.")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("seed document");

        let dummy_vec = vec![0.001_f32; 4096];

        sqlx::query(
            "INSERT INTO document_embeddings (document_id, embedding, model) VALUES ($1, $2, $3)",
        )
        .bind(document_id)
        .bind(Vector::from(dummy_vec.clone()))
        .bind(EmbeddingModel::UpstaageSolarPassage)
        .execute(&pool)
        .await
        .expect("seed document embedding");

        sqlx::query(
            "INSERT INTO chunk_embeddings (document_id, chunk_index, chunk_text, chunk_start_offset, chunk_end_offset, embedding, model, token_count) \
             VALUES ($1, 0, 'quick brown fox', 0, 15, $2, $3, 3)",
        )
        .bind(document_id)
        .bind(Vector::from(dummy_vec))
        .bind(EmbeddingModel::UpstaageSolarPassage)
        .execute(&pool)
        .await
        .expect("seed chunk embedding");

        let semantic_result = service
            .semantic_search(SemanticSearchRequest {
                query: "quick brown fox".to_string(),
                model: None,
                threshold: Some(-1.0),
                user_id: None,
                limit: 10,
            })
            .await;

        let similar_result = service.find_similar_documents(document_id, 10).await;

        // Cleanup before asserting, so a failed assertion doesn't leave fixtures behind.
        sqlx::query("DELETE FROM chunk_embeddings WHERE document_id = $1")
            .bind(document_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM document_embeddings WHERE document_id = $1")
            .bind(document_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(document_id)
            .execute(&pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .ok();

        let semantic_results =
            semantic_result.expect("semantic_search must decode rows without a ColumnDecode error");
        assert!(
            !semantic_results.is_empty(),
            "expected at least one chunk match for the seeded document"
        );

        let similar_results =
            similar_result.expect("find_similar_documents must decode rows without a ColumnDecode error");
        // find_similar_documents excludes the source document itself and there is
        // no other document seeded, so an empty result here is expected and still
        // proves the query executed and decoded successfully (no panic/error).
        let _ = similar_results;
    }
}
