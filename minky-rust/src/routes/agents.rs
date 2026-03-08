use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
use crate::models::{Agent, AgentResult, AgentTask, CreateAgent, ExecuteAgentRequest, UpdateAgent};
use crate::services::AgentService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub agent_id: Option<i32>,
}

/// List all agents
async fn list_agents(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<Vec<Agent>>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .list_agents(auth_user.id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get agent by ID
async fn get_agent(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(agent_id): Path<i32>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .get_agent(agent_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Agent not found".to_string()))
}

/// Create agent
async fn create_agent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(create): Json<CreateAgent>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .create_agent(auth_user.id, create)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update agent
async fn update_agent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(agent_id): Path<i32>,
    Json(update): Json<UpdateAgent>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .update_agent(auth_user.id, agent_id, update)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete agent
async fn delete_agent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(agent_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .delete_agent(auth_user.id, agent_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Execute agent
async fn execute_agent(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(agent_id): Path<i32>,
    Json(request): Json<ExecuteAgentRequest>,
) -> Result<Json<AgentResult>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .execute_agent(auth_user.id, agent_id, request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// List tasks
async fn list_tasks(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<AgentTask>>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    service
        .get_tasks(auth_user.id, query.agent_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_agents).post(create_agent))
        .route("/{agent_id}", get(get_agent).put(update_agent).delete(delete_agent))
        .route("/{agent_id}/execute", post(execute_agent))
        .route("/tasks", get(list_tasks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentStatus, AgentTool, AgentType, ToolCall};

    // -------------------------------------------------------------------------
    // ListTasksQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_list_tasks_query_deserialization() {
        let json = r#"{"agent_id": 5}"#;
        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.agent_id, Some(5));
    }

    #[test]
    fn test_list_tasks_query_empty() {
        let json = r#"{}"#;
        let query: ListTasksQuery = serde_json::from_str(json).unwrap();
        assert!(query.agent_id.is_none());
    }

    // -------------------------------------------------------------------------
    // Agent serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_agent_serialization() {
        let now = chrono::Utc::now();
        let agent = Agent {
            id: 1,
            name: "Summarizer Agent".to_string(),
            description: Some("Summarizes documents".to_string()),
            agent_type: AgentType::Summarizer,
            system_prompt: "You are a helpful summarizer.".to_string(),
            model: "claude-3-haiku".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
            tools: vec![],
            is_active: true,
            created_by: 1,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"name\":\"Summarizer Agent\""));
        assert!(json.contains("\"agent_type\":\"summarizer\""));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn test_agent_with_tools() {
        let now = chrono::Utc::now();
        let agent = Agent {
            id: 2,
            name: "Researcher Agent".to_string(),
            description: None,
            agent_type: AgentType::Researcher,
            system_prompt: "You research topics.".to_string(),
            model: "claude-3-sonnet".to_string(),
            temperature: 0.5,
            max_tokens: 4000,
            tools: vec![
                AgentTool {
                    name: "web_search".to_string(),
                    description: "Search the web".to_string(),
                    parameters: Some(serde_json::json!({"query": "string"})),
                },
                AgentTool {
                    name: "read_document".to_string(),
                    description: "Read a document".to_string(),
                    parameters: None,
                },
            ],
            is_active: true,
            created_by: 1,
            created_at: now,
            updated_at: now,
        };
        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("\"tools\":["));
        assert!(json.contains("\"web_search\""));
        assert!(json.contains("\"read_document\""));
    }

    // -------------------------------------------------------------------------
    // CreateAgent tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_agent_minimal() {
        let json = r#"{
            "name": "My Agent",
            "agent_type": "custom",
            "system_prompt": "You are a helpful assistant."
        }"#;
        let create: CreateAgent = serde_json::from_str(json).unwrap();
        assert_eq!(create.name, "My Agent");
        assert!(matches!(create.agent_type, AgentType::Custom));
        assert!(create.model.is_none());
        assert!(create.temperature.is_none());
    }

    #[test]
    fn test_create_agent_full() {
        let json = r#"{
            "name": "Code Reviewer",
            "description": "Reviews code for issues",
            "agent_type": "code_reviewer",
            "system_prompt": "Review the following code.",
            "model": "claude-3-opus",
            "temperature": 0.3,
            "max_tokens": 2000,
            "tools": [{"name": "analyze", "description": "Analyze code"}]
        }"#;
        let create: CreateAgent = serde_json::from_str(json).unwrap();
        assert_eq!(create.name, "Code Reviewer");
        assert_eq!(create.model, Some("claude-3-opus".to_string()));
        assert_eq!(create.temperature, Some(0.3));
        assert!(create.tools.is_some());
    }

    // -------------------------------------------------------------------------
    // UpdateAgent tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_update_agent_partial() {
        let json = r#"{"name": "Updated Name"}"#;
        let update: UpdateAgent = serde_json::from_str(json).unwrap();
        assert_eq!(update.name, Some("Updated Name".to_string()));
        assert!(update.system_prompt.is_none());
        assert!(update.model.is_none());
    }

    #[test]
    fn test_update_agent_deactivate() {
        let json = r#"{"is_active": false}"#;
        let update: UpdateAgent = serde_json::from_str(json).unwrap();
        assert_eq!(update.is_active, Some(false));
    }

    // -------------------------------------------------------------------------
    // ExecuteAgentRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_execute_agent_request_minimal() {
        let json = r#"{"input": "Hello world"}"#;
        let request: ExecuteAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.input, "Hello world");
        assert!(request.context.is_none());
        assert!(request.stream.is_none());
    }

    #[test]
    fn test_execute_agent_request_with_context() {
        let json = r#"{
            "input": "Summarize this",
            "context": {"topic": "AI"},
            "document_ids": ["550e8400-e29b-41d4-a716-446655440000"],
            "stream": true
        }"#;
        let request: ExecuteAgentRequest = serde_json::from_str(json).unwrap();
        assert!(request.context.is_some());
        assert!(request.document_ids.is_some());
        assert_eq!(request.stream, Some(true));
    }

    // -------------------------------------------------------------------------
    // AgentTask serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_agent_task_serialization() {
        let now = chrono::Utc::now();
        let task = AgentTask {
            id: "task-123".to_string(),
            agent_id: 1,
            user_id: 5,
            status: AgentStatus::Completed,
            input: "Summarize this document".to_string(),
            output: Some("This is a summary.".to_string()),
            error: None,
            tokens_used: Some(150),
            execution_time_ms: Some(1500),
            created_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"id\":\"task-123\""));
        assert!(json.contains("\"status\":\"completed\""));
        assert!(json.contains("\"tokens_used\":150"));
    }

    #[test]
    fn test_agent_task_failed() {
        let now = chrono::Utc::now();
        let task = AgentTask {
            id: "task-456".to_string(),
            agent_id: 2,
            user_id: 3,
            status: AgentStatus::Failed,
            input: "Invalid request".to_string(),
            output: None,
            error: Some("Model unavailable".to_string()),
            tokens_used: None,
            execution_time_ms: Some(100),
            created_at: now,
            completed_at: Some(now),
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"status\":\"failed\""));
        assert!(json.contains("\"error\":\"Model unavailable\""));
    }

    // -------------------------------------------------------------------------
    // AgentResult serialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_agent_result_serialization() {
        let result = AgentResult {
            task_id: "task-789".to_string(),
            output: "Here is your answer.".to_string(),
            tokens_used: 200,
            execution_time_ms: 2000,
            tool_calls: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"task_id\":\"task-789\""));
        assert!(json.contains("\"tokens_used\":200"));
    }

    #[test]
    fn test_agent_result_with_tool_calls() {
        let result = AgentResult {
            task_id: "task-abc".to_string(),
            output: "Found relevant documents.".to_string(),
            tokens_used: 500,
            execution_time_ms: 5000,
            tool_calls: vec![
                ToolCall {
                    tool_name: "search".to_string(),
                    input: serde_json::json!({"query": "AI trends"}),
                    output: serde_json::json!({"results": []}),
                    execution_time_ms: 1000,
                },
            ],
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"tool_calls\":["));
        assert!(json.contains("\"tool_name\":\"search\""));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_router_creation() {
        let _router: Router<AppState> = router();
        // Should be creatable without panicking
    }
}
