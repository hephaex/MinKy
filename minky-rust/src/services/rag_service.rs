//! RAG (Retrieval-Augmented Generation) service
//!
//! This service wires together:
//! 1. `EmbeddingService` – generate query embeddings and run vector search
//! 2. Strix Halo (local llama.cpp, OpenAI-compatible) – generate an answer from retrieved context
//! 3. PostgreSQL         – persist search history and retrieve history entries
//!
//! # Pipeline
//! ```text
//! question
//!   → embed question (OpenAI-compatible, see EMBEDDING_BASE_URL)
//!   → vector search (pgvector, top-k chunks)
//!   → assemble context
//!   → Strix Halo completion (LLM_BASE_URL, OpenAI-compatible /v1/chat/completions)
//!   → persist to search_history
//!   → return answer + sources
//! ```
//!
//! Answer generation defaults to a local model served by Strix Halo
//! (`LLM_BASE_URL`, default `http://100.64.0.15:8093/v1/chat/completions`,
//! no API key required). The Anthropic Claude integration was removed
//! (2026-09-21) so RAG works without an external LLM API key.

use chrono::Utc;
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
    services::{
        local_llm_types::{LocalLlmMessage, LocalLlmRequest, LocalLlmResponse},
        EmbeddingConfig, EmbeddingService,
    },
};

/// Default Strix Halo endpoint for RAG answer generation (OpenAI-compatible,
/// no API key required). Override with the `LLM_BASE_URL` env var.
const DEFAULT_LLM_BASE_URL: &str = "http://100.64.0.15:8093/v1/chat/completions";
/// Model name reported by the Strix Halo llama.cpp server. llama.cpp serves a
/// single model per instance, so this value was informational only when this
/// service hit Strix directly. It stopped being informational once routed
/// through CT211 (continuum-router, 2026-09-28): that router validates
/// `model` against its registered backends and rejects an unregistered
/// spelling (`model '...' is not available`, confirmed live). CT211 echoes
/// back the id WITH the `/models/` mount prefix from llama.cpp's own launch
/// argument, so this must carry that prefix to match. Kept in sync with the
/// identical constant in routes/rag.rs.
const DEFAULT_LLM_MODEL: &str = "/models/Qwen3.8-27B-UD-Q4_K_XL.gguf";

// ---------------------------------------------------------------------------
// RAG Service
// ---------------------------------------------------------------------------

/// Orchestrates the full RAG pipeline.
pub struct RagService {
    pool: PgPool,
    embedding_service: EmbeddingService,
}

