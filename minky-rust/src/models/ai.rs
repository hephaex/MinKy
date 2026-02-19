use serde::{Deserialize, Serialize};

/// LLM Provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LLMProvider {
    OpenAI,
    #[default]
    Anthropic,
}


/// AI Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModelConfig {
    pub provider: LLMProvider,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for AIModelConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Anthropic,
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// AI suggestion request
#[derive(Debug, Deserialize)]
pub struct SuggestionRequest {
    pub content: String,
    pub suggestion_type: SuggestionType,
    pub context: Option<String>,
}

/// Types of AI suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    Title,
    Summary,
    Tags,
    Improve,
    Translate,
    Grammar,
    Expand,
    Simplify,
}

/// AI suggestion response
#[derive(Debug, Serialize)]
pub struct SuggestionResponse {
    pub suggestion: String,
    pub suggestion_type: SuggestionType,
    pub tokens_used: u32,
    pub model: String,
}

/// Chat message for AI conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// Embedding request for semantic search
#[derive(Debug, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
}

/// Embedding response
#[derive(Debug, Serialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub dimensions: usize,
    pub model: String,
}

/// Document clustering result
#[derive(Debug, Serialize)]
pub struct ClusterResult {
    pub cluster_id: i32,
    pub label: String,
    pub document_ids: Vec<String>,
    pub centroid_keywords: Vec<String>,
}

/// Analytics time range
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TimeRange {
    Day,
    Week,
    #[default]
    Month,
    Year,
    Custom { start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_provider_default_is_anthropic() {
        assert_eq!(LLMProvider::default(), LLMProvider::Anthropic);
    }

    #[test]
    fn test_ai_model_config_default_provider_is_anthropic() {
        let config = AIModelConfig::default();
        assert_eq!(config.provider, LLMProvider::Anthropic);
    }

    #[test]
    fn test_ai_model_config_default_max_tokens() {
        let config = AIModelConfig::default();
        assert!(config.max_tokens > 0, "max_tokens should be positive");
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn test_ai_model_config_default_temperature_in_range() {
        let config = AIModelConfig::default();
        assert!(
            config.temperature >= 0.0 && config.temperature <= 2.0,
            "temperature should be in [0.0, 2.0]"
        );
    }

    #[test]
    fn test_ai_model_config_default_model_non_empty() {
        let config = AIModelConfig::default();
        assert!(!config.model.is_empty(), "default model name should not be empty");
    }

    #[test]
    fn test_llm_provider_serde_roundtrip() {
        let providers = [LLMProvider::OpenAI, LLMProvider::Anthropic];
        for p in &providers {
            let json = serde_json::to_string(p).unwrap();
            let back: LLMProvider = serde_json::from_str(&json).unwrap();
            assert_eq!(p, &back);
        }
    }

    #[test]
    fn test_time_range_default_is_month() {
        let range = TimeRange::default();
        assert!(matches!(range, TimeRange::Month));
    }

    #[test]
    fn test_chat_role_serde_user() {
        let json = serde_json::to_string(&ChatRole::User).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_chat_role_serde_assistant() {
        let json = serde_json::to_string(&ChatRole::Assistant).unwrap();
        assert_eq!(json, "\"assistant\"");
    }
}

