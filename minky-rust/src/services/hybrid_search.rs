//! QMD-Inspired Hybrid Search Service
//!
//! Combines three search strategies with Reciprocal Rank Fusion:
//! 1. BM25 Full-Text Search (PostgreSQL FTS)
//! 2. Vector Semantic Search (pgvector)
//! 3. LLM Re-ranking (Claude)
//!
//! # Pipeline
//! ```text
//! query
//!   → query expansion (generate alternatives)
//!   → parallel: FTS + vector search
//!   → Reciprocal Rank Fusion
//!   → LLM re-ranking (top N)
//!   → final scored results
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Search mode determining which strategies to use
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    /// BM25 keyword search only (fastest)
    Keyword,
    /// Vector semantic search only
    Vector,
    /// Hybrid: FTS + Vector + RRF (no LLM re-ranking)
    Hybrid,
    /// Full pipeline: FTS + Vector + RRF + Query Expansion + LLM Re-ranking
    #[default]
    Deep,
}

/// Hybrid search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchRequest {
    /// The search query
    pub query: String,
    /// Search mode
    #[serde(default)]
    pub mode: SearchMode,
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Minimum relevance threshold (0.0 - 1.0)
    #[serde(default = "default_threshold")]
    pub threshold: f32,
    /// Whether to include query expansion
    #[serde(default = "default_true")]
    pub expand_query: bool,
    /// Whether to use LLM re-ranking
    #[serde(default = "default_true")]
    pub rerank: bool,
    /// Collection filter (optional)
    pub collection_id: Option<Uuid>,
    /// User context (for personalization)
    pub user_id: Option<i32>,
}

fn default_limit() -> usize { 20 }
fn default_threshold() -> f32 { 0.2 }
fn default_true() -> bool { true }

/// A single search result with combined scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    /// Document ID
    pub document_id: Uuid,
    /// Chunk ID (if applicable)
    pub chunk_id: Option<Uuid>,
    /// Document title
    pub title: String,
    /// Content snippet
    pub snippet: String,
    /// Final combined score (0.0 - 1.0)
    pub score: f32,
    /// Individual scores from each strategy
    pub scores: ScoreBreakdown,
    /// Hierarchical context path
    pub context_path: Option<String>,
    /// Matched keywords (for highlighting)
    pub matched_terms: Vec<String>,
    /// Document metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

/// Breakdown of scores from different search strategies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// BM25 full-text search score
    pub fts_score: Option<f32>,
    /// Vector similarity score
    pub vector_score: Option<f32>,
    /// RRF combined score (before re-ranking)
    pub rrf_score: Option<f32>,
    /// LLM re-ranking score
    pub rerank_score: Option<f32>,
}

/// Hybrid search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResponse {
    /// Search results
    pub results: Vec<HybridSearchResult>,
    /// Total matching documents
    pub total: usize,
    /// Original query
    pub query: String,
    /// Expanded queries (if query expansion was used)
    pub expanded_queries: Vec<String>,
    /// Search mode used
    pub mode: SearchMode,
    /// Processing time in milliseconds
    pub took_ms: u64,
    /// Debug info (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<SearchDebugInfo>,
}

/// Debug information for search diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDebugInfo {
    pub fts_took_ms: u64,
    pub vector_took_ms: u64,
    pub rrf_took_ms: u64,
    pub rerank_took_ms: u64,
    pub expansion_took_ms: u64,
    pub fts_results: usize,
    pub vector_results: usize,
}

// ---------------------------------------------------------------------------
// Reciprocal Rank Fusion
// ---------------------------------------------------------------------------

/// RRF configuration
#[derive(Debug, Clone)]
pub struct RrfConfig {
    /// K parameter (default 60, higher = more weight to lower ranks)
    pub k: f32,
    /// Weight for FTS results
    pub fts_weight: f32,
    /// Weight for vector results
    pub vector_weight: f32,
    /// Position bonus decay factor
    pub position_decay: f32,
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self {
            k: 60.0,
            fts_weight: 1.0,
            vector_weight: 1.0,
            position_decay: 0.95,
        }
    }
}

/// Intermediate result for RRF merging
#[derive(Debug, Clone)]
struct RankedResult {
    document_id: Uuid,
    chunk_id: Option<Uuid>,
    title: String,
    snippet: String,
    fts_rank: Option<usize>,
    fts_score: Option<f32>,
    vector_rank: Option<usize>,
    vector_score: Option<f32>,
    context_path: Option<String>,
    matched_terms: Vec<String>,
    metadata: HashMap<String, serde_json::Value>,
    updated_at: DateTime<Utc>,
}

