//! Shared types for local/OpenAI-compatible LLM endpoints (Strix Halo llama.cpp).
//!
//! These types cover the OpenAI-compatible `/v1/chat/completions` request and
//! response shapes (both streaming SSE chunks and non-streaming bodies). They
//! are used by `RagService` (Strix Halo answer generation) and by the
//! `UnderstandingService` / `ConversationExtractionService` Upstage Solar
//! calls, which expose the same OpenAI-compatible surface.
//!
//! Strix Halo serves several models behind llama.cpp's `--reasoning off`
//! endpoint; see `RagService::generate_answer` and `routes::rag::ask_stream`.

use serde::{Deserialize, Serialize};

/// Request body for an OpenAI-compatible `/v1/chat/completions` call.
#[derive(Debug, Serialize)]
pub struct LocalLlmRequest {
    pub model: String,
    pub messages: Vec<LocalLlmMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// A single message in the local LLM conversation (OpenAI role/content shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLlmMessage {
    pub role: String,
    pub content: String,
}

/// Non-streaming response from an OpenAI-compatible `/v1/chat/completions` call.
#[derive(Debug, Deserialize)]
pub struct LocalLlmResponse {
    #[serde(default)]
    pub model: Option<String>,
    pub choices: Vec<LocalLlmChoice>,
    #[serde(default)]
    pub usage: Option<LocalLlmUsage>,
}

#[derive(Debug, Deserialize)]
pub struct LocalLlmChoice {
    pub message: LocalLlmMessage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalLlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

impl LocalLlmResponse {
    /// Extract the first message's text content, if any.
    pub fn first_text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }
}

/// One SSE chunk from an OpenAI-compatible streaming response
/// (`"object":"chat.completion.chunk"`).
#[derive(Debug, Clone, Deserialize)]
pub struct LocalLlmStreamChunk {
    #[serde(default)]
    pub model: Option<String>,
    pub choices: Vec<LocalLlmStreamChoice>,
    #[serde(default)]
    pub usage: Option<LocalLlmUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalLlmStreamChoice {
    pub delta: LocalLlmStreamDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LocalLlmStreamDelta {
    #[serde(default)]
    pub content: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_llm_request_serialization_omits_none_fields() {
        let req = LocalLlmRequest {
            model: "Qwen3.8-27B-UD-Q4_K_XL.gguf".to_string(),
            messages: vec![LocalLlmMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 512,
            temperature: None,
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"max_tokens\":512"));
        assert!(!json.contains("temperature"));
        assert!(!json.contains("stream"));
    }

    #[test]
    fn test_local_llm_request_serialization_with_stream() {
        let req = LocalLlmRequest {
            model: "m".to_string(),
            messages: vec![],
            max_tokens: 10,
            temperature: Some(0.3),
            stream: Some(true),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"temperature\":0.3"));
    }

    #[test]
    fn test_local_llm_response_deserialization() {
        let json = r#"{
            "model": "Qwen3.8-27B-UD-Q4_K_XL.gguf",
            "choices": [{"message": {"role": "assistant", "content": "Answer text"}}],
            "usage": {"prompt_tokens": 20, "completion_tokens": 30, "total_tokens": 50}
        }"#;
        let resp: LocalLlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.first_text(), Some("Answer text"));
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 20);
        assert_eq!(usage.completion_tokens, 30);
        assert_eq!(usage.total_tokens, 50);
    }

    #[test]
    fn test_local_llm_response_first_text_empty_choices() {
        let resp = LocalLlmResponse {
            model: None,
            choices: vec![],
            usage: None,
        };
        assert_eq!(resp.first_text(), None);
    }

    #[test]
    fn test_local_llm_stream_chunk_deserialization_with_content() {
        let json = r#"{"choices":[{"finish_reason":null,"index":0,"delta":{"content":"Hi"}}],"model":"m"}"#;
        let chunk: LocalLlmStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hi"));
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_local_llm_stream_chunk_deserialization_finish() {
        let json = r#"{"choices":[{"finish_reason":"stop","index":0,"delta":{}}],"model":"m"}"#;
        let chunk: LocalLlmStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].finish_reason.as_deref(), Some("stop"));
        assert!(chunk.choices[0].delta.content.is_none());
    }
}
