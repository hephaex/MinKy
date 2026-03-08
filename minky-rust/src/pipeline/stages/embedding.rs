//! Embedding stage - generates vector embeddings for chunks
//!
//! Supports multiple embedding providers:
//! - OpenAI text-embedding-3-small/large
//! - Voyage AI voyage-large-2/code-2

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::models::EmbeddingModel;
use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

use super::chunking::{Chunk, ChunkedDocument};

/// Embedding provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    /// OpenAI embeddings
    OpenAI {
        model: EmbeddingModel,
        api_key: Option<String>,
    },

    /// Voyage AI embeddings
    Voyage {
        model: EmbeddingModel,
        api_key: Option<String>,
    },

    /// Skip embedding generation (for testing)
    None,
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        Self::OpenAI {
            model: EmbeddingModel::OpenaiTextEmbedding3Small,
            api_key: None,
        }
    }
}

/// Chunk with its embedding vector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedChunk {
    /// Original chunk data
    pub chunk: Chunk,

    /// Embedding vector
    pub embedding: Vec<f32>,

    /// Model used for embedding
    pub model: EmbeddingModel,
}

/// Document with embedded chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedDocument {
    /// Document title
    pub title: String,

    /// Original plain text
    pub plain_text: String,

    /// Original content (for storage)
    pub original_content: String,

    /// MIME type
    pub mime_type: String,

    /// Extracted metadata
    pub metadata: super::metadata::ExtractedMetadata,

    /// Embedded chunks
    pub embedded_chunks: Vec<EmbeddedChunk>,

    /// Document-level embedding (average of chunk embeddings)
    pub document_embedding: Option<Vec<f32>>,

    /// Model used for embeddings
    pub embedding_model: EmbeddingModel,

    /// Headings from parsing
    pub headings: Vec<super::parsing::Heading>,

    /// Links from parsing
    pub links: Vec<super::parsing::Link>,

    /// Code blocks from parsing
    pub code_blocks: Vec<super::parsing::CodeBlock>,

    /// Source information
    pub source_type: String,
    pub source_path: Option<String>,
}

/// Embedding stage - generates vector embeddings
#[derive(Debug, Clone)]
pub struct EmbeddingStage {
    provider: EmbeddingProvider,
    http_client: reqwest::Client,
    batch_size: usize,
}

impl EmbeddingStage {
    /// Create a new embedding stage with the given provider
    pub fn new(provider: EmbeddingProvider) -> Self {
        Self {
            provider,
            http_client: reqwest::Client::new(),
            batch_size: 100, // OpenAI batch limit
        }
    }

