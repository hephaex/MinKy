//! Common types for Anthropic Claude API integration.
//!
//! This module provides shared request/response types used across multiple services
//! that interact with the Anthropic API (ai_service, understanding_service, rag_service,
//! conversation_extraction_service).

use serde::{Deserialize, Serialize};

/// Request body for Anthropic Messages API.
#[derive(Debug, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<AnthropicMessage>,
}

/// A single message in the Anthropic conversation.
#[derive(Debug, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

/// Response from Anthropic Messages API.
#[derive(Debug, Deserialize)]
pub struct AnthropicResponse {
    /// The model that generated the response.
    #[serde(default)]
    pub model: Option<String>,
    pub content: Vec<AnthropicContent>,
    #[serde(default)]
    pub usage: Option<AnthropicUsage>,
}

/// Content block in Anthropic response.
#[derive(Debug, Deserialize)]
pub struct AnthropicContent {
    pub text: String,
}

/// Token usage statistics from Anthropic API.
#[derive(Debug, Deserialize)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl AnthropicRequest {
    /// Create a new request with a system prompt and user message.
    pub fn new(model: &str, max_tokens: u32, system: Option<String>, user_message: &str) -> Self {
        Self {
            model: model.to_string(),
            max_tokens,
            system,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        }
    }

    /// Create a new request with messages already constructed.
    pub fn with_messages(
        model: &str,
        max_tokens: u32,
        system: Option<String>,
        messages: Vec<AnthropicMessage>,
    ) -> Self {
        Self {
            model: model.to_string(),
            max_tokens,
            system,
            messages,
        }
    }
}

impl AnthropicResponse {
    /// Extract the first text content from the response.
    pub fn first_text(&self) -> Option<&str> {
        self.content.first().map(|c| c.text.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_request_new() {
        let req = AnthropicRequest::new(
            "claude-3-opus-20240229",
            1024,
            Some("You are a helpful assistant.".to_string()),
            "Hello, how are you?",
        );
        assert_eq!(req.model, "claude-3-opus-20240229");
        assert_eq!(req.max_tokens, 1024);
        assert!(req.system.is_some());
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
    }

    #[test]
    fn test_anthropic_request_with_messages() {
        let messages = vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            AnthropicMessage {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
            },
        ];
        let req = AnthropicRequest::with_messages("claude-3-haiku-20240307", 512, None, messages);
        assert_eq!(req.messages.len(), 2);
        assert!(req.system.is_none());
    }

    #[test]
    fn test_anthropic_response_first_text() {
        let resp = AnthropicResponse {
            model: Some("claude-3-haiku-20240307".to_string()),
            content: vec![AnthropicContent {
                text: "Hello, world!".to_string(),
            }],
            usage: Some(AnthropicUsage {
                input_tokens: 10,
                output_tokens: 5,
            }),
        };
        assert_eq!(resp.first_text(), Some("Hello, world!"));
    }

    #[test]
    fn test_anthropic_response_first_text_empty() {
        let resp = AnthropicResponse {
            model: None,
            content: vec![],
            usage: None,
        };
        assert_eq!(resp.first_text(), None);
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let req = AnthropicRequest::new("claude-3-haiku-20240307", 256, None, "Test");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"claude-3-haiku-20240307\""));
        assert!(json.contains("\"max_tokens\":256"));
        // system should be omitted when None due to skip_serializing_if
        assert!(!json.contains("\"system\":null"));
    }

    #[test]
    fn test_anthropic_response_deserialization() {
        let json = r#"{
            "content": [{"type": "text", "text": "Response text"}],
            "usage": {"input_tokens": 100, "output_tokens": 50}
        }"#;
        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert_eq!(resp.content[0].text, "Response text");
        assert!(resp.usage.is_some());
        let usage = resp.usage.unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_anthropic_response_deserialization_without_usage() {
        let json = r#"{
            "content": [{"text": "Response text"}]
        }"#;
        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert!(resp.usage.is_none());
    }
}
