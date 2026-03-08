use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        CommitRequest, CreateBranchRequest, GitBranch, GitCommit, GitDiff, GitLogEntry,
        GitRepository, GitStatus, PullRequest, PushRequest,
    },
    services::GitService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/info", get(get_repository_info))
        .route("/status", get(get_status))
        .route("/log", get(get_log))
        .route("/branches", get(list_branches))
        .route("/branches", post(create_branch))
        .route("/branches/{name}/checkout", post(checkout_branch))
        .route("/commit", post(commit))
        .route("/push", post(push))
        .route("/pull", post(pull))
        .route("/diff", get(get_diff))
        .route("/stage", post(stage_files))
}

fn get_git_service(state: &AppState) -> GitService {
    let repo_path = state
        .config
        .git_repo_path
        .as_deref()
        .unwrap_or(".");
    GitService::new(repo_path)
}

#[derive(Debug, Serialize)]
pub struct RepositoryResponse {
    pub success: bool,
    pub data: GitRepository,
}

async fn get_repository_info(State(state): State<AppState>, _auth_user: AuthUser) -> AppResult<Json<RepositoryResponse>> {
    let service = get_git_service(&state);
    let info = service.get_repository_info()?;

    Ok(Json(RepositoryResponse {
        success: true,
        data: info,
    }))
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub success: bool,
    pub data: GitStatus,
}

