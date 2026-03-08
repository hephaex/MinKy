//! Ingestion stage - handles document input from various sources
//!
//! This is the first stage in the pipeline, responsible for reading
//! raw document content from files, URLs, or direct text input.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

/// Source of the document being ingested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentSource {
    /// Direct text content
    Text {
        title: String,
        content: String,
    },

    /// File path (local filesystem)
    File {
        path: String,
    },

    /// URL to fetch
    Url {
        url: String,
    },

    /// Raw bytes with MIME type
    Bytes {
        data: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    },
}

/// Options for document ingestion
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestionOptions {
    /// Override the detected MIME type
    pub mime_type: Option<String>,

    /// Character encoding (default: UTF-8)
    pub encoding: Option<String>,

    /// Maximum content size in bytes (default: 10MB)
    pub max_size: Option<usize>,

    /// Whether to strip HTML tags from content
    pub strip_html: bool,
}

/// Input to the ingestion stage
#[derive(Debug, Clone)]
pub struct IngestionInput {
    /// Source of the document
    pub source: DocumentSource,

    /// Ingestion options
    pub options: IngestionOptions,
}

impl IngestionInput {
    /// Create input from raw text
    pub fn from_text(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Text {
                title: title.into(),
                content: content.into(),
            },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from a file path
    pub fn from_file(path: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::File { path: path.into() },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from a URL
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Url { url: url.into() },
            options: IngestionOptions::default(),
        }
    }

    /// Create input from bytes
    pub fn from_bytes(data: Vec<u8>, mime_type: impl Into<String>) -> Self {
        Self {
            source: DocumentSource::Bytes {
                data,
                mime_type: mime_type.into(),
                filename: None,
            },
            options: IngestionOptions::default(),
        }
    }

    /// Set options
    pub fn with_options(mut self, options: IngestionOptions) -> Self {
        self.options = options;
        self
    }
}

/// Raw document output from ingestion
#[derive(Debug, Clone)]
pub struct RawDocument {
    /// Original filename or title
    pub title: String,

    /// Raw content as string
    pub content: String,

    /// Detected or specified MIME type
    pub mime_type: String,

    /// Source type for tracking
    pub source_type: String,

    /// Original source path/URL if applicable
    pub source_path: Option<String>,
}

/// Ingestion stage - reads documents from various sources
#[derive(Debug, Clone, Default)]
pub struct IngestionStage {
    http_client: Option<reqwest::Client>,
}

impl IngestionStage {
    /// Create a new ingestion stage
    pub fn new() -> Self {
        Self {
            http_client: Some(reqwest::Client::new()),
        }
    }

    /// Create without HTTP client (for testing)
    pub fn without_http() -> Self {
        Self { http_client: None }
    }

    async fn ingest_text(&self, title: String, content: String) -> PipelineResult<RawDocument> {
        Ok(RawDocument {
            title,
            content,
            mime_type: "text/plain".to_string(),
            source_type: "text".to_string(),
            source_path: None,
        })
    }