impl RagService {
    /// Create a new RagService.
    ///
    /// Default embedding model is pinned to UpstaageSolarPassage: all real
    /// chunk_embeddings/document_embeddings rows are already tagged with
    /// this model (verified 2026-09-21, 4320 non-archived rows), because
    /// EMBEDDING_BASE_URL is overridden to Strix Halo's OpenAI-compatible
    /// endpoint, which needs no API key. Selecting OpenaiTextEmbedding3Small
    /// whenever UPSTAGE_API_KEY happened to be unset or empty (the previous
    /// behavior) tags every query embedding with the wrong model, so the
    /// WHERE ce.model = $2 filter in EmbeddingService::semantic_search
    /// silently matches zero rows and RAG search always returns empty.
    pub fn new(pool: PgPool, config: Config) -> Self {
        let openai_key = config
            .openai_api_key
            .as_ref()
            .map(|s| secrecy::ExposeSecret::expose_secret(s).to_string());

        // EmbeddingService::generate_embedding requires Some(_) here for any
        // model where is_upstage() is true (see is_upstage() check below),
        // purely to fail fast when nothing is configured at all. The actual
        // HTTP call is redirected to Strix Halo via EMBEDDING_BASE_URL, which
        // ignores the Authorization header entirely (no auth required), so a
        // placeholder value is safe and keeps the real UPSTAGE_API_KEY (when
        // set) authoritative.
        let upstage_key = std::env::var("UPSTAGE_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(|| Some("unused-strix-endpoint-ignores-this".to_string()));
        let default_model = EmbeddingModel::UpstaageSolarPassage;
        let embedding_config = EmbeddingConfig {
            openai_api_key: openai_key,
            voyage_api_key: None,
            upstage_api_key: upstage_key,
            default_model,
            chunk_size: 512,
            chunk_overlap: 50,
            local_embedding_enabled: config.local_embedding_enabled,
        };

        let embedding_service = EmbeddingService::new(pool.clone(), embedding_config);

        Self {
            pool,
            embedding_service,
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    /// Full RAG pipeline: retrieve relevant chunks, then generate an answer
    /// with Strix Halo using those chunks as context.
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

        // 4. Generate answer with Strix Halo
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

    /// Call Strix Halo (local llama.cpp, OpenAI-compatible) to generate an
    /// answer grounded in the provided context. Returns
    /// `(answer, tokens_used, model_name)`.
    ///
    /// No API key is required — the endpoint is reachable only from the LAN
    /// (Tailscale/CT network). Override the target with `LLM_BASE_URL`.
    async fn generate_answer(
        &self,
        question: &str,
        context: &str,
    ) -> AppResult<(String, u32, String)> {
        let base_url = std::env::var("LLM_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_LLM_BASE_URL.to_string());

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

        let request = LocalLlmRequest {
            model: DEFAULT_LLM_MODEL.to_string(),
            messages: vec![
                LocalLlmMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                LocalLlmMessage {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
            max_tokens: 1024,
            temperature: Some(0.3),
            stream: None,
        };

        let http_client = reqwest::Client::new();

        let response = http_client
            .post(&base_url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Strix LLM request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Strix LLM error: {error_text}"
            )));
        }

        let result: LocalLlmResponse = response
            .json()
            .await
            .map_err(|e| {
                AppError::ExternalService(format!("Failed to parse Strix LLM response: {e}"))
            })?;

        let answer = result.first_text().unwrap_or_default().to_string();

        let tokens_used = result
            .usage
            .as_ref()
            .map(|u| u.total_tokens)
            .unwrap_or(0);
        let model = result
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_LLM_MODEL.to_string());

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

    #[test]
    fn test_build_context_untitled_document_shows_fallback() {
        let chunk = SemanticSearchResult {
            document_id: Uuid::new_v4(),
            chunk_id: None,
            chunk_text: Some("some text".to_string()),
            similarity: 0.75,
            document_title: None, // no title
        };
        let result = build_context_for_test(&[chunk]);
        assert!(result.contains("Untitled document"), "Missing title should show fallback");
    }

    #[test]
    fn test_build_context_chunk_with_no_text_shows_empty_body() {
        let chunk = SemanticSearchResult {
            document_id: Uuid::new_v4(),
            chunk_id: None,
            chunk_text: None, // no text
            similarity: 0.80,
            document_title: Some("My Doc".to_string()),
        };
        let result = build_context_for_test(&[chunk]);
        // Should not panic and should still contain the source reference
        assert!(result.contains("[Source 1]"), "Source label should still appear");
    }

    #[test]
    fn test_default_llm_base_url_is_strix() {
        assert!(DEFAULT_LLM_BASE_URL.contains("100.64.0.15:8093"));
        assert!(DEFAULT_LLM_BASE_URL.contains("/v1/chat/completions"));
    }

    #[test]
    fn test_default_llm_model_carries_ct211_mount_prefix() {
        // Regression (2026-09-28): CT211 (continuum-router) rejects a model
        // name that doesn't exactly match what it has registered, and it
        // registers this backend's id WITH the `/models/` mount prefix
        // llama.cpp reports (confirmed live via `curl .../v1/models`). This
        // was a silent no-op when this backend hit Strix directly (llama.cpp
        // ignores the request's `model` field), so the bare filename never
        // surfaced as a bug until the CT211 migration.
        assert!(DEFAULT_LLM_MODEL.starts_with("/models/"));
        assert!(DEFAULT_LLM_MODEL.contains("Qwen3.8-27B-UD-Q4_K_XL.gguf"));
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
