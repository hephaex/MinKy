use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// AI Agent types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Summarizer,
    Classifier,
    Translator,
    QA,
    CodeReviewer,
    Writer,
    Researcher,
    Custom,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    #[default]
    Idle,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Agent definition
#[derive(Debug, Serialize)]
pub struct Agent {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub agent_type: AgentType,
    pub system_prompt: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub tools: Vec<AgentTool>,
    pub is_active: bool,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Agent tool/capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTool {
    pub name: String,
    pub description: String,
    pub parameters: Option<serde_json::Value>,
}

/// Create agent request
#[derive(Debug, Deserialize)]
pub struct CreateAgent {
    pub name: String,
    pub description: Option<String>,
    pub agent_type: AgentType,
    pub system_prompt: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools: Option<Vec<AgentTool>>,
}

/// Update agent request
#[derive(Debug, Deserialize)]
pub struct UpdateAgent {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub tools: Option<Vec<AgentTool>>,
    pub is_active: Option<bool>,
}

/// Agent execution request
#[derive(Debug, Deserialize)]
pub struct ExecuteAgentRequest {
    pub input: String,
    pub context: Option<serde_json::Value>,
    pub document_ids: Option<Vec<uuid::Uuid>>,
    pub stream: Option<bool>,
}

/// Agent execution task
#[derive(Debug, Serialize)]
pub struct AgentTask {
    pub id: String,
    pub agent_id: i32,
    pub user_id: i32,
    pub status: AgentStatus,
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub tokens_used: Option<i32>,
    pub execution_time_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Agent execution result
#[derive(Debug, Serialize)]
pub struct AgentResult {
    pub task_id: String,
    pub output: String,
    pub tokens_used: i32,
    pub execution_time_ms: i64,
    pub tool_calls: Vec<ToolCall>,
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub execution_time_ms: i64,
}

/// Agent conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Agent conversation
#[derive(Debug, Serialize)]
pub struct AgentConversation {
    pub id: String,
    pub agent_id: i32,
    pub user_id: i32,
    pub messages: Vec<AgentMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_status_default_is_idle() {
        assert!(matches!(AgentStatus::default(), AgentStatus::Idle));
    }

    #[test]
    fn test_agent_status_serde_all_variants() {
        let variants = [
            (AgentStatus::Idle, "idle"),
            (AgentStatus::Running, "running"),
            (AgentStatus::Completed, "completed"),
            (AgentStatus::Failed, "failed"),
            (AgentStatus::Cancelled, "cancelled"),
        ];
        for (status, expected) in &variants {
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, *expected);
        }
    }

    #[test]
    fn test_agent_type_serde_snake_case() {
        let t = AgentType::CodeReviewer;
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json, "code_reviewer");
    }

    #[test]
    fn test_agent_type_qa_snake_case() {
        let t = AgentType::QA;
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json, "q_a");
    }

    #[test]
    fn test_agent_type_custom_roundtrip() {
        let t = AgentType::Custom;
        let json = serde_json::to_string(&t).unwrap();
        let back: AgentType = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, AgentType::Custom));
    }

    #[test]
    fn test_message_role_lowercase() {
        let roles = [
            (MessageRole::System, "system"),
            (MessageRole::User, "user"),
            (MessageRole::Assistant, "assistant"),
            (MessageRole::Tool, "tool"),
        ];
        for (role, expected) in &roles {
            let json = serde_json::to_value(role).unwrap();
            assert_eq!(json, *expected);
        }
    }

    #[test]
    fn test_agent_tool_serde_roundtrip() {
        let tool = AgentTool {
            name: "search".to_string(),
            description: "Search the knowledge base".to_string(),
            parameters: Some(serde_json::json!({"query": "string"})),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: AgentTool = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "search");
        assert!(back.parameters.is_some());
    }

    #[test]
    fn test_execute_agent_request_defaults() {
        let json = r#"{"input": "Summarize this document"}"#;
        let req: ExecuteAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.input, "Summarize this document");
        assert!(req.context.is_none());
        assert!(req.document_ids.is_none());
        assert!(req.stream.is_none());
    }

    #[test]
    fn test_agent_message_no_tool_calls() {
        let msg = AgentMessage {
            role: MessageRole::User,
            content: "What is RAG?".to_string(),
            tool_calls: None,
        };
        assert!(matches!(msg.role, MessageRole::User));
        assert!(msg.tool_calls.is_none());
    }
}