    async fn ingest_file(&self, path: &str) -> PipelineResult<RawDocument> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| PipelineError::InvalidInput(format!("Failed to read file: {}", e)))?;

        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(RawDocument {
            title: filename,
            content,
            mime_type,
            source_type: "file".to_string(),
            source_path: Some(path.to_string()),
        })
    }

    async fn ingest_url(&self, url: &str) -> PipelineResult<RawDocument> {
        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| PipelineError::Configuration("HTTP client not configured".into()))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to fetch URL: {}", e)))?;

        if !response.status().is_success() {
            return Err(PipelineError::ExternalService(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let mime_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let content = response
            .text()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to read response: {}", e)))?;

        // Extract title from URL
        let title = url::Url::parse(url)
            .ok()
            .and_then(|u| {
                u.path_segments()
                    .and_then(|mut s| s.next_back().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| "untitled".to_string());

        Ok(RawDocument {
            title,
            content,
            mime_type,
            source_type: "url".to_string(),
            source_path: Some(url.to_string()),
        })
    }

    async fn ingest_bytes(
        &self,
        data: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    ) -> PipelineResult<RawDocument> {
        let content = String::from_utf8(data)
            .map_err(|e| PipelineError::InvalidInput(format!("Invalid UTF-8: {}", e)))?;

        let title = filename.unwrap_or_else(|| "untitled".to_string());

        Ok(RawDocument {
            title,
            content,
            mime_type,
            source_type: "bytes".to_string(),
            source_path: None,
        })
    }
}

#[async_trait]
impl PipelineStage<IngestionInput, RawDocument> for IngestionStage {
    fn name(&self) -> &'static str {
        "ingestion"
    }

    async fn process(
        &self,
        input: IngestionInput,
        context: &mut PipelineContext,
    ) -> PipelineResult<RawDocument> {
        let mut raw_doc = match input.source {
            DocumentSource::Text { title, content } => self.ingest_text(title, content).await?,
            DocumentSource::File { path } => self.ingest_file(&path).await?,
            DocumentSource::Url { url } => self.ingest_url(&url).await?,
            DocumentSource::Bytes {
                data,
                mime_type,
                filename,
            } => self.ingest_bytes(data, mime_type, filename).await?,
        };

        // Apply options
        if let Some(mime_override) = input.options.mime_type {
            raw_doc.mime_type = mime_override;
        }

        // Check size limits
        if let Some(max_size) = input.options.max_size {
            if raw_doc.content.len() > max_size {
                return Err(PipelineError::InvalidInput(format!(
                    "Content exceeds maximum size of {} bytes",
                    max_size
                )));
            }
        }

        // Record metrics
        context.metrics.add_characters(raw_doc.content.len() as u64);
        context.set_metadata("source_type", &raw_doc.source_type);
        context.set_metadata("mime_type", &raw_doc.mime_type);

        Ok(raw_doc)
    }

    fn validate(&self, input: &IngestionInput) -> PipelineResult<()> {
        match &input.source {
            DocumentSource::Text { content, .. } => {
                if content.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "Content cannot be empty".to_string(),
                    ));
                }
            }
            DocumentSource::File { path } => {
                if path.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "File path cannot be empty".to_string(),
                    ));
                }
            }
            DocumentSource::Url { url } => {
                if url.trim().is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "URL cannot be empty".to_string(),
                    ));
                }
                // Basic URL validation
                url::Url::parse(url).map_err(|e| {
                    PipelineError::InvalidInput(format!("Invalid URL: {}", e))
                })?;
            }
            DocumentSource::Bytes { data, .. } => {
                if data.is_empty() {
                    return Err(PipelineError::InvalidInput(
                        "Bytes cannot be empty".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingestion_input_from_text() {
        let input = IngestionInput::from_text("Test", "Hello world");
        match input.source {
            DocumentSource::Text { title, content } => {
                assert_eq!(title, "Test");
                assert_eq!(content, "Hello world");
            }
            _ => panic!("Expected Text source"),
        }
    }

    #[test]
    fn test_ingestion_input_from_file() {
        let input = IngestionInput::from_file("/path/to/file.md");
        match input.source {
            DocumentSource::File { path } => {
                assert_eq!(path, "/path/to/file.md");
            }
            _ => panic!("Expected File source"),
        }
    }

    #[test]
    fn test_ingestion_input_from_url() {
        let input = IngestionInput::from_url("https://example.com/doc");
        match input.source {
            DocumentSource::Url { url } => {
                assert_eq!(url, "https://example.com/doc");
            }
            _ => panic!("Expected URL source"),
        }
    }

    #[tokio::test]
    async fn test_ingest_text() {
        let stage = IngestionStage::without_http();
        let mut context = PipelineContext::new();

        let input = IngestionInput::from_text("My Doc", "Some content here");
        let result = stage.process(input, &mut context).await;

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.title, "My Doc");
        assert_eq!(doc.content, "Some content here");
        assert_eq!(doc.source_type, "text");
    }

    #[test]
    fn test_validate_empty_content() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_text("Title", "   ");
        let result = stage.validate(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_url() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_url("not a url");
        let result = stage.validate(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_url() {
        let stage = IngestionStage::without_http();
        let input = IngestionInput::from_url("https://example.com/doc");
        let result = stage.validate(&input);
        assert!(result.is_ok());
    }
}
