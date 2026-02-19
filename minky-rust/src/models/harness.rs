use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use super::security::Severity;

/// Issue harness workflow status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HarnessStatus {
    #[default]
    Pending,
    Analyzing,
    Planning,
    Executing,
    Verifying,
    Committing,
    Completed,
    Failed,
    Cancelled,
}

/// GitHub issue for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: i32,
    pub title: String,
    pub body: Option<String>,
    pub labels: Vec<String>,
    pub state: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub html_url: String,
}

/// Issue harness workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueHarness {
    pub id: String,
    pub issue_number: i32,
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub status: HarnessStatus,
    pub current_phase: HarnessPhase,
    pub phases: Vec<PhaseResult>,
    pub plan: Option<ExecutionPlan>,
    pub verification: Option<VerificationResult>,
    pub commit_info: Option<CommitInfo>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Harness execution phase
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPhase {
    #[default]
    Init,
    Analysis,
    Planning,
    Execution,
    Verification,
    Commit,
    Close,
    Done,
}

/// Phase execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: HarnessPhase,
    pub status: PhaseStatus,
    pub agent_results: Vec<AgentAnalysis>,
    pub output: Option<String>,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Phase status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PhaseStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
}

/// Agent analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAnalysis {
    pub agent_type: AgentRole,
    pub agent_name: String,
    pub analysis: String,
    pub findings: Vec<Finding>,
    pub recommendations: Vec<Recommendation>,
    pub confidence: f32,
    pub tokens_used: i32,
    pub duration_ms: i64,
}

/// Agent role in the harness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    IssueAnalyzer,
    CodeAnalyzer,
    Architect,
    Planner,
    Developer,
    Reviewer,
    Tester,
    SecurityAuditor,
    Verifier,
}

/// Analysis finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: FindingCategory,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub location: Option<CodeLocation>,
    pub evidence: Option<String>,
}

/// Finding category
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    Bug,
    Feature,
    Improvement,
    Refactoring,
    Security,
    Performance,
    Documentation,
    Testing,
    Configuration,
}

/// Code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    pub file_path: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub snippet: Option<String>,
}

/// Recommendation from analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub action: RecommendedAction,
    pub description: String,
    pub priority: i32,
    pub estimated_effort: Option<String>,
    pub dependencies: Vec<String>,
}

/// Recommended action type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    CreateFile,
    ModifyFile,
    DeleteFile,
    AddTest,
    RefactorCode,
    UpdateDependency,
    AddDocumentation,
    FixBug,
    ImplementFeature,
    ReviewSecurity,
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub id: String,
    pub summary: String,
    pub objectives: Vec<String>,
    pub steps: Vec<PlanStep>,
    pub estimated_duration: Option<String>,
    pub risk_assessment: RiskAssessment,
    pub rollback_strategy: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Plan step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub order: i32,
    pub name: String,
    pub description: String,
    pub agent_role: AgentRole,
    pub action: StepAction,
    pub inputs: HashMap<String, String>,
    pub expected_outputs: Vec<String>,
    pub dependencies: Vec<i32>,
    pub status: PhaseStatus,
    pub result: Option<StepResult>,
}

/// Step action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    AnalyzeCode,
    WriteCode,
    ModifyCode,
    DeleteCode,
    WriteTest,
    RunTest,
    Review,
    SecurityScan,
    DocumentChange,
    ValidateChange,
}

/// Step execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub success: bool,
    pub output: String,
    pub files_changed: Vec<FileChange>,
    pub tests_passed: Option<i32>,
    pub tests_failed: Option<i32>,
    pub duration_ms: i64,
}

/// File change record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
    pub additions: i32,
    pub deletions: i32,
    pub diff: Option<String>,
}

/// File change type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: Severity,
    pub risks: Vec<Risk>,
    pub mitigations: Vec<String>,
}

/// Risk item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub category: String,
    pub description: String,
    pub severity: Severity,
    pub likelihood: String,
    pub mitigation: Option<String>,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub success: bool,
    pub issue_requirements_met: bool,
    pub test_results: TestResults,
    pub code_quality: CodeQualityResult,
    pub security_scan: Option<SecurityScanResult>,
    pub verification_notes: Vec<String>,
    pub blockers: Vec<String>,
}

/// Test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub total: i32,
    pub passed: i32,
    pub failed: i32,
    pub skipped: i32,
    pub coverage_percent: Option<f32>,
    pub failed_tests: Vec<FailedTest>,
}

