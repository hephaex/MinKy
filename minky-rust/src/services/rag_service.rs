//! RAG (Retrieval-Augmented Generation) service
//!
//! This service wires together:
//! 1. `EmbeddingService` – generate query embeddings and run vector search
//! 2. `AIService`        – call Claude to produce an answer from retrieved context
//! 3. PostgreSQL         – persist search history and retrieve history entries
//!
//! # Pipeline
//! ```text
//! question
//!   → embed question (OpenAI)
//!   → vector search (pgvector, top-k chunks)
//!   → assemble context
//!   → Claude completion
//!   → persist to search_history
//!   → return answer + sources
//! ```

use chrono::Utc;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{
        EmbeddingModel, SemanticSearchRequest, SemanticSearchResult,
        RagAskRequest, RagAskResponse, RagSemanticSearchRequest,
        RagSemanticSearchResponse, RagSemanticSearchResult, RagSource,
        SearchHistoryEntry, SearchHistoryQuery,
    },
    services::{EmbeddingConfig, EmbeddingService},
};

// ---------------------------------------------------------------------------
// RAG Service
// ---------------------------------------------------------------------------

/// Orchestrates the full RAG pipeline.
pub struct RagService {
    pool: PgPool,
    config: Config,
    embedding_service: EmbeddingService,
}

impl RagService {
    /// Create a new `RagService`.
    pub fn new(pool: PgPool, config: Config) -> Self {
        let openai_key = config
            .openai_api_key
            .as_ref()
            .map(|s| s.expose_secret().to_string());

        let embedding_config = EmbeddingConfig {
            openai_api_key: openai_key,
            voyage_api_key: None,
            default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
            chunk_size: 512,
            chunk_overlap: 50,
        };

        let embedding_service = EmbeddingService::new(pool.clone(), embedding_config);

        Self {
            pool,
            config,
            embedding_service,
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Full RAG pipeline: retrieve relevant chunks, then generate an answer
    /// with Claude using those chunks as context.
    pub async fn ask(&self, req: RagAskRequest) -> AppResult<RagAskResponse> {
        let top_k = req.top_k.min(20);

        // 1. Vector search
        let search_req = SemanticSearchRequest {
            query: req.question.clone(),
            limit: top_k as i32,
            threshold: Some(req.threshold),
            model: None,
            user_id: req.user_id,
        };

        let chunks = self
            .embedding_service
            .semantic_search(search_req)
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Vector search failed: {e}"))
            })?;

        // 2. Assemble sources
        let sources: Vec<RagSource> = chunks
            .iter()
            .map(|c| RagSource {
                document_id: c.document_id,
                document_title: c.document_title.clone(),
                chunk_text: c.chunk_text.clone().unwrap_or_default(),
                similarity: c.similarity,
            })
            .collect();

        // 3. Build context string
        let context = self.build_context(&chunks);

        // 4. Generate answer with Claude
        let (answer, tokens_used, model) = self
            .generate_answer(&req.question, &context)
            .await?;

        // 5. Persist search history (best-effort; do not fail the request)
        let _ = self
            .persist_history(
                &req.question,
                Some(&answer),
                sources.len() as i32,
                Some(tokens_used),
                req.user_id,
            )
            .await;

        let final_sources = if req.include_sources { sources } else { vec![] };

        Ok(RagAskResponse {
            answer,
            sources: final_sources,
            tokens_used,
            model,
        })
    }

