//! AI Analysis stage - uses Claude to analyze documents
//!
//! Extracts structured understanding from documents:
//! - Topics
//! - Summary
//! - Problem solved
//! - Insights
//! - Technologies
//! - Relevant roles

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{PipelineContext, PipelineError, PipelineResult, PipelineStage};

use super::embedding::EmbeddedDocument;

/// Configuration for AI analysis
#[derive(Debug, Clone)]
pub struct AIAnalysisConfig {
    /// Anthropic API key
    pub api_key: Option<String>,

    /// Model to use (default: claude-3-5-haiku)
    pub model: String,

    /// Maximum tokens for response
    pub max_tokens: u32,

    /// Skip analysis if true
    pub skip: bool,
}

impl Default for AIAnalysisConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "claude-3-5-haiku-20241022".to_string(),
            max_tokens: 1024,
            skip: false,
        }
    }
}

/// Document with AI analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedDocument {
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
    pub embedded_chunks: Vec<super::embedding::EmbeddedChunk>,

    /// Document-level embedding
    pub document_embedding: Option<Vec<f32>>,

    /// Embedding model used
    pub embedding_model: crate::models::EmbeddingModel,

    /// AI analysis results
    pub analysis: Option<DocumentAnalysis>,

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

/// AI-generated document analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAnalysis {
    /// Core topics (3-5)
    pub topics: Vec<String>,

    /// One-line summary
    pub summary: String,

    /// Problem this document solves
    pub problem_solved: Option<String>,

    /// Key insights
    pub insights: Vec<String>,

    /// Related technologies
    pub technologies: Vec<String>,

    /// Who this document is relevant for
    pub relevant_for: Vec<String>,

    /// Model used for analysis
    pub analyzer_model: String,
}

/// AI Analysis stage
#[derive(Debug, Clone)]
pub struct AIAnalysisStage {
    config: AIAnalysisConfig,
    http_client: reqwest::Client,
}

impl AIAnalysisStage {
    /// Create a new AI analysis stage
    pub fn new(config: AIAnalysisConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create with API key
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self::new(AIAnalysisConfig {
            api_key: Some(api_key.into()),
            ..Default::default()
        })
    }

    /// Create a no-op stage
    pub fn skip() -> Self {
        Self::new(AIAnalysisConfig {
            skip: true,
            ..Default::default()
        })
    }

    /// Analyze document using Claude
    async fn analyze(&self, title: &str, content: &str) -> PipelineResult<DocumentAnalysis> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| PipelineError::Configuration("Anthropic API key not configured".into()))?;

        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(title, content);

        let body = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            system: Some(system_prompt),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_prompt,
            }],
        };

        let response = self
            .http_client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Claude API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PipelineError::ExternalService(format!(
                "Claude API error: {}",
                error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| PipelineError::ExternalService(format!("Failed to parse Claude response: {}", e)))?;

        let raw_text = anthropic_response
            .content
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("");

        self.parse_response(raw_text)
    }

    fn build_system_prompt(&self) -> String {
        r#"You are a knowledge analyst for a team knowledge platform. Your task is to deeply understand documents and extract structured insights.

Analyze the provided document and respond ONLY with a valid JSON object in this exact format:
{
  "topics": ["topic1", "topic2", "topic3"],
  "summary": "One clear sentence summarizing the document.",
  "problem_solved": "The specific problem this document addresses, or null if not applicable.",
  "insights": ["insight1", "insight2", "insight3"],
  "technologies": ["tech1", "tech2"],
  "relevant_for": ["role1", "role2"]
}

Rules:
- topics: 3-5 concise key themes (strings)
- summary: exactly one sentence, no longer than 200 characters
- problem_solved: a specific problem statement or null
- insights: 2-5 actionable or important insights
- technologies: tools, frameworks, languages mentioned or used
- relevant_for: job roles or personas who benefit (e.g. "backend engineer", "team lead")
- All values must be in the same language as the document
- Respond ONLY with the JSON object, no markdown fences, no explanation"#.to_string()
    }

    fn build_user_prompt(&self, title: &str, content: &str) -> String {
        // Truncate content if too long
        let max_content_len = 8000;
        let truncated_content = if content.len() > max_content_len {
            format!("{}...[truncated]", &content[..max_content_len])
        } else {
            content.to_string()
        };

        format!("Title: {}\n\nContent:\n{}", title, truncated_content)
    }

    fn parse_response(&self, raw: &str) -> PipelineResult<DocumentAnalysis> {
        // Strip markdown code fences if present
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let parsed: RawAnalysisJson = serde_json::from_str(cleaned).map_err(|e| {
            PipelineError::stage_failure(
                "analysis",
                format!("Failed to parse analysis response: {}. Raw: {}", e, cleaned),
            )
        })?;

        Ok(DocumentAnalysis {
            topics: parsed.topics,
            summary: parsed.summary,
            problem_solved: parsed.problem_solved,
            insights: parsed.insights,
            technologies: parsed.technologies,
            relevant_for: parsed.relevant_for,
            analyzer_model: self.config.model.clone(),
        })
    }
}

