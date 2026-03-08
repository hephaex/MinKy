//! Pipeline builder for composing document processing stages
//!
//! Provides a fluent API for constructing document processing pipelines.

use sqlx::PgPool;

use crate::config::Config;
use crate::models::EmbeddingModel;

use super::context::PipelineContext;
use super::error::PipelineResult;
use super::stage::StageExecutor;
use super::stages::*;

/// Configuration for building a document pipeline
#[derive(Clone)]
pub struct PipelineConfig {
    /// Database pool
    pub pool: Option<PgPool>,

    /// Application config
    pub app_config: Option<Config>,

    /// Chunking strategy
    pub chunking_strategy: ChunkingStrategy,

    /// Embedding provider
    pub embedding_provider: EmbeddingProvider,

    /// OpenAI API key for embeddings
    pub openai_api_key: Option<String>,

    /// Anthropic API key for analysis
    pub anthropic_api_key: Option<String>,

    /// Whether to skip AI analysis
    pub skip_analysis: bool,

    /// Whether to skip embedding generation
    pub skip_embedding: bool,

    /// Whether to skip storage
    pub skip_storage: bool,

    /// User ID for document ownership
    pub user_id: Option<i32>,

    /// Whether documents should be public
    pub is_public: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            pool: None,
            app_config: None,
            chunking_strategy: ChunkingStrategy::default(),
            embedding_provider: EmbeddingProvider::None,
            openai_api_key: None,
            anthropic_api_key: None,
            skip_analysis: false,
            skip_embedding: false,
            skip_storage: false,
            user_id: None,
            is_public: false,
        }
    }
}

/// Document processing pipeline
///
/// Processes documents through multiple stages: ingestion, parsing,
/// metadata extraction, chunking, embedding, AI analysis, storage, and indexing.
#[derive(Clone)]
pub struct DocumentPipeline {
    config: PipelineConfig,
}

impl DocumentPipeline {
    /// Create a new pipeline builder
    pub fn builder() -> DocumentPipelineBuilder {
        DocumentPipelineBuilder::new()
    }

    /// Create a minimal pipeline for testing (no external APIs)
    pub fn minimal() -> Self {
        DocumentPipelineBuilder::new()
            .skip_embedding()
            .skip_analysis()
            .skip_storage()
            .build()
    }

