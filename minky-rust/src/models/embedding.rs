//! Embedding models for vector search and RAG
//!
//! This module provides types for storing and querying vector embeddings
//! using pgvector for semantic search capabilities.

use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Embedding model types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "embedding_model", rename_all = "snake_case")]
#[derive(Default)]
pub enum EmbeddingModel {
    #[serde(rename = "openai_text_embedding_3_small")]
    #[default]
    OpenaiTextEmbedding3Small,
    #[serde(rename = "openai_text_embedding_3_large")]
    OpenaiTextEmbedding3Large,
    #[serde(rename = "voyage_large_2")]
    VoyageLarge2,
    #[serde(rename = "voyage_code_2")]
    VoyageCode2,
}


impl EmbeddingModel {
    /// Get the dimension size for this embedding model
    pub fn dimension(&self) -> usize {
        match self {
            Self::OpenaiTextEmbedding3Small => 1536,
            Self::OpenaiTextEmbedding3Large => 3072,
            Self::VoyageLarge2 => 1536,
            Self::VoyageCode2 => 1536,
        }
    }

    /// Get the model identifier for API calls
    pub fn api_model_id(&self) -> &'static str {
        match self {
            Self::OpenaiTextEmbedding3Small => "text-embedding-3-small",
            Self::OpenaiTextEmbedding3Large => "text-embedding-3-large",
            Self::VoyageLarge2 => "voyage-large-2",
            Self::VoyageCode2 => "voyage-code-2",
        }
    }
}

/// Document-level embedding stored in the database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DocumentEmbedding {
    pub id: Uuid,
    pub document_id: Uuid,
    #[sqlx(try_from = "Vec<f32>")]
    pub embedding: Option<Vec<f32>>,
    pub model: EmbeddingModel,
    pub token_count: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Chunk-level embedding for RAG
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChunkEmbedding {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub chunk_start_offset: i32,
    pub chunk_end_offset: i32,
    #[sqlx(try_from = "Vec<f32>")]
    pub embedding: Option<Vec<f32>>,
    pub model: EmbeddingModel,
    pub token_count: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// AI-analyzed document understanding
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DocumentUnderstanding {
    pub id: Uuid,
    pub document_id: Uuid,
    pub topics: Vec<String>,
    pub summary: Option<String>,
    pub problem_solved: Option<String>,
    pub insights: Vec<String>,
    pub technologies: Vec<String>,
    pub relevant_for: Vec<String>,
    pub related_document_ids: Vec<Uuid>,
    pub analyzed_at: DateTime<Utc>,
    pub analyzer_model: Option<String>,
}

/// Queue entry for async embedding generation
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct EmbeddingQueueEntry {
    pub id: Uuid,
    pub document_id: Uuid,
    pub status: String,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Request to create a document embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentEmbeddingRequest {
    pub document_id: Uuid,
    pub model: Option<EmbeddingModel>,
}

/// Request to create chunk embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChunkEmbeddingsRequest {
    pub document_id: Uuid,
    pub chunks: Vec<ChunkData>,
    pub model: Option<EmbeddingModel>,
}

/// Individual chunk data for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub text: String,
    pub start_offset: i32,
    pub end_offset: i32,
    pub metadata: Option<serde_json::Value>,
}

/// Request for semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub threshold: Option<f32>,
    pub model: Option<EmbeddingModel>,
    pub user_id: Option<i32>,
}

fn default_limit() -> i32 {
    10
}

/// Result of semantic search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub document_id: Uuid,
    pub chunk_id: Option<Uuid>,
    pub chunk_text: Option<String>,
    pub similarity: f32,
    pub document_title: Option<String>,
}

/// Request for document understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUnderstandingRequest {
    pub document_id: Uuid,
    pub content: String,
    pub title: Option<String>,
}

/// Response for document understanding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUnderstandingResponse {
    pub topics: Vec<String>,
    pub summary: String,
    pub problem_solved: Option<String>,
    pub insights: Vec<String>,
    pub technologies: Vec<String>,
    pub relevant_for: Vec<String>,
}

/// Embedding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingStats {
    pub total_documents: i64,
    pub documents_with_embeddings: i64,
    pub total_chunks: i64,
    pub pending_queue: i64,
    pub failed_queue: i64,
}

/// Vector operations helper
pub struct VectorOps;

impl VectorOps {
    /// Create a pgvector Vector from f32 slice
    pub fn from_slice(data: &[f32]) -> Vector {
        Vector::from(data.to_vec())
    }

    /// Calculate cosine similarity between two vectors
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_model_dimension() {
        assert_eq!(EmbeddingModel::OpenaiTextEmbedding3Small.dimension(), 1536);
        assert_eq!(EmbeddingModel::OpenaiTextEmbedding3Large.dimension(), 3072);
        assert_eq!(EmbeddingModel::VoyageLarge2.dimension(), 1536);
        assert_eq!(EmbeddingModel::VoyageCode2.dimension(), 1536);
    }

    #[test]
    fn test_embedding_model_default() {
        assert_eq!(EmbeddingModel::default(), EmbeddingModel::OpenaiTextEmbedding3Small);
    }

    #[test]
    fn test_embedding_model_api_id() {
        assert_eq!(
            EmbeddingModel::OpenaiTextEmbedding3Small.api_model_id(),
            "text-embedding-3-small"
        );
        assert_eq!(
            EmbeddingModel::OpenaiTextEmbedding3Large.api_model_id(),
            "text-embedding-3-large"
        );
        assert_eq!(EmbeddingModel::VoyageLarge2.api_model_id(), "voyage-large-2");
        assert_eq!(EmbeddingModel::VoyageCode2.api_model_id(), "voyage-code-2");
    }

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorOps::cosine_similarity(&a, &b) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        assert!((VectorOps::cosine_similarity(&a, &c) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        assert!((VectorOps::cosine_similarity(&a, &b) - (-1.0)).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(VectorOps::cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(VectorOps::cosine_similarity(&a, &b), 0.0);
    }
}
