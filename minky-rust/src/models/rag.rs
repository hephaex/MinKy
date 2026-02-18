//! RAG (Retrieval-Augmented Generation) models
//!
//! This module provides request/response types for the RAG search API,
//! including natural language question answering backed by vector search
//! and Claude AI generation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Ask (RAG question-answer)
// ---------------------------------------------------------------------------

/// Request to ask a natural-language question against the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagAskRequest {
    /// The natural-language question to answer.
    pub question: String,

    /// Maximum number of context chunks to retrieve (default: 5, max: 20).
    #[serde(default = "default_top_k")]
    pub top_k: usize,

    /// Minimum similarity threshold for retrieved chunks (0.0–1.0, default: 0.7).
    #[serde(default = "default_threshold")]
    pub threshold: f32,

    /// Optionally restrict results to a specific user's documents.
    pub user_id: Option<i32>,

    /// Whether to include the raw retrieved chunks in the response.
    #[serde(default)]
    pub include_sources: bool,
}

fn default_top_k() -> usize {
    5
}

fn default_threshold() -> f32 {
    0.7
}

/// A single source document/chunk that was used to generate the answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSource {
    /// The document UUID.
    pub document_id: Uuid,

    /// Human-readable title of the document (if available).
    pub document_title: Option<String>,

    /// The specific chunk text that was used as context.
    pub chunk_text: String,

    /// Cosine similarity score (0.0–1.0) between the query and this chunk.
    pub similarity: f32,
}

/// Response from a RAG question-answer request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagAskResponse {
    /// The generated answer from Claude.
    pub answer: String,

    /// Source documents/chunks used to generate the answer.
    pub sources: Vec<RagSource>,

    /// Total number of tokens consumed by the LLM call.
    pub tokens_used: u32,

    /// The AI model that generated the answer.
    pub model: String,
}

// ---------------------------------------------------------------------------
// Semantic search (vector-only, no generation)
// ---------------------------------------------------------------------------

/// Request for a pure vector-similarity search without LLM generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSemanticSearchRequest {
    /// The search query text.
    pub query: String,

    /// Maximum number of results (default: 10, max: 50).
    #[serde(default = "default_search_limit")]
    pub limit: i32,

    /// Minimum similarity threshold (default: 0.6).
    #[serde(default = "default_search_threshold")]
    pub threshold: f32,

    /// Optionally restrict results to a specific user's documents.
    pub user_id: Option<i32>,
}

fn default_search_limit() -> i32 {
    10
}

fn default_search_threshold() -> f32 {
    0.6
}

/// A single result from a semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSemanticSearchResult {
    /// The document UUID.
    pub document_id: Uuid,

    /// UUID of the specific chunk (if chunked search was used).
    pub chunk_id: Option<Uuid>,

    /// The matching chunk text.
    pub chunk_text: Option<String>,

    /// Cosine similarity score.
    pub similarity: f32,

    /// Human-readable document title.
    pub document_title: Option<String>,
}

/// Response body for semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSemanticSearchResponse {
    /// Ordered list of matching results (highest similarity first).
    pub results: Vec<RagSemanticSearchResult>,

    /// Total number of results returned.
    pub total: usize,

    /// The query that was searched.
    pub query: String,
}

// ---------------------------------------------------------------------------
// Search history
// ---------------------------------------------------------------------------

/// A persisted record of a past search or RAG query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub id: Uuid,

    /// The original question or query text.
    pub query: String,

    /// The generated answer (null for semantic-only searches).
    pub answer: Option<String>,

    /// Number of source chunks that were retrieved.
    pub source_count: i32,

    /// Tokens consumed (null for semantic-only searches).
    pub tokens_used: Option<u32>,

    /// User who performed the search.
    pub user_id: Option<i32>,

    /// When the search was performed.
    pub created_at: DateTime<Utc>,
}

/// Query parameters for listing search history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryQuery {
    /// Filter by user ID.
    pub user_id: Option<i32>,

    /// Maximum number of entries to return (default: 20, max: 100).
    #[serde(default = "default_history_limit")]
    pub limit: i32,

    /// Offset for pagination (default: 0).
    #[serde(default)]
    pub offset: i32,
}

fn default_history_limit() -> i32 {
    20
}
