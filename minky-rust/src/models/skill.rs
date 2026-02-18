use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Development skill types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    CodeReviewer,
    Debugger,
    Refactorer,
    Architect,
    TddGuide,
    DocWriter,
    Planner,
    SecurityReviewer,
    PerformanceAnalyzer,
    TestGenerator,
    ApiDesigner,
    DatabaseDesigner,
    Custom(String),
}

impl Default for SkillType {
    fn default() -> Self {
        Self::CodeReviewer
    }
}

/// Skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub skill_type: SkillType,
    pub description: String,
    pub system_prompt: String,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub triggers: Vec<SkillTrigger>,
    pub capabilities: Vec<SkillCapability>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub is_active: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Skill trigger - when to activate the skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrigger {
    pub trigger_type: TriggerType,
    pub pattern: Option<String>,
    pub keywords: Vec<String>,
    pub file_patterns: Vec<String>,
}

/// Trigger type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Keyword,
    FilePattern,
    ErrorCode,
    Command,
    Automatic,
    Manual,
}

/// Skill capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCapability {
    ReadFiles,
    WriteFiles,
    ExecuteCommands,
    SearchCode,
    AnalyzeCode,
    GenerateCode,
    RunTests,
    AccessDatabase,
    CallExternalApis,
    ManageGit,
}

/// Skill execution request
#[derive(Debug, Deserialize)]
pub struct ExecuteSkillRequest {
    pub skill_id: Option<String>,
    pub skill_type: Option<SkillType>,
    pub input: String,
    pub context: Option<SkillContext>,
    pub options: Option<SkillExecutionOptions>,
}

/// Skill context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub document_ids: Option<Vec<uuid::Uuid>>,
    pub file_paths: Option<Vec<String>>,
    pub code_snippet: Option<String>,
    pub language: Option<String>,
    pub error_message: Option<String>,
    pub previous_output: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Skill execution options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionOptions {
    pub stream: Option<bool>,
    pub max_iterations: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub include_reasoning: Option<bool>,
    pub output_format: Option<OutputFormat>,
}

/// Output format
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Text,
    Markdown,
    Json,
    Code,
    Diff,
}

/// Skill execution result
#[derive(Debug, Serialize)]
pub struct SkillResult {
    pub execution_id: String,
    pub skill_id: String,
    pub skill_type: SkillType,
    pub output: String,
    pub reasoning: Option<String>,
    pub suggestions: Vec<SkillSuggestion>,
    pub code_changes: Vec<CodeChange>,
    pub follow_up_skills: Vec<String>,
    pub confidence: f32,
    pub tokens_used: i32,
    pub execution_time_ms: i64,
    pub created_at: DateTime<Utc>,
}

/// Skill suggestion
#[derive(Debug, Clone, Serialize)]
pub struct SkillSuggestion {
    pub suggestion_type: SkillSuggestionType,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub file_path: Option<String>,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub code_snippet: Option<String>,
    pub fix: Option<String>,
}

/// Suggestion type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSuggestionType {
    BugFix,
    SecurityIssue,
    PerformanceImprovement,
    CodeStyle,
    Refactoring,
    Documentation,
    TestCoverage,
    BestPractice,
    Deprecation,
}

/// Priority level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Code change
#[derive(Debug, Clone, Serialize)]
pub struct CodeChange {
    pub file_path: String,
    pub change_type: ChangeType,
    pub original: Option<String>,
    pub modified: String,
    pub line_start: i32,
    pub line_end: i32,
    pub description: String,
}

/// Change type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Add,
    Modify,
    Delete,
    Rename,
    Move,
}

/// Create skill request
#[derive(Debug, Deserialize)]
pub struct CreateSkill {
    pub name: String,
    pub skill_type: SkillType,
    pub description: String,
    pub system_prompt: String,
    pub triggers: Option<Vec<SkillTrigger>>,
    pub capabilities: Option<Vec<SkillCapability>>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub priority: Option<i32>,
}

/// Update skill request
#[derive(Debug, Deserialize)]
pub struct UpdateSkill {
    pub name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub triggers: Option<Vec<SkillTrigger>>,
    pub capabilities: Option<Vec<SkillCapability>>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub priority: Option<i32>,
    pub is_active: Option<bool>,
}