/// Failed test info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTest {
    pub name: String,
    pub file: String,
    pub error: String,
}

/// Code quality result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeQualityResult {
    pub score: f32,
    pub issues: Vec<QualityIssue>,
    pub metrics: HashMap<String, f32>,
}

/// Quality issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub file: String,
    pub line: Option<i32>,
}

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub vulnerabilities_found: i32,
    pub critical: i32,
    pub high: i32,
    pub medium: i32,
    pub low: i32,
    pub details: Vec<SecurityVulnerability>,
}

/// Security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVulnerability {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub file: Option<String>,
    pub remediation: Option<String>,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub branch: String,
    pub files_changed: i32,
    pub additions: i32,
    pub deletions: i32,
    pub pushed: bool,
    pub pr_number: Option<i32>,
    pub pr_url: Option<String>,
}

/// Start harness request
#[derive(Debug, Deserialize)]
pub struct StartHarnessRequest {
    pub issue_number: i32,
    pub options: Option<HarnessOptions>,
}

/// Harness options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HarnessOptions {
    pub auto_commit: Option<bool>,
    pub auto_close_issue: Option<bool>,
    pub create_pr: Option<bool>,
    pub run_tests: Option<bool>,
    pub security_scan: Option<bool>,
    pub max_iterations: Option<i32>,
    pub timeout_minutes: Option<i32>,
    pub dry_run: Option<bool>,
    pub branch_prefix: Option<String>,
    pub reviewers: Option<Vec<String>>,
}

/// Harness summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSummary {
    pub id: String,
    pub issue_number: i32,
    pub issue_title: String,
    pub status: HarnessStatus,
    pub current_phase: HarnessPhase,
    pub progress_percent: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Harness statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessStats {
    pub total_runs: i64,
    pub successful: i64,
    pub failed: i64,
    pub in_progress: i64,
    pub avg_duration_minutes: f64,
    pub issues_closed: i64,
    pub commits_made: i64,
    pub by_status: HashMap<String, i64>,
}

/// Agent prompts for harness
pub mod harness_prompts {
    pub const ISSUE_ANALYZER: &str = r#"You are an expert issue analyzer. Given a GitHub issue:
1. Understand the problem or feature request
2. Identify the type (bug, feature, improvement, etc.)
3. Extract acceptance criteria
4. Identify affected components/files
5. Assess complexity and effort

Output structured analysis with clear findings and recommendations."#;

    pub const CODE_ANALYZER: &str = r#"You are a code analysis expert. Analyze the codebase to:
1. Identify files related to the issue
2. Understand current implementation
3. Find potential root causes (for bugs)
4. Identify integration points
5. Note any technical debt

Provide detailed code-level analysis."#;

    pub const ARCHITECT: &str = r#"You are a software architect. Design the solution:
1. Propose architectural approach
2. Consider scalability and maintainability
3. Identify design patterns to use
4. Plan component interactions
5. Define interfaces and contracts

Create a high-level design document."#;

    pub const PLANNER: &str = r#"You are a development planner. Create an execution plan:
1. Break down work into steps
2. Define dependencies between steps
3. Assign appropriate agent roles
4. Estimate effort for each step
5. Identify risks and mitigations

Output a detailed step-by-step plan."#;

    pub const DEVELOPER: &str = r#"You are an expert developer. Implement the solution:
1. Write clean, maintainable code
2. Follow project conventions
3. Include appropriate error handling
4. Add necessary comments
5. Consider edge cases

Output the code changes needed."#;

    pub const REVIEWER: &str = r#"You are a code reviewer. Review the changes:
1. Check code quality and style
2. Verify logic correctness
3. Look for potential bugs
4. Assess test coverage
5. Ensure documentation

Provide detailed review feedback."#;

    pub const TESTER: &str = r#"You are a testing expert. Create and run tests:
1. Write unit tests for new code
2. Create integration tests if needed
3. Test edge cases
4. Verify error handling
5. Check for regressions

Output test code and results."#;