async fn get_status(State(state): State<AppState>, _auth_user: AuthUser) -> AppResult<Json<StatusResponse>> {
    let service = get_git_service(&state);
    let status = service.get_status()?;

    Ok(Json(StatusResponse {
        success: true,
        data: status,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct LogResponse {
    pub success: bool,
    pub data: Vec<GitLogEntry>,
}

async fn get_log(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<LogQuery>,
) -> AppResult<Json<LogResponse>> {
    let service = get_git_service(&state);
    let limit = query.limit.unwrap_or(20).min(100);
    let log = service.get_log(limit)?;

    Ok(Json(LogResponse {
        success: true,
        data: log,
    }))
}

#[derive(Debug, Serialize)]
pub struct BranchesResponse {
    pub success: bool,
    pub data: Vec<GitBranch>,
}

async fn list_branches(State(state): State<AppState>, _auth_user: AuthUser) -> AppResult<Json<BranchesResponse>> {
    let service = get_git_service(&state);
    let branches = service.list_branches()?;

    Ok(Json(BranchesResponse {
        success: true,
        data: branches,
    }))
}

#[derive(Debug, Serialize)]
pub struct BranchResponse {
    pub success: bool,
    pub data: GitBranch,
}

async fn create_branch(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<CreateBranchRequest>,
) -> AppResult<Json<BranchResponse>> {
    let service = get_git_service(&state);
    let branch = service.create_branch(payload)?;

    Ok(Json(BranchResponse {
        success: true,
        data: branch,
    }))
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

async fn checkout_branch(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(name): Path<String>,
) -> AppResult<Json<MessageResponse>> {
    let service = get_git_service(&state);
    service.checkout(&name)?;

    Ok(Json(MessageResponse {
        success: true,
        message: format!("Switched to branch '{}'", name),
    }))
}

#[derive(Debug, Serialize)]
pub struct CommitResponse {
    pub success: bool,
    pub data: GitCommit,
}

async fn commit(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<CommitRequest>,
) -> AppResult<Json<CommitResponse>> {
    let service = get_git_service(&state);
    let commit = service.commit(payload)?;

    Ok(Json(CommitResponse {
        success: true,
        data: commit,
    }))
}

async fn push(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<PushRequest>,
) -> AppResult<Json<MessageResponse>> {
    let service = get_git_service(&state);
    service.push(payload)?;

    Ok(Json(MessageResponse {
        success: true,
        message: "Pushed successfully".to_string(),
    }))
}

async fn pull(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<PullRequest>,
) -> AppResult<Json<MessageResponse>> {
    let service = get_git_service(&state);
    service.pull(payload)?;

    Ok(Json(MessageResponse {
        success: true,
        message: "Pulled successfully".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub staged: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DiffResponse {
    pub success: bool,
    pub data: GitDiff,
}

async fn get_diff(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Query(query): Query<DiffQuery>,
) -> AppResult<Json<DiffResponse>> {
    let service = get_git_service(&state);
    let staged = query.staged.unwrap_or(false);
    let diff = service.get_diff(staged)?;

    Ok(Json(DiffResponse {
        success: true,
        data: diff,
    }))
}

#[derive(Debug, Deserialize)]
pub struct StageRequest {
    pub files: Vec<String>,
}

async fn stage_files(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<StageRequest>,
) -> AppResult<Json<MessageResponse>> {
    let service = get_git_service(&state);
    service.stage_files(&payload.files)?;

    Ok(Json(MessageResponse {
        success: true,
        message: format!("Staged {} file(s)", payload.files.len()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{FileStatus, GitFileChange};

    // -------------------------------------------------------------------------
    // LogQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_log_query_deserialization() {
        let json = r#"{"limit": 50}"#;
        let query: LogQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_log_query_empty() {
        let json = r#"{}"#;
        let query: LogQuery = serde_json::from_str(json).unwrap();
        assert!(query.limit.is_none());
    }

    // -------------------------------------------------------------------------
    // DiffQuery tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_diff_query_deserialization() {
        let json = r#"{"staged": true}"#;
        let query: DiffQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.staged, Some(true));
    }

    #[test]
    fn test_diff_query_staged_false() {
        let json = r#"{"staged": false}"#;
        let query: DiffQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.staged, Some(false));
    }

    #[test]
    fn test_diff_query_empty() {
        let json = r#"{}"#;
        let query: DiffQuery = serde_json::from_str(json).unwrap();
        assert!(query.staged.is_none());
    }

    // -------------------------------------------------------------------------
    // StageRequest tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_stage_request_deserialization() {
        let json = r#"{"files": ["src/main.rs", "Cargo.toml"]}"#;
        let request: StageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.files.len(), 2);
        assert_eq!(request.files[0], "src/main.rs");
    }

    #[test]
    fn test_stage_request_empty_files() {
        let json = r#"{"files": []}"#;
        let request: StageRequest = serde_json::from_str(json).unwrap();
        assert!(request.files.is_empty());
    }

    // -------------------------------------------------------------------------
    // RepositoryResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_repository_response_serialization() {
        let now = chrono::Utc::now();
        let response = RepositoryResponse {
            success: true,
            data: GitRepository {
                path: "/home/user/project".to_string(),
                remote_url: Some("https://github.com/user/repo.git".to_string()),
                current_branch: "main".to_string(),
                is_clean: true,
                last_commit: Some(GitCommit {
                    hash: "abc123def456".to_string(),
                    short_hash: "abc123d".to_string(),
                    message: "Initial commit".to_string(),
                    author: "John Doe".to_string(),
                    author_email: "john@example.com".to_string(),
                    date: now,
                }),
                uncommitted_changes: 0,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"current_branch\":\"main\""));
    }

    // -------------------------------------------------------------------------
    // StatusResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_status_response_serialization() {
        let response = StatusResponse {
            success: true,
            data: GitStatus {
                branch: "feature/new".to_string(),
                ahead: 2,
                behind: 0,
                staged: vec![GitFileChange {
                    path: "src/lib.rs".to_string(),
                    status: FileStatus::Modified,
                }],
                unstaged: vec![],
                untracked: vec!["temp.txt".to_string()],
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"branch\":\"feature/new\""));
        assert!(json.contains("\"ahead\":2"));
    }

    // -------------------------------------------------------------------------
    // LogResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_log_response_serialization() {
        let now = chrono::Utc::now();
        let response = LogResponse {
            success: true,
            data: vec![GitLogEntry {
                hash: "abcdef123456".to_string(),
                short_hash: "abcdef1".to_string(),
                message: "feat: add new feature".to_string(),
                author: "Jane Doe".to_string(),
                date: now,
                files_changed: 3,
                insertions: 50,
                deletions: 10,
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"files_changed\":3"));
        assert!(json.contains("\"insertions\":50"));
    }

    // -------------------------------------------------------------------------
    // BranchesResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_branches_response_serialization() {
        let response = BranchesResponse {
            success: true,
            data: vec![GitBranch {
                name: "main".to_string(),
                is_current: true,
                is_remote: false,
                last_commit: None,
                tracking: Some("origin/main".to_string()),
            }],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"main\""));
        assert!(json.contains("\"is_current\":true"));
    }

    // -------------------------------------------------------------------------
    // BranchResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_branch_response_serialization() {
        let response = BranchResponse {
            success: true,
            data: GitBranch {
                name: "feature/test".to_string(),
                is_current: false,
                is_remote: false,
                last_commit: None,
                tracking: None,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"name\":\"feature/test\""));
    }

    // -------------------------------------------------------------------------
    // MessageResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_message_response_serialization() {
        let response = MessageResponse {
            success: true,
            message: "Operation successful".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"message\":\"Operation successful\""));
    }

    // -------------------------------------------------------------------------
    // CommitResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_commit_response_serialization() {
        let now = chrono::Utc::now();
        let response = CommitResponse {
            success: true,
            data: GitCommit {
                hash: "fedcba654321".to_string(),
                short_hash: "fedcba6".to_string(),
                message: "fix: resolve bug".to_string(),
                author: "Developer".to_string(),
                author_email: "dev@example.com".to_string(),
                date: now,
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"message\":\"fix: resolve bug\""));
    }

    // -------------------------------------------------------------------------
    // DiffResponse tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_diff_response_serialization() {
        let response = DiffResponse {
            success: true,
            data: GitDiff {
                files: vec![],
                stats: crate::models::GitDiffStats {
                    files_changed: 2,
                    insertions: 30,
                    deletions: 5,
                },
            },
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"files_changed\":2"));
        assert!(json.contains("\"insertions\":30"));
    }

    // -------------------------------------------------------------------------
    // Router tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_routes_creation() {
        let _router: Router<AppState> = routes();
        // Should be creatable without panicking
    }
}
