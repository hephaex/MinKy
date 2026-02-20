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