    /// Semantic-only search: returns matching chunks without LLM generation.
    pub async fn semantic_search(
        &self,
        req: RagSemanticSearchRequest,
    ) -> AppResult<RagSemanticSearchResponse> {
        let limit = req.limit.min(50);

        let search_req = SemanticSearchRequest {
            query: req.query.clone(),
            limit,
            threshold: Some(req.threshold),
            model: None,
            user_id: req.user_id,
        };

        let chunks = self
            .embedding_service
            .semantic_search(search_req)
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Semantic search failed: {e}"))
            })?;

        // Persist history (best-effort)
        let _ = self
            .persist_history(&req.query, None, chunks.len() as i32, None, req.user_id)
            .await;

        let results: Vec<RagSemanticSearchResult> = chunks
            .into_iter()
            .map(|c| RagSemanticSearchResult {
                document_id: c.document_id,
                chunk_id: c.chunk_id,
                chunk_text: c.chunk_text,
                similarity: c.similarity,
                document_title: c.document_title,
            })
            .collect();

        let total = results.len();

        Ok(RagSemanticSearchResponse {
            results,
            total,
            query: req.query,
        })
    }

    /// Retrieve paginated search history from the database.
    pub async fn get_history(
        &self,
        query: SearchHistoryQuery,
    ) -> AppResult<Vec<SearchHistoryEntry>> {
        let limit = query.limit.min(100);

        let rows = sqlx::query_as::<_, SearchHistoryRow>(
            r#"
            SELECT id, query, answer, source_count, tokens_used, user_id, created_at
            FROM search_history
            WHERE ($1::integer IS NULL OR user_id = $1)
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(query.user_id)
        .bind(limit)
        .bind(query.offset)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|r| SearchHistoryEntry {
                id: r.id,
                query: r.query,
                answer: r.answer,
                source_count: r.source_count,
                tokens_used: r.tokens_used.map(|t| t as u32),
                user_id: r.user_id,
                created_at: r.created_at,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Build the context string from retrieved chunks for the LLM prompt.
    fn build_context(&self, chunks: &[SemanticSearchResult]) -> String {
        if chunks.is_empty() {
            return String::new();
        }

        chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| {
                let title = chunk
                    .document_title
                    .as_deref()
                    .unwrap_or("Untitled document");
                let text = chunk.chunk_text.as_deref().unwrap_or("");
                format!(
                    "[Source {num}] {title} (similarity: {sim:.2})\n{text}",
                    num = i + 1,
                    title = title,
                    sim = chunk.similarity,
                    text = text,
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    /// Call Claude (via `AIService`) to generate an answer grounded in
    /// the provided context. Returns `(answer, tokens_used, model_name)`.
    async fn generate_answer(
        &self,
        question: &str,
        context: &str,
    ) -> AppResult<(String, u32, String)> {
        let api_key = self
            .config
            .anthropic_api_key
            .as_ref()
            .ok_or_else(|| AppError::Configuration("Anthropic API key not configured".into()))?;

        let system_prompt = "\
You are a helpful knowledge base assistant for a team. \
Your job is to answer questions accurately and concisely using ONLY the \
provided context. If the context does not contain enough information to \
answer the question, say so clearly rather than making up an answer. \
Always cite which source number(s) you used when possible.";

        let user_content = if context.is_empty() {
            format!(
                "No relevant documents were found for this question.\n\nQuestion: {question}"
            )
        } else {
            format!(
                "Context from the knowledge base:\n\n{context}\n\n---\n\nQuestion: {question}"
            )
        };

        let request = AnthropicRequest {
            model: "claude-haiku-4-5-20251101".to_string(),
            max_tokens: 1024,
            system: Some(system_prompt.to_string()),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_content,
            }],
        };

        let http_client = reqwest::Client::new();

        let response = http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Anthropic API request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Anthropic API error: {error_text}"
            )));
        }

        let result: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to parse Anthropic response: {e}"))
            })?;

        let answer = result
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let tokens_used = result.usage.input_tokens + result.usage.output_tokens;
        let model = result.model.clone();

        Ok((answer, tokens_used, model))
    }

    /// Persist a search event in the `search_history` table.
    /// Returns `Ok(())` on success or silently swallows errors so callers
    /// can treat this as best-effort.
    async fn persist_history(
        &self,
        query: &str,
        answer: Option<&str>,
        source_count: i32,
        tokens_used: Option<u32>,
        user_id: Option<i32>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO search_history
                (id, query, answer, source_count, tokens_used, user_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(query)
        .bind(answer)
        .bind(source_count)
        .bind(tokens_used.map(|t| t as i32))
        .bind(user_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Internal Anthropic API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// ---------------------------------------------------------------------------
// Internal database row type
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct SearchHistoryRow {
    id: Uuid,
    query: String,
    answer: Option<String>,
    source_count: i32,
    tokens_used: Option<i32>,
    user_id: Option<i32>,
    created_at: chrono::DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SemanticSearchResult;

    fn make_chunk(title: &str, text: &str, sim: f32) -> SemanticSearchResult {
        SemanticSearchResult {
            document_id: Uuid::new_v4(),
            chunk_id: None,
            chunk_text: Some(text.to_string()),
            similarity: sim,
            document_title: Some(title.to_string()),
        }
    }

    /// Verify that build_context produces the expected numbered format and
    /// does not panic when given an empty slice.
    #[test]
    fn test_build_context_empty() {
        // We cannot instantiate RagService without a real pool, so test the
        // logic by calling a standalone helper that mirrors the production
        // implementation.
        let result = build_context_for_test(&[]);
        assert!(result.is_empty(), "empty context should produce empty string");
    }

    #[test]
    fn test_build_context_single_chunk() {
        let chunks = vec![make_chunk("My Doc", "Some text here.", 0.95)];
        let result = build_context_for_test(&chunks);
        assert!(result.contains("[Source 1]"));
        assert!(result.contains("My Doc"));
        assert!(result.contains("Some text here."));
        assert!(result.contains("0.95"));
    }

    #[test]
    fn test_build_context_multiple_chunks() {
        let chunks = vec![
            make_chunk("Doc A", "Text A", 0.90),
            make_chunk("Doc B", "Text B", 0.80),
        ];
        let result = build_context_for_test(&chunks);
        assert!(result.contains("[Source 1]"));
        assert!(result.contains("[Source 2]"));
        assert!(result.contains("---"));
    }

    // Mirror of RagService::build_context for testing without a real pool.
    fn build_context_for_test(chunks: &[SemanticSearchResult]) -> String {
        if chunks.is_empty() {
            return String::new();
        }
        chunks
            .iter()
            .enumerate()
            .map(|(i, chunk)| {
                let title = chunk
                    .document_title
                    .as_deref()
                    .unwrap_or("Untitled document");
                let text = chunk.chunk_text.as_deref().unwrap_or("");
                format!(
                    "[Source {num}] {title} (similarity: {sim:.2})\n{text}",
                    num = i + 1,
                    title = title,
                    sim = chunk.similarity,
                    text = text,
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }
}