impl Default for AIAnalysisStage {
    fn default() -> Self {
        Self::skip()
    }
}

#[async_trait]
impl PipelineStage<EmbeddedDocument, AnalyzedDocument> for AIAnalysisStage {
    fn name(&self) -> &'static str {
        "ai_analysis"
    }

    fn should_skip(&self, _input: &EmbeddedDocument, _context: &PipelineContext) -> bool {
        self.config.skip
    }

    async fn process(
        &self,
        input: EmbeddedDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<AnalyzedDocument> {
        let analysis = if self.config.skip {
            None
        } else {
            let result = self.analyze(&input.title, &input.plain_text).await?;
            context.metrics.increment_api_calls();

            // Store topics in context for later use
            context.set_metadata("topics", &result.topics);
            context.set_metadata("summary", &result.summary);

            Some(result)
        };

        Ok(AnalyzedDocument {
            title: input.title,
            plain_text: input.plain_text,
            original_content: input.original_content,
            mime_type: input.mime_type,
            metadata: input.metadata,
            embedded_chunks: input.embedded_chunks,
            document_embedding: input.document_embedding,
            embedding_model: input.embedding_model,
            analysis,
            headings: input.headings,
            links: input.links,
            code_blocks: input.code_blocks,
            source_type: input.source_type,
            source_path: input.source_path,
        })
    }
}

// Anthropic API types

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Deserialize)]
struct RawAnalysisJson {
    topics: Vec<String>,
    summary: String,
    problem_solved: Option<String>,
    insights: Vec<String>,
    technologies: Vec<String>,
    relevant_for: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedded_doc() -> EmbeddedDocument {
        EmbeddedDocument {
            title: "Test Document".to_string(),
            plain_text: "This is test content about Rust programming.".to_string(),
            original_content: "This is test content about Rust programming.".to_string(),
            mime_type: "text/plain".to_string(),
            metadata: super::super::metadata::ExtractedMetadata::default(),
            embedded_chunks: Vec::new(),
            document_embedding: None,
            embedding_model: crate::models::EmbeddingModel::OpenaiTextEmbedding3Small,
            headings: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            source_type: "test".to_string(),
            source_path: None,
        }
    }

    #[tokio::test]
    async fn test_skip_analysis() {
        let stage = AIAnalysisStage::skip();
        let mut context = PipelineContext::new();

        let doc = make_embedded_doc();
        let result = stage.process(doc, &mut context).await.unwrap();

        assert!(result.analysis.is_none());
    }

    #[test]
    fn test_parse_valid_response() {
        let stage = AIAnalysisStage::skip();

        let raw = r#"{
            "topics": ["Rust", "async programming", "tokio"],
            "summary": "An introduction to async Rust with the Tokio runtime.",
            "problem_solved": "Handling I/O-bound concurrency without blocking threads.",
            "insights": ["Use async/await for I/O", "tokio::spawn for tasks"],
            "technologies": ["Rust", "Tokio", "async-std"],
            "relevant_for": ["backend engineer", "systems programmer"]
        }"#;

        let result = stage.parse_response(raw);
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.topics.len(), 3);
        assert!(analysis.problem_solved.is_some());
    }

    #[test]
    fn test_parse_response_with_markdown_fences() {
        let stage = AIAnalysisStage::skip();

        let raw = r#"```json
{
    "topics": ["Rust"],
    "summary": "A Rust guide.",
    "problem_solved": null,
    "insights": ["Ownership is key"],
    "technologies": ["Rust"],
    "relevant_for": ["developer"]
}
```"#;

        let result = stage.parse_response(raw);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_user_prompt() {
        let stage = AIAnalysisStage::skip();
        let prompt = stage.build_user_prompt("My Title", "Some content");

        assert!(prompt.contains("Title: My Title"));
        assert!(prompt.contains("Some content"));
    }
}