    /// Process a document through the full pipeline
    pub async fn process(&self, input: IngestionInput) -> PipelineResult<PipelineOutput> {
        let mut context = PipelineContext::new();

        if let Some(user_id) = self.config.user_id {
            context.user_id = Some(user_id);
        }

        // Stage 1: Ingestion
        let ingestion_stage = IngestionStage::new();
        let raw_doc = StageExecutor::execute(&ingestion_stage, input, &mut context).await?;

        // Stage 2: Parsing
        let parsing_stage = ParsingStage::new();
        let parsed_doc = StageExecutor::execute(&parsing_stage, raw_doc, &mut context).await?;

        // Stage 3: Metadata Extraction
        let metadata_stage = MetadataExtractionStage::new();
        let doc_with_metadata = StageExecutor::execute(&metadata_stage, parsed_doc, &mut context).await?;

        // Stage 4: Chunking
        let chunking_stage = ChunkingStage::new(self.config.chunking_strategy.clone());
        let chunked_doc = StageExecutor::execute(&chunking_stage, doc_with_metadata, &mut context).await?;

        // Stage 5: Embedding
        let embedding_stage = if self.config.skip_embedding {
            EmbeddingStage::none()
        } else {
            match &self.config.embedding_provider {
                EmbeddingProvider::OpenAI { model, api_key } => {
                    let key = api_key
                        .clone()
                        .or_else(|| self.config.openai_api_key.clone())
                        .unwrap_or_default();
                    EmbeddingStage::openai_with_model(key, *model)
                }
                EmbeddingProvider::Voyage { model, api_key } => {
                    let key = api_key.clone().unwrap_or_default();
                    EmbeddingStage::new(EmbeddingProvider::Voyage {
                        model: *model,
                        api_key: Some(key),
                    })
                }
                EmbeddingProvider::None => EmbeddingStage::none(),
            }
        };
        let embedded_doc = StageExecutor::execute(&embedding_stage, chunked_doc, &mut context).await?;

        // Stage 6: AI Analysis
        let analysis_stage = if self.config.skip_analysis {
            AIAnalysisStage::skip()
        } else if let Some(ref api_key) = self.config.anthropic_api_key {
            AIAnalysisStage::with_api_key(api_key)
        } else {
            AIAnalysisStage::skip()
        };
        let analyzed_doc = StageExecutor::execute(&analysis_stage, embedded_doc, &mut context).await?;

        // Stages 7 & 8: Storage and Indexing (require database)
        if self.config.skip_storage {
            // Return a mock output without storage
            return Ok(PipelineOutput {
                document_id: context.job_id, // Use job ID as document ID
                title: analyzed_doc.title,
                chunks_count: analyzed_doc.embedded_chunks.len(),
                analyzed: analyzed_doc.analysis.is_some(),
                topics: analyzed_doc
                    .analysis
                    .as_ref()
                    .map(|a| a.topics.clone())
                    .unwrap_or_default(),
                summary: analyzed_doc.analysis.as_ref().map(|a| a.summary.clone()),
            });
        }

        let pool = self.config.pool.clone().ok_or_else(|| {
            super::error::PipelineError::Configuration("Database pool not configured".into())
        })?;

        // Stage 7: Storage
        let storage_config = super::stages::storage::StorageConfig {
            user_id: self.config.user_id,
            is_public: self.config.is_public,
            ..Default::default()
        };
        let storage_stage = StorageStage::new(pool.clone(), storage_config);
        let stored_doc = StageExecutor::execute(&storage_stage, analyzed_doc, &mut context).await?;

        // Stage 8: Indexing
        let indexing_stage = IndexingStage::with_pool(pool);
        let output = StageExecutor::execute(&indexing_stage, stored_doc, &mut context).await?;

        Ok(output)
    }

    /// Process a document from text content
    pub async fn process_text(
        &self,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> PipelineResult<PipelineOutput> {
        let input = IngestionInput::from_text(title, content);
        self.process(input).await
    }

    /// Process a document from a file path
    pub async fn process_file(&self, path: impl Into<String>) -> PipelineResult<PipelineOutput> {
        let input = IngestionInput::from_file(path);
        self.process(input).await
    }

    /// Get the pipeline configuration
    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }
}

/// Builder for creating document pipelines
#[derive(Clone, Default)]
pub struct DocumentPipelineBuilder {
    config: PipelineConfig,
}

