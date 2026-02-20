use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use std::collections::HashMap;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    models::{
        builtin_prompts, CodeChange, CreateSkill, ExecuteSkillRequest,
        Priority, Skill, SkillCapability, SkillExecutionHistory,
        SkillRegistry, SkillResult, SkillStats, SkillSuggestion, SkillTrigger, SkillType,
        SkillSuggestionType, TriggerType, UpdateSkill,
    },
    services::AIService,
};

/// Raw DB row type for skill queries
type SkillRow = (
    String,
    String,
    String,
    String,
    String,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    Option<serde_json::Value>,
    String,
    f32,
    i32,
    bool,
    i32,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// Raw DB row type for skill execution history queries
/// Columns: execution_id, skill_id, skill_type, user_id, input_summary, output_summary,
///          success, tokens_used, execution_time_ms, created_at
type SkillHistoryRow = (
    String,
    String,
    String,
    i32,
    String,
    String,
    bool,
    i32,
    i64,
    chrono::DateTime<chrono::Utc>,
);

/// Skill service for managing and executing development skills
pub struct SkillService {
    db: PgPool,
    config: Config,
    builtin_skills: HashMap<SkillType, Skill>,
}

impl SkillService {
    pub fn new(db: PgPool, config: Config) -> Self {
        let mut service = Self {
            db,
            config,
            builtin_skills: HashMap::new(),
        };
        service.register_builtin_skills();
        service
    }

    /// Register built-in development skills
    fn register_builtin_skills(&mut self) {
        let skills = vec![
            self.create_builtin_skill(
                SkillType::CodeReviewer,
                "Code Reviewer",
                "Expert code review for quality, security, and performance",
                builtin_prompts::CODE_REVIEWER,
                vec!["review", "check", "analyze", "코드리뷰"],
                vec!["*.rs", "*.py", "*.ts", "*.js"],
            ),
            self.create_builtin_skill(
                SkillType::Debugger,
                "Debugger",
                "Debug errors and find root causes",
                builtin_prompts::DEBUGGER,
                vec!["debug", "error", "fix", "에러", "버그"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::Refactorer,
                "Refactorer",
                "Refactor code for better structure and readability",
                builtin_prompts::REFACTORER,
                vec!["refactor", "clean", "improve", "리팩토링"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::Architect,
                "Architect",
                "System design and architecture decisions",
                builtin_prompts::ARCHITECT,
                vec!["design", "architect", "structure", "설계", "아키텍처"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::TddGuide,
                "TDD Guide",
                "Test-driven development guidance",
                builtin_prompts::TDD_GUIDE,
                vec!["test", "tdd", "coverage", "테스트"],
                vec!["*_test.rs", "*.test.ts", "test_*.py"],
            ),
            self.create_builtin_skill(
                SkillType::DocWriter,
                "Documentation Writer",
                "Generate technical documentation",
                builtin_prompts::DOC_WRITER,
                vec!["document", "readme", "docs", "문서"],
                vec!["*.md", "README*"],
            ),
            self.create_builtin_skill(
                SkillType::Planner,
                "Development Planner",
                "Plan features and break down tasks",
                builtin_prompts::PLANNER,
                vec!["plan", "breakdown", "estimate", "계획"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::SecurityReviewer,
                "Security Reviewer",
                "Security vulnerability analysis",
                builtin_prompts::SECURITY_REVIEWER,
                vec!["security", "vulnerability", "audit", "보안"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::PerformanceAnalyzer,
                "Performance Analyzer",
                "Performance optimization analysis",
                builtin_prompts::PERFORMANCE_ANALYZER,
                vec!["performance", "optimize", "slow", "성능"],
                vec![],
            ),
            self.create_builtin_skill(
                SkillType::TestGenerator,
                "Test Generator",
                "Generate comprehensive tests",
                builtin_prompts::TEST_GENERATOR,
                vec!["generate test", "write test", "테스트 생성"],
                vec![],
            ),
        ];

        for skill in skills {
            self.builtin_skills.insert(skill.skill_type.clone(), skill);
        }
    }

    fn create_builtin_skill(
        &self,
        skill_type: SkillType,
        name: &str,
        description: &str,
        system_prompt: &str,
        keywords: Vec<&str>,
        file_patterns: Vec<&str>,
    ) -> Skill {
        let now = Utc::now();
        Skill {
            id: format!("builtin_{:?}", skill_type).to_lowercase(),
            name: name.to_string(),
            skill_type,
            description: description.to_string(),
            system_prompt: system_prompt.to_string(),
            input_schema: None,
            output_schema: None,
            triggers: vec![SkillTrigger {
                trigger_type: TriggerType::Keyword,
                pattern: None,
                keywords: keywords.into_iter().map(String::from).collect(),
                file_patterns: file_patterns.into_iter().map(String::from).collect(),
            }],
            capabilities: vec![
                SkillCapability::ReadFiles,
                SkillCapability::AnalyzeCode,
                SkillCapability::GenerateCode,
            ],
            model: "claude-sonnet-4-20250514".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            is_active: true,
            priority: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get skill registry with all available skills
    pub async fn get_registry(&self) -> Result<SkillRegistry> {
        let mut skills: Vec<Skill> = self.builtin_skills.values().cloned().collect();

        // Load custom skills from database
        let custom_skills = self.list_custom_skills().await?;
        skills.extend(custom_skills);

        let total = skills.len() as i64;

        let mut by_type: HashMap<String, i64> = HashMap::new();
        for skill in &skills {
            let type_name = format!("{:?}", skill.skill_type);
            *by_type.entry(type_name).or_insert(0) += 1;
        }

        Ok(SkillRegistry {
            skills,
            total,
            by_type,
        })
    }

    /// Get skill by ID
    pub async fn get_skill(&self, skill_id: &str) -> Result<Option<Skill>> {
        // Check builtin skills first
        for skill in self.builtin_skills.values() {
            if skill.id == skill_id {
                return Ok(Some(skill.clone()));
            }
        }

        // Check custom skills in database
        self.get_custom_skill(skill_id).await
    }

    /// Get skill by type
    pub fn get_skill_by_type(&self, skill_type: &SkillType) -> Option<&Skill> {
        self.builtin_skills.get(skill_type)
    }

    /// Find matching skill based on input
    pub fn find_matching_skill(&self, input: &str) -> Option<&Skill> {
        let input_lower = input.to_lowercase();

        for skill in self.builtin_skills.values() {
            for trigger in &skill.triggers {
                for keyword in &trigger.keywords {
                    if input_lower.contains(&keyword.to_lowercase()) {
                        return Some(skill);
                    }
                }
            }
        }

        None
    }

    /// Execute a skill
    pub async fn execute_skill(
        &self,
        user_id: i32,
        request: ExecuteSkillRequest,
    ) -> AppResult<SkillResult> {
        let start_time = std::time::Instant::now();
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Find the skill to execute
        let skill = if let Some(skill_id) = &request.skill_id {
            self.get_skill(skill_id)
                .await
                .map_err(AppError::Internal)?
                .ok_or_else(|| AppError::NotFound("Skill not found".to_string()))?
        } else if let Some(skill_type) = &request.skill_type {
            self.get_skill_by_type(skill_type)
                .cloned()
                .ok_or_else(|| AppError::NotFound("Skill type not found".to_string()))?
        } else {
            // Auto-detect skill based on input
            self.find_matching_skill(&request.input)
                .cloned()
                .ok_or_else(|| {
                    AppError::Validation("Could not determine skill type from input".to_string())
                })?
        };

        if !skill.is_active {
            return Err(AppError::Validation("Skill is not active".to_string()));
        }

        // Build the prompt with context
        let prompt = self.build_prompt(&skill, &request);

        // Execute with AI service
        let ai_service = AIService::new(self.config.clone());
        let suggestion_request = crate::models::SuggestionRequest {
            content: prompt,
            suggestion_type: crate::models::SuggestionType::Improve,
            context: request.context.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default()),
        };

        let response = ai_service.generate_suggestion(suggestion_request).await?;

        let execution_time = start_time.elapsed().as_millis() as i64;

        // Parse the output for suggestions and code changes
        let (suggestions, code_changes) = self.parse_output(&response.suggestion, &skill);

        // Determine follow-up skills
        let follow_up_skills = self.suggest_follow_up_skills(&skill, &response.suggestion);

        let result = SkillResult {
            execution_id: execution_id.clone(),
            skill_id: skill.id.clone(),
            skill_type: skill.skill_type.clone(),
            output: response.suggestion,
            reasoning: if request.options.as_ref().and_then(|o| o.include_reasoning).unwrap_or(false) {
                Some("Analysis based on provided context and code patterns.".to_string())
            } else {
                None
            },
            suggestions,
            code_changes,
            follow_up_skills,
            confidence: 0.85, // Default confidence score
            tokens_used: response.tokens_used as i32,
            execution_time_ms: execution_time,
            created_at: Utc::now(),
        };

        // Log execution history
        self.log_execution(user_id, &skill, &result).await?;

        Ok(result)
    }

    fn build_prompt(&self, skill: &Skill, request: &ExecuteSkillRequest) -> String {
        let mut prompt = format!("{}\n\n", skill.system_prompt);

        if let Some(context) = &request.context {
            if let Some(code) = &context.code_snippet {
                prompt.push_str(&format!("Code to analyze:\n```\n{}\n```\n\n", code));
            }
            if let Some(error) = &context.error_message {
                prompt.push_str(&format!("Error message:\n{}\n\n", error));
            }
            if let Some(lang) = &context.language {
                prompt.push_str(&format!("Programming language: {}\n\n", lang));
            }
            if let Some(prev) = &context.previous_output {
                prompt.push_str(&format!("Previous analysis:\n{}\n\n", prev));
            }
        }

        prompt.push_str(&format!("User request: {}", request.input));
        prompt
    }

    fn parse_output(&self, output: &str, skill: &Skill) -> (Vec<SkillSuggestion>, Vec<CodeChange>) {
        let mut suggestions = Vec::new();
        let code_changes = Vec::new();

        // Simple parsing - in production, use more sophisticated parsing
        let lines: Vec<&str> = output.lines().collect();
        let mut current_suggestion: Option<SkillSuggestion> = None;

        for line in lines {
            // Look for suggestion markers
            if line.contains("CRITICAL:") || line.contains("🔴") {
                if let Some(s) = current_suggestion.take() {
                    suggestions.push(s);
                }
                current_suggestion = Some(SkillSuggestion {
                    suggestion_type: self.infer_suggestion_type(skill),
                    title: line.replace("CRITICAL:", "").replace("🔴", "").trim().to_string(),
                    description: String::new(),
                    priority: Priority::Critical,
                    file_path: None,
                    line_start: None,
                    line_end: None,
                    code_snippet: None,
                    fix: None,
                });
            } else if line.contains("HIGH:") || line.contains("🟠") {
                if let Some(s) = current_suggestion.take() {
                    suggestions.push(s);
                }
                current_suggestion = Some(SkillSuggestion {
                    suggestion_type: self.infer_suggestion_type(skill),
                    title: line.replace("HIGH:", "").replace("🟠", "").trim().to_string(),
                    description: String::new(),
                    priority: Priority::High,
                    file_path: None,
                    line_start: None,
                    line_end: None,
                    code_snippet: None,
                    fix: None,
                });
            } else if line.contains("MEDIUM:") || line.contains("🟡") {
                if let Some(s) = current_suggestion.take() {
                    suggestions.push(s);
                }
                current_suggestion = Some(SkillSuggestion {
                    suggestion_type: self.infer_suggestion_type(skill),
                    title: line.replace("MEDIUM:", "").replace("🟡", "").trim().to_string(),
                    description: String::new(),
                    priority: Priority::Medium,
                    file_path: None,
                    line_start: None,
                    line_end: None,
                    code_snippet: None,
                    fix: None,
                });
            } else if let Some(ref mut s) = current_suggestion {
                s.description.push_str(line);
                s.description.push('\n');
            }
        }

        if let Some(s) = current_suggestion {
            suggestions.push(s);
        }

        (suggestions, code_changes)
    }

    fn infer_suggestion_type(&self, skill: &Skill) -> SkillSuggestionType {
        match skill.skill_type {
            SkillType::CodeReviewer => SkillSuggestionType::CodeStyle,
            SkillType::Debugger => SkillSuggestionType::BugFix,
            SkillType::Refactorer => SkillSuggestionType::Refactoring,
            SkillType::SecurityReviewer => SkillSuggestionType::SecurityIssue,
            SkillType::PerformanceAnalyzer => SkillSuggestionType::PerformanceImprovement,
            SkillType::TddGuide | SkillType::TestGenerator => SkillSuggestionType::TestCoverage,
            SkillType::DocWriter => SkillSuggestionType::Documentation,
            _ => SkillSuggestionType::BestPractice,
        }
    }

    fn suggest_follow_up_skills(&self, current_skill: &Skill, _output: &str) -> Vec<String> {
        match current_skill.skill_type {
            SkillType::CodeReviewer => vec![
                "builtin_securityreviewer".to_string(),
                "builtin_performanceanalyzer".to_string(),
            ],
            SkillType::Debugger => vec![
                "builtin_testgenerator".to_string(),
            ],
            SkillType::Refactorer => vec![
                "builtin_codereview".to_string(),
                "builtin_docwriter".to_string(),
            ],
            SkillType::Planner => vec![
                "builtin_architect".to_string(),
            ],
            _ => vec![],
        }
    }

    async fn log_execution(&self, user_id: i32, skill: &Skill, result: &SkillResult) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO skill_executions (
                execution_id, skill_id, skill_type, user_id, input_summary, output_summary,
                success, tokens_used, execution_time_ms, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(&result.execution_id)
        .bind(&skill.id)
        .bind(serde_json::to_string(&skill.skill_type)?)
        .bind(user_id)
        .bind(&result.output[..result.output.len().min(200)])
        .bind(&result.output[..result.output.len().min(500)])
        .bind(true)
        .bind(result.tokens_used)
        .bind(result.execution_time_ms)
        .bind(result.created_at)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// List custom skills from database
    async fn list_custom_skills(&self) -> Result<Vec<Skill>> {
        let rows: Vec<SkillRow> = sqlx::query_as(
            r#"
            SELECT id, name, skill_type, description, system_prompt, input_schema, output_schema,
                   triggers, capabilities, model, temperature, max_tokens, is_active, priority,
                   created_at, updated_at
            FROM skills
            WHERE is_active = true
            ORDER BY priority DESC, created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Skill {
                id: r.0,
                name: r.1,
                skill_type: serde_json::from_str(&r.2).unwrap_or(SkillType::Custom("unknown".to_string())),
                description: r.3,
                system_prompt: r.4,
                input_schema: r.5,
                output_schema: r.6,
                triggers: r.7.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
                capabilities: r.8.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
                model: r.9,
                temperature: r.10,
                max_tokens: r.11,
                is_active: r.12,
                priority: r.13,
                created_at: r.14,
                updated_at: r.15,
            })
            .collect())
    }

    async fn get_custom_skill(&self, skill_id: &str) -> Result<Option<Skill>> {
        let row: Option<SkillRow> = sqlx::query_as(
            r#"
            SELECT id, name, skill_type, description, system_prompt, input_schema, output_schema,
                   triggers, capabilities, model, temperature, max_tokens, is_active, priority,
                   created_at, updated_at
            FROM skills
            WHERE id = $1
            "#,
        )
        .bind(skill_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| Skill {
            id: r.0,
            name: r.1,
            skill_type: serde_json::from_str(&r.2).unwrap_or(SkillType::Custom("unknown".to_string())),
            description: r.3,
            system_prompt: r.4,
            input_schema: r.5,
            output_schema: r.6,
            triggers: r.7.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            capabilities: r.8.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
            model: r.9,
            temperature: r.10,
            max_tokens: r.11,
            is_active: r.12,
            priority: r.13,
            created_at: r.14,
            updated_at: r.15,
        }))
    }

    /// Create custom skill
    pub async fn create_skill(&self, user_id: i32, create: CreateSkill) -> Result<Skill> {
        let skill_id = uuid::Uuid::new_v4().to_string();
        let skill_type_str = serde_json::to_string(&create.skill_type)?;
        let triggers_json = create.triggers.as_ref().map(serde_json::to_value).transpose()?;
        let capabilities_json = create.capabilities.as_ref().map(serde_json::to_value).transpose()?;

        sqlx::query(
            r#"
            INSERT INTO skills (
                id, name, skill_type, description, system_prompt, triggers, capabilities,
                model, temperature, max_tokens, is_active, priority, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, $11, $12, NOW(), NOW())
            "#,
        )
        .bind(&skill_id)
        .bind(&create.name)
        .bind(&skill_type_str)
        .bind(&create.description)
        .bind(&create.system_prompt)
        .bind(triggers_json)
        .bind(capabilities_json)
        .bind(create.model.as_deref().unwrap_or("claude-sonnet-4-20250514"))
        .bind(create.temperature.unwrap_or(0.3))
        .bind(create.max_tokens.unwrap_or(4096))
        .bind(create.priority.unwrap_or(0))
        .bind(user_id)
        .execute(&self.db)
        .await?;

        self.get_custom_skill(&skill_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create skill"))
    }

    /// Update skill
    pub async fn update_skill(&self, skill_id: &str, update: UpdateSkill) -> Result<Skill> {
        if let Some(name) = &update.name {
            sqlx::query("UPDATE skills SET name = $1, updated_at = NOW() WHERE id = $2")
                .bind(name)
                .bind(skill_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(description) = &update.description {
            sqlx::query("UPDATE skills SET description = $1, updated_at = NOW() WHERE id = $2")
                .bind(description)
                .bind(skill_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(system_prompt) = &update.system_prompt {
            sqlx::query("UPDATE skills SET system_prompt = $1, updated_at = NOW() WHERE id = $2")
                .bind(system_prompt)
                .bind(skill_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(is_active) = update.is_active {
            sqlx::query("UPDATE skills SET is_active = $1, updated_at = NOW() WHERE id = $2")
                .bind(is_active)
                .bind(skill_id)
                .execute(&self.db)
                .await?;
        }

        self.get_custom_skill(skill_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Skill not found"))
    }

    /// Delete skill
    pub async fn delete_skill(&self, skill_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM skills WHERE id = $1")
            .bind(skill_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Get skill stats
    pub async fn get_stats(&self) -> Result<SkillStats> {
        let totals: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::bigint,
                COUNT(*) FILTER (WHERE success = true)::bigint,
                COUNT(*) FILTER (WHERE success = false)::bigint,
                COALESCE(SUM(tokens_used), 0)::bigint
            FROM skill_executions
            "#,
        )
        .fetch_one(&self.db)
        .await
        .unwrap_or((0, 0, 0, 0));

        let avg_time: (Option<f64>,) = sqlx::query_as(
            "SELECT AVG(execution_time_ms::float) FROM skill_executions",
        )
        .fetch_one(&self.db)
        .await
        .unwrap_or((None,));

        Ok(SkillStats {
            total_executions: totals.0,
            successful_executions: totals.1,
            failed_executions: totals.2,
            total_tokens_used: totals.3,
            avg_execution_time_ms: avg_time.0.unwrap_or(0.0),
            by_skill_type: HashMap::new(),
            most_used_skills: vec![],
        })
    }

    /// Get execution history
    pub async fn get_history(&self, user_id: i32, limit: i32) -> Result<Vec<SkillExecutionHistory>> {
        let rows: Vec<SkillHistoryRow> = sqlx::query_as(
            r#"
            SELECT execution_id, skill_id, skill_type, user_id, input_summary, output_summary,
                   success, tokens_used, execution_time_ms, created_at
            FROM skill_executions
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SkillExecutionHistory {
                execution_id: r.0,
                skill_id: r.1,
                skill_type: serde_json::from_str(&r.2).unwrap_or(SkillType::Custom("unknown".to_string())),
                user_id: r.3,
                input_summary: r.4,
                output_summary: r.5,
                success: r.6,
                tokens_used: r.7,
                execution_time_ms: r.8,
                created_at: r.9,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SkillContext;
    use secrecy::SecretString;

    fn make_service() -> SkillService {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let config = Config {
            environment: "test".to_string(),
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
            cors_allowed_origins: "http://localhost:3000".to_string(),
        };
        SkillService::new(pool, config)
    }

    #[tokio::test]
    async fn test_get_skill_by_type_code_reviewer_exists() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::CodeReviewer);
        assert!(skill.is_some(), "Built-in CodeReviewer skill should exist");
    }

    #[tokio::test]
    async fn test_get_skill_by_type_debugger_exists() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::Debugger);
        assert!(skill.is_some(), "Built-in Debugger skill should exist");
    }

    #[tokio::test]
    async fn test_get_skill_by_type_planner_exists() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::Planner);
        assert!(skill.is_some(), "Built-in Planner skill should exist");
    }

    #[tokio::test]
    async fn test_find_matching_skill_review_keyword() {
        let svc = make_service();
        let skill = svc.find_matching_skill("please review this code");
        assert!(skill.is_some(), "Input containing 'review' should match a skill");
    }

    #[tokio::test]
    async fn test_find_matching_skill_debug_keyword() {
        let svc = make_service();
        let skill = svc.find_matching_skill("I have an error in my code");
        assert!(skill.is_some(), "Input containing 'error' should match a skill");
    }

    #[tokio::test]
    async fn test_find_matching_skill_no_match_returns_none() {
        let svc = make_service();
        let skill = svc.find_matching_skill("xyzzy_nonsense_zzz_foobar");
        assert!(skill.is_none(), "Unknown input should return None");
    }

    #[tokio::test]
    async fn test_find_matching_skill_korean_trigger() {
        let svc = make_service();
        let skill = svc.find_matching_skill("코드리뷰 해줘");
        assert!(skill.is_some(), "Korean trigger '코드리뷰' should match a skill");
    }

    #[tokio::test]
    async fn test_build_prompt_includes_system_prompt() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::CodeReviewer).unwrap();
        let request = ExecuteSkillRequest {
            skill_id: None,
            skill_type: Some(SkillType::CodeReviewer),
            input: "analyze this".to_string(),
            context: None,
            options: None,
        };
        let prompt = svc.build_prompt(skill, &request);
        assert!(prompt.contains(&skill.system_prompt), "Prompt should include system prompt");
        assert!(prompt.contains("analyze this"), "Prompt should include user request");
    }

    #[tokio::test]
    async fn test_build_prompt_includes_code_snippet_when_provided() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::Debugger).unwrap();
        let request = ExecuteSkillRequest {
            skill_id: None,
            skill_type: Some(SkillType::Debugger),
            input: "fix this".to_string(),
            context: Some(SkillContext {
                document_ids: None,
                file_paths: None,
                code_snippet: Some("fn main() {}".to_string()),
                language: None,
                error_message: None,
                previous_output: None,
                metadata: None,
            }),
            options: None,
        };
        let prompt = svc.build_prompt(skill, &request);
        assert!(prompt.contains("fn main() {}"), "Prompt should include code snippet");
    }

    #[tokio::test]
    async fn test_build_prompt_includes_error_message_when_provided() {
        let svc = make_service();
        let skill = svc.get_skill_by_type(&SkillType::Debugger).unwrap();
        let request = ExecuteSkillRequest {
            skill_id: None,
            skill_type: Some(SkillType::Debugger),
            input: "debug this".to_string(),
            context: Some(SkillContext {
                document_ids: None,
                file_paths: None,
                code_snippet: None,
                language: None,
                error_message: Some("NullPointerException at line 42".to_string()),
                previous_output: None,
                metadata: None,
            }),
            options: None,
        };
        let prompt = svc.build_prompt(skill, &request);
        assert!(prompt.contains("NullPointerException"), "Prompt should include error message");
    }
}
