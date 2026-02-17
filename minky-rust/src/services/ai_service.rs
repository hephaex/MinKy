use anyhow::Result;
use reqwest::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{
        AIModelConfig, ChatMessage, ChatRole, EmbeddingResponse, LLMProvider,
        SuggestionRequest, SuggestionResponse, SuggestionType,
    },
};

/// AI service for LLM interactions
pub struct AIService {
    client: Client,
    config: Config,
    model_config: AIModelConfig,
}

impl AIService {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
            model_config: AIModelConfig::default(),
        }
    }

    pub fn with_model_config(mut self, model_config: AIModelConfig) -> Self {
        self.model_config = model_config;
        self
    }

    /// Generate a suggestion based on content
    pub async fn generate_suggestion(&self, request: SuggestionRequest) -> AppResult<SuggestionResponse> {
        let system_prompt = self.get_system_prompt(&request.suggestion_type);
        let user_prompt = self.build_user_prompt(&request);

        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: system_prompt,
            },
            ChatMessage {
                role: ChatRole::User,
                content: user_prompt,
            },
        ];

        let (response, tokens) = self.chat_completion(messages).await?;

        Ok(SuggestionResponse {
            suggestion: response,
            suggestion_type: request.suggestion_type,
            tokens_used: tokens,
            model: self.model_config.model.clone(),
        })
    }

    /// Get system prompt for suggestion type
    fn get_system_prompt(&self, suggestion_type: &SuggestionType) -> String {
        match suggestion_type {
            SuggestionType::Title => {
                "You are a helpful assistant that generates concise, descriptive titles for documents. \
                 Generate a single title that captures the main topic. Keep it under 100 characters.".to_string()
            }
            SuggestionType::Summary => {
                "You are a helpful assistant that creates clear, concise summaries. \
                 Summarize the key points in 2-3 sentences.".to_string()
            }
            SuggestionType::Tags => {
                "You are a helpful assistant that suggests relevant tags for documents. \
                 Return 3-5 tags as a comma-separated list. Tags should be lowercase, single words or short phrases.".to_string()
            }
            SuggestionType::Improve => {
                "You are a helpful writing assistant. Improve the given text for clarity, \
                 grammar, and readability while preserving the original meaning.".to_string()
            }
            SuggestionType::Translate => {
                "You are a professional translator. Translate the text accurately while \
                 maintaining the original tone and meaning.".to_string()
            }
            SuggestionType::Grammar => {
                "You are a grammar checker. Fix any grammatical errors in the text \
                 and return the corrected version.".to_string()
            }
            SuggestionType::Expand => {
                "You are a helpful writing assistant. Expand on the given text, \
                 adding more details and explanations while maintaining the original style.".to_string()
            }
            SuggestionType::Simplify => {
                "You are a helpful writing assistant. Simplify the given text to make it \
                 easier to understand, using simpler words and shorter sentences.".to_string()
            }
        }
    }

    /// Build user prompt from request
    fn build_user_prompt(&self, request: &SuggestionRequest) -> String {
        let mut prompt = request.content.clone();

        if let Some(context) = &request.context {
            prompt = format!("Context: {}\n\nContent: {}", context, prompt);
        }

        prompt
    }

    /// Make chat completion API call
    async fn chat_completion(&self, messages: Vec<ChatMessage>) -> AppResult<(String, u32)> {
        match self.model_config.provider {
            LLMProvider::Anthropic => self.anthropic_completion(messages).await,
            LLMProvider::OpenAI => self.openai_completion(messages).await,
        }
    }

    /// Anthropic Claude API call
    async fn anthropic_completion(&self, messages: Vec<ChatMessage>) -> AppResult<(String, u32)> {
        let api_key = self.config.anthropic_api_key.as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Anthropic API key not configured")))?;

        let system_msg = messages.iter()
            .find(|m| matches!(m.role, ChatRole::System))
            .map(|m| m.content.clone())
            .unwrap_or_default();

        let user_messages: Vec<_> = messages.iter()
            .filter(|m| !matches!(m.role, ChatRole::System))
            .map(|m| AnthropicMessage {
                role: match m.role {
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = AnthropicRequest {
            model: self.model_config.model.clone(),
            max_tokens: self.model_config.max_tokens,
            system: Some(system_msg),
            messages: user_messages,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Anthropic API error: {}", error_text)));
        }

        let result: AnthropicResponse = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        let content = result.content.first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let tokens = result.usage.input_tokens + result.usage.output_tokens;

        Ok((content, tokens))
    }

    /// OpenAI API call
    async fn openai_completion(&self, messages: Vec<ChatMessage>) -> AppResult<(String, u32)> {
        let api_key = self.config.openai_api_key.as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("OpenAI API key not configured")))?;

        let openai_messages: Vec<_> = messages.iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model_config.model.clone(),
            messages: openai_messages,
            max_tokens: Some(self.model_config.max_tokens),
            temperature: Some(self.model_config.temperature),
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("OpenAI API error: {}", error_text)));
        }

        let result: OpenAIResponse = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        let content = result.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let tokens = result.usage.total_tokens;

        Ok((content, tokens))
    }

    /// Generate embeddings for semantic search
    pub async fn generate_embedding(&self, text: &str) -> AppResult<EmbeddingResponse> {
        let api_key = self.config.openai_api_key.as_ref()
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("OpenAI API key not configured")))?;

        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: text.to_string(),
        };

        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("OpenAI API error: {}", error_text)));
        }

        let result: EmbeddingAPIResponse = response.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse response: {}", e)))?;

        let embedding = result.data.first()
            .map(|d| d.embedding.clone())
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embedding: embedding.clone(),
            dimensions: embedding.len(),
            model: "text-embedding-3-small".to_string(),
        })
    }
}

// Anthropic API types
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
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// OpenAI API types
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    total_tokens: u32,
}

// Embedding API types
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingAPIResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}
