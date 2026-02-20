use anyhow::Result;
use sqlx::PgPool;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{
        Agent, AgentMessage, AgentResult, AgentStatus, AgentTask, AgentTool,
        AgentType, CreateAgent, ExecuteAgentRequest, MessageRole, UpdateAgent,
    },
    services::AIService,
};

/// Raw DB row type for agent queries
type AgentRow = (
    i32,
    String,
    Option<String>,
    String,
    String,
    String,
    f32,
    i32,
    Option<serde_json::Value>,
    bool,
    i32,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// Raw DB row type for agent task queries
type AgentTaskRow = (
    String,
    i32,
    i32,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i64>,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
);

/// Agent service for AI agent management
pub struct AgentService {
    db: PgPool,
    config: Config,
}

impl AgentService {
    pub fn new(db: PgPool, config: Config) -> Self {
        Self { db, config }
    }

    /// List all agents
    pub async fn list_agents(&self, user_id: i32) -> Result<Vec<Agent>> {
        let rows: Vec<AgentRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, agent_type, system_prompt, model, temperature, max_tokens, tools, is_active, created_by, created_at, updated_at
            FROM agents
            WHERE created_by = $1 OR is_public = true
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let tools: Vec<AgentTool> = r
                    .8
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                Agent {
                    id: r.0,
                    name: r.1,
                    description: r.2,
                    agent_type: serde_json::from_str(&r.3).unwrap_or(AgentType::Custom),
                    system_prompt: r.4,
                    model: r.5,
                    temperature: r.6,
                    max_tokens: r.7,
                    tools,
                    is_active: r.9,
                    created_by: r.10,
                    created_at: r.11,
                    updated_at: r.12,
                }
            })
            .collect())
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: i32) -> Result<Option<Agent>> {
        let row: Option<AgentRow> = sqlx::query_as(
            r#"
            SELECT id, name, description, agent_type, system_prompt, model, temperature, max_tokens, tools, is_active, created_by, created_at, updated_at
            FROM agents
            WHERE id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| {
            let tools: Vec<AgentTool> = r
                .8
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            Agent {
                id: r.0,
                name: r.1,
                description: r.2,
                agent_type: serde_json::from_str(&r.3).unwrap_or(AgentType::Custom),
                system_prompt: r.4,
                model: r.5,
                temperature: r.6,
                max_tokens: r.7,
                tools,
                is_active: r.9,
                created_by: r.10,
                created_at: r.11,
                updated_at: r.12,
            }
        }))
    }

    /// Create agent
    pub async fn create_agent(&self, user_id: i32, create: CreateAgent) -> Result<Agent> {
        let tools_json = create
            .tools
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let agent_type_str = serde_json::to_string(&create.agent_type)?;

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO agents (name, description, agent_type, system_prompt, model, temperature, max_tokens, tools, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(&create.name)
        .bind(&create.description)
        .bind(&agent_type_str)
        .bind(&create.system_prompt)
        .bind(create.model.as_deref().unwrap_or("claude-sonnet-4-20250514"))
        .bind(create.temperature.unwrap_or(0.7))
        .bind(create.max_tokens.unwrap_or(4096))
        .bind(tools_json)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        self.get_agent(row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created agent"))
    }

    /// Update agent
    pub async fn update_agent(
        &self,
        user_id: i32,
        agent_id: i32,
        update: UpdateAgent,
    ) -> Result<Agent> {
        // Verify ownership
        let owner_check: Option<(i32,)> =
            sqlx::query_as("SELECT created_by FROM agents WHERE id = $1")
                .bind(agent_id)
                .fetch_optional(&self.db)
                .await?;

        if let Some((owner_id,)) = owner_check {
            if owner_id != user_id {
                return Err(anyhow::anyhow!("Not authorized to update this agent"));
            }
        } else {
            return Err(anyhow::anyhow!("Agent not found"));
        }

        if let Some(name) = &update.name {
            sqlx::query("UPDATE agents SET name = $1, updated_at = NOW() WHERE id = $2")
                .bind(name)
                .bind(agent_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(system_prompt) = &update.system_prompt {
            sqlx::query("UPDATE agents SET system_prompt = $1, updated_at = NOW() WHERE id = $2")
                .bind(system_prompt)
                .bind(agent_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(model) = &update.model {
            sqlx::query("UPDATE agents SET model = $1, updated_at = NOW() WHERE id = $2")
                .bind(model)
                .bind(agent_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(temperature) = update.temperature {
            sqlx::query("UPDATE agents SET temperature = $1, updated_at = NOW() WHERE id = $2")
                .bind(temperature)
                .bind(agent_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(is_active) = update.is_active {
            sqlx::query("UPDATE agents SET is_active = $1, updated_at = NOW() WHERE id = $2")
                .bind(is_active)
                .bind(agent_id)
                .execute(&self.db)
                .await?;
        }

        self.get_agent(agent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Agent not found"))
    }

    /// Delete agent
    pub async fn delete_agent(&self, user_id: i32, agent_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM agents WHERE id = $1 AND created_by = $2")
            .bind(agent_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Execute agent
    pub async fn execute_agent(
        &self,
        _user_id: i32,
        agent_id: i32,
        request: ExecuteAgentRequest,
    ) -> AppResult<AgentResult> {
        let agent = self
            .get_agent(agent_id)
            .await
            .map_err(AppError::Internal)?
            .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

        if !agent.is_active {
            return Err(AppError::Validation("Agent is not active".to_string()));
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        let start_time = std::time::Instant::now();

        // Build messages
        let _messages = [AgentMessage {
                role: MessageRole::System,
                content: agent.system_prompt.clone(),
                tool_calls: None,
            },
            AgentMessage {
                role: MessageRole::User,
                content: request.input.clone(),
                tool_calls: None,
            }];

        // Call AI service
        let ai_service = AIService::new(self.config.clone());
        let suggestion_request = crate::models::SuggestionRequest {
            content: request.input.clone(),
            suggestion_type: crate::models::SuggestionType::Improve,
            context: request.context.as_ref().map(|c| c.to_string()),
        };

        let response = ai_service.generate_suggestion(suggestion_request).await?;

        let execution_time = start_time.elapsed().as_millis() as i64;

        Ok(AgentResult {
            task_id,
            output: response.suggestion,
            tokens_used: response.tokens_used as i32,
            execution_time_ms: execution_time,
            tool_calls: vec![],
        })
    }

    /// Get agent tasks
    pub async fn get_tasks(&self, user_id: i32, agent_id: Option<i32>) -> Result<Vec<AgentTask>> {
        let rows: Vec<AgentTaskRow> = sqlx::query_as(
            r#"
            SELECT id, agent_id, user_id, status, input, output, error, tokens_used, execution_time_ms, created_at, completed_at
            FROM agent_tasks
            WHERE user_id = $1 AND ($2::int IS NULL OR agent_id = $2)
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .bind(user_id)
        .bind(agent_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AgentTask {
                id: r.0,
                agent_id: r.1,
                user_id: r.2,
                status: serde_json::from_str(&r.3).unwrap_or(AgentStatus::Idle),
                input: r.4,
                output: r.5,
                error: r.6,
                tokens_used: r.7,
                execution_time_ms: r.8,
                created_at: r.9,
                completed_at: r.10,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use secrecy::SecretString;

    fn make_config() -> Config {
        Config {
            host: "127.0.0.1".to_string(),
            port: 8000,
            database_url: "postgres://localhost/test_db".to_string(),
            database_max_connections: 1,
            jwt_secret: SecretString::from("test-secret"),
            jwt_expiration_hours: 24,
            opensearch_url: None,
            openai_api_key: None,
            anthropic_api_key: None,
            git_repo_path: None,
            slack_client_id: None,
            slack_client_secret: None,
            slack_redirect_uri: None,
            slack_signing_secret: None,
        }
    }

    fn make_service() -> AgentService {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        AgentService::new(pool, make_config())
    }

    #[test]
    fn test_agent_row_structure() {
        let now = Utc::now();
        let row: AgentRow = (
            1,
            "TestAgent".to_string(),
            Some("A test agent".to_string()),
            "custom".to_string(),
            "You are a test agent".to_string(),
            "claude-3-sonnet".to_string(),
            0.7,
            4096,
            None,
            true,
            100,
            now,
            now,
        );

        assert_eq!(row.0, 1);
        assert_eq!(row.1, "TestAgent");
        assert_eq!(row.6, 0.7);
        assert!(row.9);
    }

    #[test]
    fn test_create_agent_structure() {
        let create = CreateAgent {
            name: "MyAgent".to_string(),
            description: Some("My agent".to_string()),
            agent_type: AgentType::Custom,
            system_prompt: "System message".to_string(),
            model: Some("claude-3-opus".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(2000),
            tools: None,
        };

        assert_eq!(create.name, "MyAgent");
        assert_eq!(create.temperature, Some(0.5));
    }

    #[test]
    fn test_agent_structure() {
        let now = Utc::now();
        let agent = Agent {
            id: 42,
            name: "TestAgent".to_string(),
            description: Some("Test".to_string()),
            agent_type: AgentType::CodeReviewer,
            system_prompt: "Review code".to_string(),
            model: "claude-3-sonnet".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            tools: vec![],
            is_active: true,
            created_by: 1,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(agent.id, 42);
        assert_eq!(agent.temperature, 0.3);
        assert!(agent.is_active);
    }

    #[test]
    fn test_agent_task_row_structure() {
        let now = Utc::now();
        let row: AgentTaskRow = (
            "task-1".to_string(),
            1,
            10,
            "running".to_string(),
            "input text".to_string(),
            Some("output text".to_string()),
            None,
            Some(500),
            Some(1500),
            now,
            Some(now),
        );

        assert_eq!(row.0, "task-1");
        assert_eq!(row.1, 1);
        assert_eq!(row.4, "input text");
    }

    #[test]
    fn test_agent_task_structure() {
        let now = Utc::now();
        let task = AgentTask {
            id: "task-123".to_string(),
            agent_id: 5,
            user_id: 10,
            status: AgentStatus::Running,
            input: "Task input".to_string(),
            output: Some("Task output".to_string()),
            error: None,
            tokens_used: Some(250),
            execution_time_ms: Some(2000),
            created_at: now,
            completed_at: Some(now),
        };

        assert_eq!(task.agent_id, 5);
        assert!(task.error.is_none());
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_agent_message_structure() {
        let msg = AgentMessage {
            role: MessageRole::User,
            content: "Hello agent".to_string(),
            tool_calls: None,
        };

        assert_eq!(msg.content, "Hello agent");
        assert!(matches!(msg.role, MessageRole::User));
    }

    #[test]
    fn test_agent_result_structure() {
        let result = AgentResult {
            task_id: "task-5".to_string(),
            output: "Result output".to_string(),
            tokens_used: 300,
            execution_time_ms: 2000,
            tool_calls: vec![],
        };

        assert_eq!(result.task_id, "task-5");
        assert_eq!(result.output, "Result output");
        assert_eq!(result.tokens_used, 300);
    }

    #[test]
    fn test_update_agent_partial() {
        let update = UpdateAgent {
            name: Some("NewName".to_string()),
            description: None,
            system_prompt: None,
            temperature: None,
            model: None,
            max_tokens: None,
            tools: None,
            is_active: None,
        };

        assert_eq!(update.name, Some("NewName".to_string()));
        assert!(update.temperature.is_none());
    }

    #[test]
    fn test_execute_agent_request_structure() {
        let req = ExecuteAgentRequest {
            input: "Process this".to_string(),
            context: None,
            document_ids: None,
            stream: Some(true),
        };

        assert_eq!(req.input, "Process this");
        assert_eq!(req.stream, Some(true));
    }

    #[test]
    fn test_agent_types() {
        let _custom = AgentType::Custom;
        let _reviewer = AgentType::CodeReviewer;
        let _summarizer = AgentType::Summarizer;

        // Just verify they exist
        assert!(matches!(AgentType::Custom, AgentType::Custom));
        assert!(matches!(AgentType::CodeReviewer, AgentType::CodeReviewer));
    }

    #[test]
    fn test_agent_statuses() {
        let idle = AgentStatus::Idle;
        let running = AgentStatus::Running;

        assert!(matches!(idle, AgentStatus::Idle));
        assert!(matches!(running, AgentStatus::Running));
    }

    #[test]
    fn test_message_roles() {
        let user = MessageRole::User;
        let assistant = MessageRole::Assistant;

        assert!(matches!(user, MessageRole::User));
        assert!(matches!(assistant, MessageRole::Assistant));
    }

    #[test]
    fn test_agent_tool_structure() {
        let tool = AgentTool {
            name: "TestTool".to_string(),
            description: "A test tool".to_string(),
            parameters: None,
        };

        assert_eq!(tool.name, "TestTool");
        assert_eq!(tool.description, "A test tool");
    }

    #[test]
    fn test_execute_agent_request_with_documents() {
        let req = ExecuteAgentRequest {
            input: "Analyze docs".to_string(),
            context: None,
            document_ids: Some(vec![]),
            stream: Some(false),
        };

        assert_eq!(req.input, "Analyze docs");
        assert!(req.document_ids.is_some());
    }
}
