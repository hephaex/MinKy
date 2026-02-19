//! Conversation knowledge extraction pipeline.
//!
//! This service orchestrates the full flow:
//!   1. Receive a list of `PlatformMessage` objects
//!   2. Filter them with `SlackService::apply_filter`
//!   3. Decide if the thread is worth analysing
//!   4. Build an LLM prompt via `SlackService::build_conversation_prompt`
//!   5. Call the Anthropic / OpenAI Claude API
//!   6. Parse the response into `ExtractedKnowledge`
//!   7. Classify and return the extraction result
//!
//! The pure business-logic helpers live in `SlackService` (unit-tested).
//! This service owns all async I/O and is tested with integration tests or
//! mocked HTTP (wiremock).

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    models::{ExtractedKnowledge, ExtractionStatus, MessageFilter, PlatformMessage},
    services::{ConversationStats, SlackService},
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Runtime configuration for the extraction pipeline.
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Anthropic API key (Claude).  Takes precedence when present.
    pub anthropic_api_key: Option<String>,

    /// Minimum reply count for a thread to be eligible (default: 1).
    pub min_replies: usize,

    /// Minimum total character count across all messages (default: 100).
    pub min_chars: usize,

    /// Claude model to use.
    pub model: String,

    /// Maximum tokens to request from the LLM.
    pub max_tokens: u32,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: None,
            min_replies: 1,
            min_chars: 100,
            model: "claude-3-5-haiku-20241022".to_string(),
            max_tokens: 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// Extraction result
// ---------------------------------------------------------------------------

/// The outcome of running the pipeline against a single conversation thread.
#[derive(Debug)]
pub struct ExtractionResult {
    pub knowledge: ExtractedKnowledge,
    pub status: ExtractionStatus,
    pub stats: ConversationStats,
}

// ---------------------------------------------------------------------------
// Anthropic API types (minimal)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
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

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Orchestrates knowledge extraction from messaging platform conversations.
pub struct ConversationExtractionService {
    http: Client,
    config: ExtractionConfig,
}

impl ConversationExtractionService {
    pub fn new(config: ExtractionConfig) -> Self {
        Self {
            http: Client::new(),
            config,
        }
    }

    /// Extract knowledge from a slice of messages that belong to one thread.
    ///
    /// Returns `AppError::Validation` when the thread does not meet quality
    /// thresholds (too short, single participant, etc.).
    pub async fn extract(
        &self,
        conversation_id: &str,
        messages: &[PlatformMessage],
        filter: &MessageFilter,
    ) -> Result<ExtractionResult, AppError> {
        // 1. Apply filter
        let filtered: Vec<&PlatformMessage> = SlackService::apply_filter(messages, filter);

        if filtered.is_empty() {
            return Err(AppError::Validation(
                "No messages matched the filter criteria".to_string(),
            ));
        }

        // Collect into owned slice for analysis helpers
        let filtered_owned: Vec<PlatformMessage> = filtered.into_iter().cloned().collect();

        // 2. Quality gate
        let is_worth = SlackService::is_thread_worth_analysing(
            &filtered_owned,
            self.config.min_replies,
            self.config.min_chars,
        );

        if !is_worth {
            return Err(AppError::Validation(
                "Thread does not meet minimum quality criteria (replies/chars/participants)"
                    .to_string(),
            ));
        }

        // 3. Build prompt
        let conversation_text = SlackService::build_conversation_prompt(&filtered_owned);

        // 4. Call LLM
        let raw_response = self.call_llm(&conversation_text).await?;

        // 5. Detect platform from first message
        let platform = filtered_owned[0].platform.clone();

        // 6. Parse response
        let knowledge = SlackService::parse_extraction_response(conversation_id, platform, &raw_response)
            .ok_or_else(|| {
                AppError::ExternalService(
                    "LLM returned unparseable extraction response".to_string(),
                )
            })?;

        // 7. Classify
        let status = SlackService::classify_status(&knowledge);

        // 8. Compute stats
        let stats = ConversationStats::compute(&filtered_owned);

        Ok(ExtractionResult {
            knowledge,
            status,
            stats,
        })
    }

    /// Build the system prompt for knowledge extraction.
    pub fn build_system_prompt() -> String {
        r#"You are a knowledge extraction assistant. Given a conversation from a team messaging platform, extract the key knowledge shared in the discussion.

Return ONLY valid JSON with this exact schema:
{
  "title": "Short descriptive title (5-10 words)",
  "summary": "2-3 sentence summary of the main topic and outcome",
  "insights": ["insight 1", "insight 2"],
  "problem_solved": "Description of the problem if one was solved, or empty string",
  "technologies": ["tech1", "tech2"],
  "relevant_for": ["role1", "role2"],
  "confidence": 0.0-1.0
}

Guidelines:
- confidence >= 0.7 means high-quality extractable knowledge
- confidence < 0.3 means the conversation lacks extractable knowledge
- relevant_for should use role labels like: backend-engineer, frontend-engineer, devops, product-manager, data-scientist
- Keep insights concise and actionable"#.to_string()
    }

    /// Send the conversation text to the configured LLM and return the raw text response.
    async fn call_llm(&self, conversation_text: &str) -> Result<String, AppError> {
        let api_key = self
            .config
            .anthropic_api_key
            .as_deref()
            .ok_or_else(|| AppError::Configuration("ANTHROPIC_API_KEY not configured".to_string()))?;

        let request_body = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            system: Self::build_system_prompt(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: format!(
                    "Extract knowledge from this conversation:\n\n{}",
                    conversation_text
                ),
            }],
        };

        let response = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Anthropic API request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Anthropic API error {status}: {body}"
            )));
        }

        let anthropic_resp: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Anthropic response: {e}")))?;

        anthropic_resp
            .content
            .into_iter()
            .next()
            .map(|c| c.text)
            .ok_or_else(|| AppError::ExternalService("Anthropic returned empty content".to_string()))
    }
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic only – no I/O)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_config_default_values() {
        let config = ExtractionConfig::default();
        assert_eq!(config.min_replies, 1);
        assert_eq!(config.min_chars, 100);
        assert!(config.anthropic_api_key.is_none());
        assert_eq!(config.max_tokens, 1024);
    }

    #[test]
    fn test_extraction_config_model_is_claude_haiku() {
        let config = ExtractionConfig::default();
        assert!(config.model.contains("claude"));
    }

    #[test]
    fn test_build_system_prompt_contains_json_schema() {
        let prompt = ConversationExtractionService::build_system_prompt();
        assert!(prompt.contains("\"title\""));
        assert!(prompt.contains("\"summary\""));
        assert!(prompt.contains("\"confidence\""));
    }

    #[test]
    fn test_build_system_prompt_contains_confidence_guideline() {
        let prompt = ConversationExtractionService::build_system_prompt();
        assert!(prompt.contains("0.7"));
        assert!(prompt.contains("0.3"));
    }

    #[test]
    fn test_build_system_prompt_mentions_role_labels() {
        let prompt = ConversationExtractionService::build_system_prompt();
        assert!(prompt.contains("backend-engineer"));
        assert!(prompt.contains("devops"));
    }

    #[test]
    fn test_service_creation_with_custom_config() {
        let config = ExtractionConfig {
            anthropic_api_key: Some("sk-test".to_string()),
            min_replies: 2,
            min_chars: 200,
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 2048,
        };
        let service = ConversationExtractionService::new(config.clone());
        assert_eq!(service.config.min_replies, 2);
        assert_eq!(service.config.min_chars, 200);
        assert_eq!(service.config.max_tokens, 2048);
    }
}
