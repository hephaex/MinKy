use serde::{Deserialize, Serialize};

/// LLM Provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
}

impl Default for LLMProvider {
    fn default() -> Self {
        Self::Anthropic
    }
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
pub enum TimeRange {
    Day,
    Week,
    Month,
    Year,
    Custom { start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc> },
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::Month
    }
}