    /// Create with OpenAI provider
    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::new(EmbeddingProvider::OpenAI {
            model: EmbeddingModel::OpenaiTextEmbedding3Small,
            api_key: Some(api_key.into()),
        })
    }

    /// Create with OpenAI provider and specific model
    pub fn openai_with_model(api_key: impl Into<String>, model: EmbeddingModel) -> Self {
        Self::new(EmbeddingProvider::OpenAI {
            model,
            api_key: Some(api_key.into()),
        })
    }

    /// Create a no-op stage for testing
    pub fn none() -> Self {
        Self::new(EmbeddingProvider::None)
    }

    /// Generate embedding for a single text
    #[allow(dead_code)]
    async fn generate_embedding(
        &self,
        text: &str,
        model: EmbeddingModel,
        api_key: &str,
    ) -> PipelineResult<Vec<f32>> {
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
            .map_err(|e| PipelineError::ExternalService(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PipelineError::ExternalService(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let response: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to parse response: {}", e)))?;

        response
            .data
            .first()
            .map(|d| d.embedding.clone())
            .ok_or_else(|| PipelineError::ExternalService("No embedding returned".into()))
    }

    /// Generate embeddings for multiple texts (batch)
    async fn generate_embeddings_batch(
        &self,
        texts: &[String],
        model: EmbeddingModel,
        api_key: &str,
    ) -> PipelineResult<Vec<Vec<f32>>> {
        let mut all_embeddings = Vec::with_capacity(texts.len());

        // Process in batches
        for chunk in texts.chunks(self.batch_size) {
            let request = OpenAIBatchEmbeddingRequest {
                input: chunk.to_vec(),
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
                .map_err(|e| PipelineError::ExternalService(format!("OpenAI API error: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(PipelineError::ExternalService(format!(
                    "OpenAI API error: {}",
                    error_text
                )));
            }

            let response: OpenAIEmbeddingResponse = response.json().await.map_err(|e| {
                PipelineError::ExternalService(format!("Failed to parse response: {}", e))
            })?;

            // Sort by index to ensure order matches input
            let mut sorted_data = response.data;
            sorted_data.sort_by_key(|d| d.index);

            for item in sorted_data {
                all_embeddings.push(item.embedding);
            }
        }

        Ok(all_embeddings)
    }

    /// Generate a fake embedding for testing
    fn generate_fake_embedding(&self, _text: &str, model: EmbeddingModel) -> Vec<f32> {
        vec![0.0; model.dimension()]
    }

    /// Compute document embedding as average of chunk embeddings
    fn compute_document_embedding(&self, chunk_embeddings: &[Vec<f32>]) -> Option<Vec<f32>> {
        if chunk_embeddings.is_empty() {
            return None;
        }

        let dim = chunk_embeddings[0].len();
        let mut avg = vec![0.0; dim];
        let count = chunk_embeddings.len() as f32;

        for embedding in chunk_embeddings {
            for (i, val) in embedding.iter().enumerate() {
                avg[i] += val / count;
            }
        }

        Some(avg)
    }
}

impl Default for EmbeddingStage {
    fn default() -> Self {
        Self::none()
    }
}

#[async_trait]
impl PipelineStage<ChunkedDocument, EmbeddedDocument> for EmbeddingStage {
    fn name(&self) -> &'static str {
        "embedding"
    }

    async fn process(
        &self,
        input: ChunkedDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<EmbeddedDocument> {
        let (model, api_key) = match &self.provider {
            EmbeddingProvider::OpenAI { model, api_key } => {
                let key = api_key
                    .clone()
                    .ok_or_else(|| PipelineError::Configuration("OpenAI API key not configured".into()))?;
                (*model, key)
            }
            EmbeddingProvider::Voyage { model, api_key } => {
                let key = api_key
                    .clone()
                    .ok_or_else(|| PipelineError::Configuration("Voyage API key not configured".into()))?;
                (*model, key)
            }
            EmbeddingProvider::None => {
                // Generate fake embeddings for testing
                let model = EmbeddingModel::OpenaiTextEmbedding3Small;
                let embedded_chunks: Vec<EmbeddedChunk> = input
                    .chunks
                    .into_iter()
                    .map(|chunk| {
                        let embedding = self.generate_fake_embedding(&chunk.text, model);
                        EmbeddedChunk {
                            chunk,
                            embedding,
                            model,
                        }
                    })
                    .collect();

                let all_embeddings: Vec<Vec<f32>> =
                    embedded_chunks.iter().map(|c| c.embedding.clone()).collect();
                let document_embedding = self.compute_document_embedding(&all_embeddings);

                context.metrics.add_embeddings(embedded_chunks.len() as u32);

                return Ok(EmbeddedDocument {
                    title: input.title,
                    plain_text: input.plain_text,
                    original_content: input.original_content,
                    mime_type: input.mime_type,
                    metadata: input.metadata,
                    embedded_chunks,
                    document_embedding,
                    embedding_model: model,
                    headings: input.headings,
                    links: input.links,
                    code_blocks: input.code_blocks,
                    source_type: input.source_type,
                    source_path: input.source_path,
                });
            }
        };

        // Generate embeddings for all chunks
        let texts: Vec<String> = input.chunks.iter().map(|c| c.text.clone()).collect();

        if texts.is_empty() {
            return Ok(EmbeddedDocument {
                title: input.title,
                plain_text: input.plain_text,
                original_content: input.original_content,
                mime_type: input.mime_type,
                metadata: input.metadata,
                embedded_chunks: Vec::new(),
                document_embedding: None,
                embedding_model: model,
                headings: input.headings,
                links: input.links,
                code_blocks: input.code_blocks,
                source_type: input.source_type,
                source_path: input.source_path,
            });
        }

        let embeddings = self.generate_embeddings_batch(&texts, model, &api_key).await?;

        // Combine chunks with their embeddings
        let embedded_chunks: Vec<EmbeddedChunk> = input
            .chunks
            .into_iter()
            .zip(embeddings.iter())
            .map(|(chunk, embedding)| EmbeddedChunk {
                chunk,
                embedding: embedding.clone(),
                model,
            })
            .collect();

        // Compute document-level embedding
        let document_embedding = self.compute_document_embedding(&embeddings);

        // Update metrics
        context.metrics.add_embeddings(embedded_chunks.len() as u32);
        context.metrics.increment_api_calls();

        Ok(EmbeddedDocument {
            title: input.title,
            plain_text: input.plain_text,
            original_content: input.original_content,
            mime_type: input.mime_type,
            metadata: input.metadata,
            embedded_chunks,
            document_embedding,
            embedding_model: model,
            headings: input.headings,
            links: input.links,
            code_blocks: input.code_blocks,
            source_type: input.source_type,
            source_path: input.source_path,
        })
    }
}

// OpenAI API types

#[derive(Serialize)]
#[allow(dead_code)]
struct OpenAIEmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Serialize)]
struct OpenAIBatchEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    #[serde(default)]
    index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunked_doc(chunks: Vec<Chunk>) -> ChunkedDocument {
        ChunkedDocument {
            title: "Test".to_string(),
            plain_text: "Test content".to_string(),
            original_content: "Test content".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: super::super::metadata::ExtractedMetadata::default(),
            chunks,
            headings: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            source_type: "test".to_string(),
            source_path: None,
        }
    }

    #[tokio::test]
    async fn test_none_provider() {
        let stage = EmbeddingStage::none();
        let mut context = PipelineContext::new();

        let chunks = vec![
            Chunk {
                index: 0,
                text: "First chunk".to_string(),
                start_offset: 0,
                end_offset: 11,
                token_count: 3,
                metadata: None,
            },
            Chunk {
                index: 1,
                text: "Second chunk".to_string(),
                start_offset: 12,
                end_offset: 24,
                token_count: 3,
                metadata: None,
            },
        ];

        let doc = make_chunked_doc(chunks);
        let result = stage.process(doc, &mut context).await.unwrap();

        assert_eq!(result.embedded_chunks.len(), 2);
        assert_eq!(
            result.embedded_chunks[0].embedding.len(),
            EmbeddingModel::OpenaiTextEmbedding3Small.dimension()
        );
        assert!(result.document_embedding.is_some());
    }

    #[tokio::test]
    async fn test_empty_chunks() {
        let stage = EmbeddingStage::none();
        let mut context = PipelineContext::new();

        let doc = make_chunked_doc(Vec::new());
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(result.embedded_chunks.is_empty());
        assert!(result.document_embedding.is_none());
    }

    #[test]
    fn test_compute_document_embedding() {
        let stage = EmbeddingStage::none();

        let embeddings = vec![vec![1.0, 2.0, 3.0], vec![3.0, 4.0, 5.0]];

        let avg = stage.compute_document_embedding(&embeddings).unwrap();
        assert_eq!(avg.len(), 3);
        assert!((avg[0] - 2.0).abs() < 0.001);
        assert!((avg[1] - 3.0).abs() < 0.001);
        assert!((avg[2] - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_document_embedding_empty() {
        let stage = EmbeddingStage::none();
        let result = stage.compute_document_embedding(&[]);
        assert!(result.is_none());
    }
}