impl DocumentPipelineBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: PipelineConfig::default(),
        }
    }

    /// Set the database pool
    pub fn pool(mut self, pool: PgPool) -> Self {
        self.config.pool = Some(pool);
        self
    }

    /// Set the application config
    pub fn app_config(mut self, config: Config) -> Self {
        // Extract API keys from config
        if let Some(ref key) = config.openai_api_key {
            use secrecy::ExposeSecret;
            self.config.openai_api_key = Some(key.expose_secret().to_string());
        }
        if let Some(ref key) = config.anthropic_api_key {
            use secrecy::ExposeSecret;
            self.config.anthropic_api_key = Some(key.expose_secret().to_string());
        }
        self.config.app_config = Some(config);
        self
    }

    /// Set chunking strategy
    pub fn chunking(mut self, strategy: ChunkingStrategy) -> Self {
        self.config.chunking_strategy = strategy;
        self
    }

    /// Use semantic chunking with specified max tokens
    pub fn semantic_chunking(mut self, max_tokens: usize) -> Self {
        self.config.chunking_strategy = ChunkingStrategy::Semantic {
            max_tokens,
            respect_boundaries: true,
        };
        self
    }

    /// Use fixed size chunking
    pub fn fixed_chunking(mut self, size: usize, overlap: usize) -> Self {
        self.config.chunking_strategy = ChunkingStrategy::FixedSize { size, overlap };
        self
    }

    /// Set embedding provider
    pub fn embedding(mut self, provider: EmbeddingProvider) -> Self {
        self.config.embedding_provider = provider;
        self
    }

    /// Use OpenAI embeddings
    pub fn openai_embedding(mut self, api_key: impl Into<String>) -> Self {
        self.config.openai_api_key = Some(api_key.into());
        self.config.embedding_provider = EmbeddingProvider::OpenAI {
            model: EmbeddingModel::OpenaiTextEmbedding3Small,
            api_key: self.config.openai_api_key.clone(),
        };
        self
    }

    /// Use OpenAI embeddings with specific model
    pub fn openai_embedding_with_model(
        mut self,
        api_key: impl Into<String>,
        model: EmbeddingModel,
    ) -> Self {
        let key = api_key.into();
        self.config.openai_api_key = Some(key.clone());
        self.config.embedding_provider = EmbeddingProvider::OpenAI {
            model,
            api_key: Some(key),
        };
        self
    }

    /// Set Anthropic API key for analysis
    pub fn anthropic_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.config.anthropic_api_key = Some(api_key.into());
        self
    }

    /// Skip embedding generation
    pub fn skip_embedding(mut self) -> Self {
        self.config.skip_embedding = true;
        self
    }

    /// Skip AI analysis
    pub fn skip_analysis(mut self) -> Self {
        self.config.skip_analysis = true;
        self
    }

    /// Skip storage (useful for testing)
    pub fn skip_storage(mut self) -> Self {
        self.config.skip_storage = true;
        self
    }

    /// Set user ID for document ownership
    pub fn user_id(mut self, user_id: i32) -> Self {
        self.config.user_id = Some(user_id);
        self
    }

    /// Make documents public
    pub fn public(mut self) -> Self {
        self.config.is_public = true;
        self
    }

    /// Build the pipeline
    pub fn build(self) -> DocumentPipeline {
        DocumentPipeline { config: self.config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let pipeline = DocumentPipeline::builder().build();
        assert!(pipeline.config.pool.is_none());
        assert!(!pipeline.config.skip_analysis);
    }

    #[test]
    fn test_builder_skip_options() {
        let pipeline = DocumentPipeline::builder()
            .skip_embedding()
            .skip_analysis()
            .skip_storage()
            .build();

        assert!(pipeline.config.skip_embedding);
        assert!(pipeline.config.skip_analysis);
        assert!(pipeline.config.skip_storage);
    }

    #[test]
    fn test_builder_user_id() {
        let pipeline = DocumentPipeline::builder().user_id(42).build();
        assert_eq!(pipeline.config.user_id, Some(42));
    }

    #[test]
    fn test_builder_public() {
        let pipeline = DocumentPipeline::builder().public().build();
        assert!(pipeline.config.is_public);
    }

    #[test]
    fn test_semantic_chunking() {
        let pipeline = DocumentPipeline::builder().semantic_chunking(256).build();

        match pipeline.config.chunking_strategy {
            ChunkingStrategy::Semantic { max_tokens, .. } => {
                assert_eq!(max_tokens, 256);
            }
            _ => panic!("Expected Semantic chunking"),
        }
    }

    #[test]
    fn test_fixed_chunking() {
        let pipeline = DocumentPipeline::builder().fixed_chunking(100, 10).build();

        match pipeline.config.chunking_strategy {
            ChunkingStrategy::FixedSize { size, overlap } => {
                assert_eq!(size, 100);
                assert_eq!(overlap, 10);
            }
            _ => panic!("Expected FixedSize chunking"),
        }
    }

    #[test]
    fn test_minimal_pipeline() {
        let pipeline = DocumentPipeline::minimal();
        assert!(pipeline.config.skip_embedding);
        assert!(pipeline.config.skip_analysis);
        assert!(pipeline.config.skip_storage);
    }

    #[tokio::test]
    async fn test_process_text_minimal() {
        let pipeline = DocumentPipeline::minimal();

        let result = pipeline
            .process_text("Test Document", "This is test content for the pipeline.")
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.title, "Test Document");
        assert!(output.chunks_count > 0);
    }
}