    pub const VERIFIER: &str = r#"You are a verification expert. Verify the solution:
1. Check if issue requirements are met
2. Verify all tests pass
3. Confirm code quality standards
4. Validate no regressions introduced
5. Ensure documentation is updated

Provide verification report."#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_status_default_is_pending() {
        assert_eq!(HarnessStatus::default(), HarnessStatus::Pending);
    }

    #[test]
    fn test_harness_phase_default_is_init() {
        assert_eq!(HarnessPhase::default(), HarnessPhase::Init);
    }

    #[test]
    fn test_harness_status_serde_snake_case() {
        let status = HarnessStatus::Executing;
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json, "executing");
    }

    #[test]
    fn test_harness_status_all_variants_snake_case() {
        let pairs = [
            (HarnessStatus::Pending, "pending"),
            (HarnessStatus::Analyzing, "analyzing"),
            (HarnessStatus::Planning, "planning"),
            (HarnessStatus::Executing, "executing"),
            (HarnessStatus::Verifying, "verifying"),
            (HarnessStatus::Committing, "committing"),
            (HarnessStatus::Completed, "completed"),
            (HarnessStatus::Failed, "failed"),
            (HarnessStatus::Cancelled, "cancelled"),
        ];
        for (status, expected) in &pairs {
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, *expected, "HarnessStatus::{status:?} should serialize as {expected}");
        }
    }

    #[test]
    fn test_harness_phase_all_variants_snake_case() {
        let pairs = [
            (HarnessPhase::Init, "init"),
            (HarnessPhase::Analysis, "analysis"),
            (HarnessPhase::Planning, "planning"),
            (HarnessPhase::Execution, "execution"),
            (HarnessPhase::Verification, "verification"),
            (HarnessPhase::Commit, "commit"),
            (HarnessPhase::Close, "close"),
            (HarnessPhase::Done, "done"),
        ];
        for (phase, expected) in &pairs {
            let json = serde_json::to_value(phase).unwrap();
            assert_eq!(json, *expected, "HarnessPhase::{phase:?} should serialize as {expected}");
        }
    }

    #[test]
    fn test_phase_status_serde() {
        let pairs = [
            (PhaseStatus::Pending, "pending"),
            (PhaseStatus::Running, "running"),
            (PhaseStatus::Success, "success"),
            (PhaseStatus::Failed, "failed"),
            (PhaseStatus::Skipped, "skipped"),
        ];
        for (status, expected) in &pairs {
            let json = serde_json::to_value(status).unwrap();
            assert_eq!(json, *expected);
        }
    }

    #[test]
    fn test_agent_role_serde_snake_case() {
        let role = AgentRole::SecurityAuditor;
        let json = serde_json::to_value(&role).unwrap();
        assert_eq!(json, "security_auditor");
    }

    #[test]
    fn test_finding_category_serde_snake_case() {
        let cat = FindingCategory::Refactoring;
        let json = serde_json::to_value(&cat).unwrap();
        assert_eq!(json, "refactoring");
    }

    #[test]
    fn test_recommended_action_create_file_snake_case() {
        let action = RecommendedAction::CreateFile;
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json, "create_file");
    }

    #[test]
    fn test_step_action_write_test_snake_case() {
        let action = StepAction::WriteTest;
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json, "write_test");
    }

    #[test]
    fn test_harness_options_all_none_by_default() {
        let opts = HarnessOptions::default();
        assert!(opts.auto_commit.is_none());
        assert!(opts.create_pr.is_none());
        assert!(opts.dry_run.is_none());
        assert!(opts.reviewers.is_none());
    }

    #[test]
    fn test_file_change_type_serde_lowercase() {
        let pairs = [
            (FileChangeType::Added, "added"),
            (FileChangeType::Modified, "modified"),
            (FileChangeType::Deleted, "deleted"),
            (FileChangeType::Renamed, "renamed"),
        ];
        for (ct, expected) in &pairs {
            let json = serde_json::to_value(ct).unwrap();
            assert_eq!(json, *expected);
        }
    }

    #[test]
    fn test_harness_prompts_are_non_empty() {
        use harness_prompts::*;
        assert!(!ISSUE_ANALYZER.is_empty());
        assert!(!CODE_ANALYZER.is_empty());
        assert!(!ARCHITECT.is_empty());
        assert!(!PLANNER.is_empty());
        assert!(!DEVELOPER.is_empty());
        assert!(!REVIEWER.is_empty());
        assert!(!TESTER.is_empty());
        assert!(!VERIFIER.is_empty());
    }

    #[test]
    fn test_start_harness_request_no_options() {
        let json = r#"{"issue_number": 42}"#;
        let req: StartHarnessRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.issue_number, 42);
        assert!(req.options.is_none());
    }
}