/// Skill registry - collection of available skills
#[derive(Debug, Serialize)]
pub struct SkillRegistry {
    pub skills: Vec<Skill>,
    pub total: i64,
    pub by_type: HashMap<String, i64>,
}

/// Skill execution history
#[derive(Debug, Serialize)]
pub struct SkillExecutionHistory {
    pub execution_id: String,
    pub skill_id: String,
    pub skill_type: SkillType,
    pub user_id: i32,
    pub input_summary: String,
    pub output_summary: String,
    pub success: bool,
    pub tokens_used: i32,
    pub execution_time_ms: i64,
    pub created_at: DateTime<Utc>,
}

/// Skill stats
#[derive(Debug, Serialize)]
pub struct SkillStats {
    pub total_executions: i64,
    pub successful_executions: i64,
    pub failed_executions: i64,
    pub total_tokens_used: i64,
    pub avg_execution_time_ms: f64,
    pub by_skill_type: HashMap<String, i64>,
    pub most_used_skills: Vec<(String, i64)>,
}

/// Built-in skill prompts
pub mod builtin_prompts {
    pub const CODE_REVIEWER: &str = r#"You are an expert code reviewer. Analyze the provided code for:
1. Code quality and readability
2. Potential bugs and edge cases
3. Security vulnerabilities
4. Performance issues
5. Best practices and design patterns

Provide specific, actionable feedback with line numbers where applicable.
Format: Use markdown with code blocks for examples."#;

    pub const DEBUGGER: &str = r#"You are an expert debugger. Given an error or unexpected behavior:
1. Analyze the error message and stack trace
2. Identify the root cause
3. Suggest specific fixes with code examples
4. Explain why the error occurred
5. Recommend preventive measures

Be precise and provide working solutions."#;

    pub const REFACTORER: &str = r#"You are a code refactoring specialist. Your task is to:
1. Identify code smells and anti-patterns
2. Suggest refactoring opportunities
3. Provide refactored code with explanations
4. Maintain functionality while improving structure
5. Follow SOLID principles and clean code practices

Output the refactored code with clear before/after comparisons."#;

    pub const ARCHITECT: &str = r#"You are a software architect. Help with:
1. System design and architecture decisions
2. Component interactions and dependencies
3. Scalability and performance considerations
4. Technology stack recommendations
5. Trade-off analysis

Provide diagrams descriptions and clear architectural guidance."#;

    pub const TDD_GUIDE: &str = r#"You are a TDD (Test-Driven Development) expert. Guide users through:
1. Writing failing tests first (RED)
2. Implementing minimal code to pass (GREEN)
3. Refactoring while keeping tests green (REFACTOR)
4. Test coverage analysis
5. Test organization and naming

Generate test code with clear assertions and edge cases."#;

    pub const DOC_WRITER: &str = r#"You are a technical documentation specialist. Create:
1. Clear and concise documentation
2. API documentation with examples
3. Code comments and docstrings
4. README files and guides
5. Architecture documentation

Use appropriate formatting (markdown, JSDoc, rustdoc, etc.)."#;

    pub const PLANNER: &str = r#"You are a development planner. Help with:
1. Breaking down features into tasks
2. Estimating complexity and dependencies
3. Identifying risks and blockers
4. Creating implementation roadmaps
5. Prioritizing work items

Output structured plans with clear milestones."#;

    pub const SECURITY_REVIEWER: &str = r#"You are a security expert. Analyze code for:
1. OWASP Top 10 vulnerabilities
2. Authentication and authorization issues
3. Input validation and sanitization
4. Secrets and credential handling
5. Secure coding practices

Flag issues with severity levels and remediation steps."#;

    pub const PERFORMANCE_ANALYZER: &str = r#"You are a performance optimization expert. Analyze:
1. Algorithm complexity (Big O)
2. Memory usage and leaks
3. Database query optimization
4. Caching opportunities
5. Async/parallel processing

Provide benchmarks and optimization suggestions."#;

    pub const TEST_GENERATOR: &str = r#"You are a test generation specialist. Generate:
1. Unit tests with edge cases
2. Integration tests
3. Property-based tests
4. Mock and stub implementations
5. Test fixtures and factories

Aim for high coverage with meaningful assertions."#;
}