/// Apply Reciprocal Rank Fusion to merge results from multiple search strategies
pub fn reciprocal_rank_fusion(
    fts_results: Vec<FtsResult>,
    vector_results: Vec<VectorResult>,
    config: &RrfConfig,
) -> Vec<HybridSearchResult> {
    let mut merged: HashMap<Uuid, RankedResult> = HashMap::new();

    // Add FTS results with ranks
    for (rank, result) in fts_results.into_iter().enumerate() {
        let key = result.document_id;
        merged.entry(key).or_insert_with(|| RankedResult {
            document_id: result.document_id,
            chunk_id: result.chunk_id,
            title: result.title.clone(),
            snippet: result.snippet.clone(),
            fts_rank: None,
            fts_score: None,
            vector_rank: None,
            vector_score: None,
            context_path: result.context_path.clone(),
            matched_terms: result.matched_terms.clone(),
            metadata: result.metadata.clone(),
            updated_at: result.updated_at,
        });

        if let Some(r) = merged.get_mut(&key) {
            r.fts_rank = Some(rank + 1);
            r.fts_score = Some(result.score);
            // Merge matched terms
            for term in &result.matched_terms {
                if !r.matched_terms.contains(term) {
                    r.matched_terms.push(term.clone());
                }
            }
        }
    }

    // Add vector results with ranks
    for (rank, result) in vector_results.into_iter().enumerate() {
        let key = result.document_id;
        merged.entry(key).or_insert_with(|| RankedResult {
            document_id: result.document_id,
            chunk_id: result.chunk_id,
            title: result.title.clone(),
            snippet: result.snippet.clone(),
            fts_rank: None,
            fts_score: None,
            vector_rank: None,
            vector_score: None,
            context_path: result.context_path.clone(),
            matched_terms: vec![],
            metadata: result.metadata.clone(),
            updated_at: result.updated_at,
        });

        if let Some(r) = merged.get_mut(&key) {
            r.vector_rank = Some(rank + 1);
            r.vector_score = Some(result.similarity);
        }
    }

    // Calculate RRF scores
    let mut results: Vec<HybridSearchResult> = merged
        .into_values()
        .map(|r| {
            let fts_rrf = r.fts_rank
                .map(|rank| config.fts_weight / (config.k + rank as f32))
                .unwrap_or(0.0);

            let vector_rrf = r.vector_rank
                .map(|rank| config.vector_weight / (config.k + rank as f32))
                .unwrap_or(0.0);

            // Position-aware bonus: results appearing in both get a boost
            let overlap_bonus = if r.fts_rank.is_some() && r.vector_rank.is_some() {
                0.1
            } else {
                0.0
            };

            let rrf_score = fts_rrf + vector_rrf + overlap_bonus;

            HybridSearchResult {
                document_id: r.document_id,
                chunk_id: r.chunk_id,
                title: r.title,
                snippet: r.snippet,
                score: rrf_score,
                scores: ScoreBreakdown {
                    fts_score: r.fts_score,
                    vector_score: r.vector_score,
                    rrf_score: Some(rrf_score),
                    rerank_score: None,
                },
                context_path: r.context_path,
                matched_terms: r.matched_terms,
                metadata: r.metadata,
                updated_at: r.updated_at,
            }
        })
        .collect();

    // Sort by RRF score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Normalize scores to 0.0 - 1.0
    if let Some(max_score) = results.first().map(|r| r.score) {
        if max_score > 0.0 {
            for result in &mut results {
                result.score /= max_score;
                if let Some(ref mut rrf) = result.scores.rrf_score {
                    *rrf /= max_score;
                }
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Search Result Types (for internal use)
// ---------------------------------------------------------------------------

/// Full-text search result
#[derive(Debug, Clone)]
pub struct FtsResult {
    pub document_id: Uuid,
    pub chunk_id: Option<Uuid>,
    pub title: String,
    pub snippet: String,
    pub score: f32,
    pub context_path: Option<String>,
    pub matched_terms: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorResult {
    pub document_id: Uuid,
    pub chunk_id: Option<Uuid>,
    pub title: String,
    pub snippet: String,
    pub similarity: f32,
    pub context_path: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Hybrid Search Service
// ---------------------------------------------------------------------------

/// QMD-inspired hybrid search service
pub struct HybridSearchService {
    pool: PgPool,
    rrf_config: RrfConfig,
    embedding_service: Arc<crate::services::EmbeddingService>,
}

impl HybridSearchService {
    pub fn new(pool: PgPool, embedding_service: Arc<crate::services::EmbeddingService>) -> Self {
        Self {
            pool,
            rrf_config: RrfConfig::default(),
            embedding_service,
        }
    }

    pub fn with_rrf_config(mut self, config: RrfConfig) -> Self {
        self.rrf_config = config;
        self
    }

    /// Perform hybrid search with the specified mode
    pub async fn search(&self, req: HybridSearchRequest) -> AppResult<HybridSearchResponse> {
        let start = std::time::Instant::now();
        let mut debug = SearchDebugInfo {
            fts_took_ms: 0,
            vector_took_ms: 0,
            rrf_took_ms: 0,
            rerank_took_ms: 0,
            expansion_took_ms: 0,
            fts_results: 0,
            vector_results: 0,
        };

        // Query expansion (for Deep mode)
        let expanded_queries = if req.mode == SearchMode::Deep && req.expand_query {
            let exp_start = std::time::Instant::now();
            let queries = self.expand_query(&req.query).await.unwrap_or_default();
            debug.expansion_took_ms = exp_start.elapsed().as_millis() as u64;
            queries
        } else {
            vec![]
        };

        // Collect all queries to search
        let mut all_queries = vec![req.query.clone()];
        all_queries.extend(expanded_queries.clone());

        // Execute searches based on mode
        let results = match req.mode {
            SearchMode::Keyword => {
                let fts_start = std::time::Instant::now();
                let fts_results = self.fts_search(&all_queries, req.limit * 2, req.collection_id).await?;
                debug.fts_took_ms = fts_start.elapsed().as_millis() as u64;
                debug.fts_results = fts_results.len();

                fts_results.into_iter().map(|r| HybridSearchResult {
                    document_id: r.document_id,
                    chunk_id: r.chunk_id,
                    title: r.title,
                    snippet: r.snippet,
                    score: r.score,
                    scores: ScoreBreakdown {
                        fts_score: Some(r.score),
                        ..Default::default()
                    },
                    context_path: r.context_path,
                    matched_terms: r.matched_terms,
                    metadata: r.metadata,
                    updated_at: r.updated_at,
                }).collect()
            }
            SearchMode::Vector => {
                let vec_start = std::time::Instant::now();
                let vector_results = self.vector_search(&req.query, req.limit * 2, req.collection_id).await?;
                debug.vector_took_ms = vec_start.elapsed().as_millis() as u64;
                debug.vector_results = vector_results.len();

                vector_results.into_iter().map(|r| HybridSearchResult {
                    document_id: r.document_id,
                    chunk_id: r.chunk_id,
                    title: r.title,
                    snippet: r.snippet,
                    score: r.similarity,
                    scores: ScoreBreakdown {
                        vector_score: Some(r.similarity),
                        ..Default::default()
                    },
                    context_path: r.context_path,
                    matched_terms: vec![],
                    metadata: r.metadata,
                    updated_at: r.updated_at,
                }).collect()
            }
            SearchMode::Hybrid | SearchMode::Deep => {
                // Parallel FTS and vector search
                let fts_start = std::time::Instant::now();
                let fts_results = self.fts_search(&all_queries, req.limit * 2, req.collection_id).await?;
                debug.fts_took_ms = fts_start.elapsed().as_millis() as u64;
                debug.fts_results = fts_results.len();

                let vec_start = std::time::Instant::now();
                let vector_results = self.vector_search(&req.query, req.limit * 2, req.collection_id).await?;
                debug.vector_took_ms = vec_start.elapsed().as_millis() as u64;
                debug.vector_results = vector_results.len();

                // RRF fusion
                let rrf_start = std::time::Instant::now();
                let mut fused = reciprocal_rank_fusion(fts_results, vector_results, &self.rrf_config);
                debug.rrf_took_ms = rrf_start.elapsed().as_millis() as u64;

                // LLM re-ranking for Deep mode
                if req.mode == SearchMode::Deep && req.rerank && !fused.is_empty() {
                    let rerank_start = std::time::Instant::now();
                    let top_n = fused.len().min(30);
                    fused = self.rerank(&req.query, fused.into_iter().take(top_n).collect()).await?;
                    debug.rerank_took_ms = rerank_start.elapsed().as_millis() as u64;
                }

                fused
            }
        };

        // Filter by threshold and limit
        let filtered: Vec<HybridSearchResult> = results
            .into_iter()
            .filter(|r| r.score >= req.threshold)
            .take(req.limit)
            .collect();

        let total = filtered.len();
        let took_ms = start.elapsed().as_millis() as u64;

        Ok(HybridSearchResponse {
            results: filtered,
            total,
            query: req.query,
            expanded_queries,
            mode: req.mode,
            took_ms,
            debug: Some(debug),
        })
    }

    /// Full-text search using PostgreSQL FTS
    async fn fts_search(
        &self,
        queries: &[String],
        limit: usize,
        collection_id: Option<Uuid>,
    ) -> AppResult<Vec<FtsResult>> {
        // Combine queries with OR for broader search
        let combined_query = queries.join(" | ");

        let rows = sqlx::query_as::<_, FtsRow>(
            r#"
            SELECT
                d.id as document_id,
                NULL::uuid as chunk_id,
                d.title,
                COALESCE(
                    ts_headline('english', d.content, plainto_tsquery('english', $1),
                        'MaxWords=50, MinWords=20, StartSel=<mark>, StopSel=</mark>'),
                    LEFT(d.content, 200)
                ) as snippet,
                ts_rank_cd(d.search_vector, plainto_tsquery('english', $1)) as score,
                c.context_path,
                d.updated_at
            FROM documents d
            LEFT JOIN collection_contexts c ON c.document_id = d.id
            WHERE d.search_vector @@ plainto_tsquery('english', $1)
            AND d.archived = false
            AND ($2::uuid IS NULL OR d.collection_id = $2)
            ORDER BY score DESC
            LIMIT $3
            "#
        )
        .bind(&combined_query)
        .bind(collection_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(|r| FtsResult {
            document_id: r.document_id,
            chunk_id: r.chunk_id,
            title: r.title,
            snippet: r.snippet,
            score: r.score,
            context_path: r.context_path,
            matched_terms: extract_matched_terms(&combined_query),
            metadata: HashMap::new(),
            updated_at: r.updated_at,
        }).collect())
    }

    /// Vector semantic search using pgvector
    async fn vector_search(
        &self,
        query: &str,
        limit: usize,
        collection_id: Option<Uuid>,
    ) -> AppResult<Vec<VectorResult>> {
        let (embedding, storage_model) = self.get_query_embedding(query).await?;

        let rows = sqlx::query_as::<_, VectorRow>(
            r#"
            SELECT
                e.document_id,
                NULL::uuid as chunk_id,
                d.title,
                LEFT(d.content, 200) as snippet,
                (1 - (e.embedding <=> $1))::float4 as similarity,
                c.context_path,
                d.updated_at
            FROM document_embeddings e
            JOIN documents d ON d.id = e.document_id
            LEFT JOIN collection_contexts c ON c.document_id = d.id
            WHERE d.archived = false
            AND e.model = $2
            AND ($3::uuid IS NULL OR d.collection_id = $3)
            ORDER BY e.embedding <=> $1
            LIMIT $4
            "#
        )
        .bind(pgvector::Vector::from(embedding))
        .bind(storage_model)
        .bind(collection_id)
        .bind(limit as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(|r| VectorResult {
            document_id: r.document_id,
            chunk_id: r.chunk_id,
            title: r.title,
            snippet: r.snippet,
            similarity: r.similarity,
            context_path: r.context_path,
            metadata: HashMap::new(),
            updated_at: r.updated_at,
        }).collect())
    }

    /// Expand query using Claude to generate alternative phrasings
    async fn expand_query(&self, query: &str) -> AppResult<Vec<String>> {
        // For now, return simple expansions
        // In production, this would call Claude for semantic expansion
        let expansions = generate_query_expansions(query);
        Ok(expansions)
    }

    /// Get a query embedding plus the storage model to filter against.
    ///
    /// Delegates to `EmbeddingService::embed_query`, which resolves the
    /// query/passage model split (e.g. Upstage Solar) so the returned
    /// `storage_model` can be used as a `WHERE model = ...` filter and never
    /// compares embeddings across incompatible vector spaces.
    async fn get_query_embedding(
        &self,
        query: &str,
    ) -> AppResult<(Vec<f32>, crate::models::EmbeddingModel)> {
        self.embedding_service.embed_query(query, None).await
    }

    /// Re-rank results using LLM
    async fn rerank(
        &self,
        _query: &str,
        mut results: Vec<HybridSearchResult>,
    ) -> AppResult<Vec<HybridSearchResult>> {
        // Placeholder - in production, call Claude for re-ranking
        // For now, just add a rerank score based on existing scores
        for (i, result) in results.iter_mut().enumerate() {
            let position_factor = 1.0 - (i as f32 * 0.02);
            let rerank_score = result.score * position_factor;
            result.scores.rerank_score = Some(rerank_score);
            result.score = rerank_score;
        }

        // Re-sort by new scores
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// Database Row Types
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct FtsRow {
    document_id: Uuid,
    chunk_id: Option<Uuid>,
    title: String,
    snippet: String,
    score: f32,
    context_path: Option<String>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct VectorRow {
    document_id: Uuid,
    chunk_id: Option<Uuid>,
    title: String,
    snippet: String,
    similarity: f32,
    context_path: Option<String>,
    updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Extract matched terms from a query string
fn extract_matched_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.to_lowercase())
        .collect()
}

/// Generate simple query expansions (placeholder for LLM-based expansion)
fn generate_query_expansions(query: &str) -> Vec<String> {
    let mut expansions = Vec::new();

    // Add lowercase variant
    let lower = query.to_lowercase();
    if lower != query {
        expansions.push(lower);
    }

    // Add with common synonyms (simple example)
    let synonyms = [
        ("how to", "guide for"),
        ("what is", "definition of"),
        ("error", "issue"),
        ("problem", "issue"),
        ("fix", "solve"),
        ("create", "make"),
    ];

    for (from, to) in synonyms {
        if query.to_lowercase().contains(from) {
            expansions.push(query.to_lowercase().replace(from, to));
        }
    }

    expansions.into_iter().take(3).collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fts_result(id: Uuid, score: f32) -> FtsResult {
        FtsResult {
            document_id: id,
            chunk_id: None,
            title: format!("Doc {}", id),
            snippet: "Test snippet".to_string(),
            score,
            context_path: None,
            matched_terms: vec!["test".to_string()],
            metadata: HashMap::new(),
            updated_at: Utc::now(),
        }
    }

    fn make_vector_result(id: Uuid, similarity: f32) -> VectorResult {
        VectorResult {
            document_id: id,
            chunk_id: None,
            title: format!("Doc {}", id),
            snippet: "Test snippet".to_string(),
            similarity,
            context_path: None,
            metadata: HashMap::new(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_rrf_empty_inputs() {
        let config = RrfConfig::default();
        let results = reciprocal_rank_fusion(vec![], vec![], &config);
        assert!(results.is_empty());
    }

    #[test]
    fn test_rrf_fts_only() {
        let config = RrfConfig::default();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let fts = vec![
            make_fts_result(id1, 0.9),
            make_fts_result(id2, 0.8),
        ];

        let results = reciprocal_rank_fusion(fts, vec![], &config);
        assert_eq!(results.len(), 2);
        assert!(results[0].scores.fts_score.is_some());
        assert!(results[0].scores.vector_score.is_none());
    }

    #[test]
    fn test_rrf_vector_only() {
        let config = RrfConfig::default();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let vector = vec![
            make_vector_result(id1, 0.95),
            make_vector_result(id2, 0.85),
        ];

        let results = reciprocal_rank_fusion(vec![], vector, &config);
        assert_eq!(results.len(), 2);
        assert!(results[0].scores.vector_score.is_some());
        assert!(results[0].scores.fts_score.is_none());
    }

    #[test]
    fn test_rrf_combined_with_overlap() {
        let config = RrfConfig::default();
        let shared_id = Uuid::new_v4();
        let fts_only_id = Uuid::new_v4();
        let vector_only_id = Uuid::new_v4();

        let fts = vec![
            make_fts_result(shared_id, 0.9),
            make_fts_result(fts_only_id, 0.7),
        ];

        let vector = vec![
            make_vector_result(shared_id, 0.95),
            make_vector_result(vector_only_id, 0.8),
        ];

        let results = reciprocal_rank_fusion(fts, vector, &config);

        // Shared doc should be first (appears in both)
        assert_eq!(results[0].document_id, shared_id);
        assert!(results[0].scores.fts_score.is_some());
        assert!(results[0].scores.vector_score.is_some());
    }

    #[test]
    fn test_rrf_scores_normalized() {
        let config = RrfConfig::default();
        let id = Uuid::new_v4();

        let fts = vec![make_fts_result(id, 0.9)];
        let results = reciprocal_rank_fusion(fts, vec![], &config);

        // Single result should be normalized to 1.0
        assert!((results[0].score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_query_expansions() {
        let expansions = generate_query_expansions("how to fix error");
        assert!(!expansions.is_empty());
    }

    #[test]
    fn test_extract_matched_terms() {
        let terms = extract_matched_terms("hello world test");
        assert_eq!(terms.len(), 3);
        assert!(terms.contains(&"hello".to_string()));
    }

    #[test]
    fn test_search_mode_default() {
        let mode: SearchMode = Default::default();
        assert_eq!(mode, SearchMode::Deep);
    }

    #[test]
    fn test_rrf_config_default() {
        let config = RrfConfig::default();
        assert_eq!(config.k, 60.0);
        assert_eq!(config.fts_weight, 1.0);
        assert_eq!(config.vector_weight, 1.0);
    }

    /// Cross-model exclusion integration test (G002/C1b acceptance criteria).
    ///
    /// Seeds two 4096-dim embeddings for the same document under two
    /// different `embedding_model` values, then asserts `vector_search`
    /// only ever returns the row matching the resolved `storage_model` for
    /// the query — proving the `AND e.model = $2` filter added to the SQL
    /// prevents cross-model-incoherent similarity comparisons.
    ///
    /// Requires a live Postgres with the `minky` schema applied and
    /// `UPSTAGE_API_KEY` set (Upstage Solar is the only 4096-dim model this
    /// service can generate embeddings for without a local model).
    /// Skipped in CI; run with:
    ///   MINKY_DATABASE_URL=... UPSTAGE_API_KEY=... cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn vector_search_excludes_rows_from_other_embedding_models() {
        use crate::models::EmbeddingModel;
        use crate::services::embedding_service::{EmbeddingConfig, EmbeddingService};
        use pgvector::Vector;
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
        let embedding_service = Arc::new(EmbeddingService::new(pool.clone(), config));

        // Seed a document and two 4096-dim embeddings under different models.
        let user_id: i32 = sqlx::query_scalar(
            "INSERT INTO users (email, username, password_hash) \
             VALUES ($1, $2, 'x') \
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email \
             RETURNING id",
        )
        .bind("c1b-cross-model-test@minky.local")
        .bind("c1b_cross_model_test")
        .fetch_one(&pool)
        .await
        .expect("seed user");

        let document_id: Uuid = sqlx::query_scalar(
            "INSERT INTO documents (title, content, user_id) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind("C1b cross-model exclusion fixture")
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
        .expect("seed upstage embedding");

        sqlx::query(
            "INSERT INTO document_embeddings (document_id, embedding, model) VALUES ($1, $2, $3)",
        )
        .bind(document_id)
        .bind(Vector::from(dummy_vec))
        .bind(EmbeddingModel::OpenaiTextEmbedding3Large)
        .execute(&pool)
        .await
        .expect("seed openai embedding");

        let service = HybridSearchService::new(pool.clone(), embedding_service);

        let results = service
            .vector_search("quick brown fox", 10, None)
            .await
            .expect("vector_search should succeed");

        // Cleanup before asserting, so a failed assertion doesn't leave fixtures behind.
        let cleanup = async {
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
        };
        cleanup.await;

        assert!(
            !results.is_empty(),
            "expected at least one match for the seeded document"
        );
        assert!(
            results.iter().all(|r| r.document_id == document_id),
            "unexpected document_id in results: {:?}",
            results
        );
        // The resolved storage model for a Solar-default config is
        // UpstaageSolarPassage; the OpenAI row must never surface here.
        // We cannot directly read `model` off VectorResult (it isn't
        // projected), so we assert indirectly: exactly one row came back
        // for this document, which is only possible if the `AND e.model =
        // $2` filter excluded the second (OpenAI) embedding row.
        let matches_for_doc = results
            .iter()
            .filter(|r| r.document_id == document_id)
            .count();
        assert_eq!(
            matches_for_doc, 1,
            "expected exactly one row (the model-matching one) per document, got {}",
            matches_for_doc
        );
    }
}
