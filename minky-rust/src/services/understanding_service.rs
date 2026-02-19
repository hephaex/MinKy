//! Document Understanding Service
//!
//! Analyzes documents using the Claude API to extract:
//! - Core topics (3-5 key themes)
//! - One-line summary
//! - Problem solved (if applicable)
//! - Key insights
//! - Related technologies and tools
//! - Target audience by role
//!
//! # Example
//!
//! ```no_run
//! use minky::services::UnderstandingService;
//! use minky::config::Config;
//!
//! async fn example(config: Config) {
//!     let service = UnderstandingService::new(config);
//!     // service.analyze_document(request).await?;
//! }
//! ```

use reqwest::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{DocumentUnderstandingRequest, DocumentUnderstandingResponse},
};

/// Service for AI-powered document understanding using Claude
pub struct UnderstandingService {
    client: Client,
    config: Config,
}

impl UnderstandingService {
    /// Create a new understanding service
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Analyze a document using Claude and return structured understanding
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Configuration`] if the Anthropic API key is not set.
    /// Returns [`AppError::ExternalService`] if the Claude API call fails.
    /// Returns [`AppError::Validation`] if the AI response cannot be parsed.
    pub async fn analyze_document(
        &self,
        request: DocumentUnderstandingRequest,
    ) -> AppResult<DocumentUnderstandingResponse> {
        let api_key = self
            .config
            .anthropic_api_key
            .as_ref()
            .ok_or_else(|| AppError::Configuration("Anthropic API key not configured".into()))?;

        let system_prompt = build_system_prompt();
        let user_prompt = build_user_prompt(&request);

        let body = AnthropicRequest {
            model: "claude-3-5-haiku-20241022".to_string(),
            max_tokens: 1024,
            system: Some(system_prompt),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_prompt,
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Claude API request failed: {e}")))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Claude API error: {error_text}"
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Claude response: {e}")))?;

        let raw_text = anthropic_response
            .content
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("");

        parse_understanding_response(raw_text)
    }

    /// Analyze a document by fetching its content from the database and calling Claude
    ///
    /// # Errors
    ///
    /// Returns [`AppError::NotFound`] if the document does not exist.
    /// Returns errors from [`Self::analyze_document`] for API failures.
    pub async fn analyze_document_by_id(
        &self,
        db: &sqlx::PgPool,
        document_id: Uuid,
    ) -> AppResult<DocumentUnderstandingResponse> {
        let row: (String, String) =
            sqlx::query_as("SELECT title, content FROM documents WHERE id = $1")
                .bind(document_id)
                .fetch_one(db)
                .await
                .map_err(|_| AppError::NotFound(format!("Document {document_id} not found")))?;

        let (title, content) = row;

        let request = DocumentUnderstandingRequest {
            document_id,
            content,
            title: Some(title),
        };

        self.analyze_document(request).await
    }
}

/// Build the system prompt that instructs Claude to respond with structured JSON
fn build_system_prompt() -> String {
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

/// Build the user prompt from the document request
fn build_user_prompt(request: &DocumentUnderstandingRequest) -> String {
    let title_section = match &request.title {
        Some(title) => format!("Title: {title}\n\n"),
        None => String::new(),
    };
    format!("{title_section}Content:\n{}", request.content)
}

/// Parse the raw JSON text from Claude into a [`DocumentUnderstandingResponse`]
fn parse_understanding_response(raw: &str) -> AppResult<DocumentUnderstandingResponse> {
    // Strip markdown code fences if present (defensive parsing)
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: RawUnderstandingJson = serde_json::from_str(cleaned).map_err(|e| {
        AppError::Validation(format!(
            "Failed to parse document understanding response: {e}. Raw: {cleaned}"
        ))
    })?;

    Ok(DocumentUnderstandingResponse {
        topics: parsed.topics,
        summary: parsed.summary,
        problem_solved: parsed.problem_solved,
        insights: parsed.insights,
        technologies: parsed.technologies,
        relevant_for: parsed.relevant_for,
    })
}

// ---------------------------------------------------------------------------
// Internal types for Anthropic API
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
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

/// Intermediate struct for JSON deserialization
#[derive(Debug, Deserialize)]
struct RawUnderstandingJson {
    topics: Vec<String>,
    summary: String,
    problem_solved: Option<String>,
    insights: Vec<String>,
    technologies: Vec<String>,
    relevant_for: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_response() {
        let raw = r#"{
            "topics": ["Rust", "async programming", "tokio"],
            "summary": "An introduction to async Rust with the Tokio runtime.",
            "problem_solved": "Handling I/O-bound concurrency without blocking threads.",
            "insights": ["Use async/await for I/O", "tokio::spawn for tasks"],
            "technologies": ["Rust", "Tokio", "async-std"],
            "relevant_for": ["backend engineer", "systems programmer"]
        }"#;

        let result = parse_understanding_response(raw);
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert_eq!(resp.topics.len(), 3);
        assert_eq!(resp.summary, "An introduction to async Rust with the Tokio runtime.");
        assert!(resp.problem_solved.is_some());
        assert_eq!(resp.insights.len(), 2);
        assert_eq!(resp.technologies.len(), 3);
        assert_eq!(resp.relevant_for.len(), 2);
    }

    #[test]
    fn test_parse_response_strips_markdown_fences() {
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

        let result = parse_understanding_response(raw);
        assert!(result.is_ok());

        let resp = result.unwrap();
        assert_eq!(resp.topics, vec!["Rust"]);
        assert!(resp.problem_solved.is_none());
    }

    #[test]
    fn test_parse_invalid_response_returns_error() {
        let raw = "This is not valid JSON";
        let result = parse_understanding_response(raw);
        assert!(result.is_err());
        matches!(result.unwrap_err(), AppError::Validation(_));
    }

    #[test]
    fn test_build_user_prompt_with_title() {
        let request = DocumentUnderstandingRequest {
            document_id: Uuid::new_v4(),
            content: "Some content here.".to_string(),
            title: Some("My Doc".to_string()),
        };

        let prompt = build_user_prompt(&request);
        assert!(prompt.contains("Title: My Doc"));
        assert!(prompt.contains("Some content here."));
    }

    #[test]
    fn test_build_user_prompt_without_title() {
        let request = DocumentUnderstandingRequest {
            document_id: Uuid::new_v4(),
            content: "Content only.".to_string(),
            title: None,
        };

        let prompt = build_user_prompt(&request);
        assert!(!prompt.contains("Title:"));
        assert!(prompt.contains("Content only."));
    }

    #[test]
    fn test_build_system_prompt_contains_json_instruction() {
        let prompt = build_system_prompt();
        assert!(
            prompt.contains("JSON"),
            "System prompt should instruct Claude to respond with JSON"
        );
    }

    #[test]
    fn test_build_system_prompt_contains_required_fields() {
        let prompt = build_system_prompt();
        // Prompt must mention all required JSON fields
        for field in &["topics", "summary", "insights", "technologies", "relevant_for"] {
            assert!(
                prompt.contains(field),
                "System prompt should mention required field '{field}'"
            );
        }
    }

    #[test]
    fn test_build_user_prompt_title_appears_before_content() {
        let request = DocumentUnderstandingRequest {
            document_id: Uuid::new_v4(),
            content: "Body text.".to_string(),
            title: Some("The Title".to_string()),
        };
        let prompt = build_user_prompt(&request);
        let title_pos = prompt.find("The Title").expect("Title must be in prompt");
        let content_pos = prompt.find("Body text.").expect("Content must be in prompt");
        assert!(title_pos < content_pos, "Title should appear before content in the prompt");
    }

    #[test]
    fn test_parse_response_null_problem_solved_maps_to_none() {
        let raw = r#"{
            "topics": ["Rust"],
            "summary": "A Rust guide.",
            "problem_solved": null,
            "insights": ["Ownership is key"],
            "technologies": ["Rust"],
            "relevant_for": ["engineer"]
        }"#;
        let result = parse_understanding_response(raw).unwrap();
        assert!(result.problem_solved.is_none(), "null problem_solved should map to None");
    }
}
