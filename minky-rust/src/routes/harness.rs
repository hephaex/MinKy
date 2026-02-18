use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::models::{
    HarnessStats, HarnessSummary, IssueHarness, StartHarnessRequest,
};
use crate::services::HarnessService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i32>,
}

/// Start harness for an issue
async fn start_harness(
    State(state): State<AppState>,
    Json(request): Json<StartHarnessRequest>,
) -> Result<Json<IssueHarness>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    service
        .start_harness(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Start harness for an issue by number (shorthand)
async fn start_harness_for_issue(
    State(state): State<AppState>,
    Path(issue_number): Path<i32>,
) -> Result<Json<IssueHarness>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    let request = StartHarnessRequest {
        issue_number,
        options: None,
    };

    service
        .start_harness(request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get harness status
async fn get_harness(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<Json<IssueHarness>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    service
        .get_harness(&harness_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Harness not found".to_string()))
}

/// List all harnesses
async fn list_harnesses(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<HarnessSummary>>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());
    let limit = query.limit.unwrap_or(50);

    service
        .list_harnesses(limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get harness statistics
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<HarnessStats>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    service
        .get_stats()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Cancel a running harness
async fn cancel_harness(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    service
        .cancel_harness(&harness_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get harness phases for a specific harness
async fn get_harness_phases(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<Json<Vec<crate::models::PhaseResult>>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    let harness = service
        .get_harness(&harness_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Harness not found".to_string()))?;

    Ok(Json(harness.phases))
}

/// Get execution plan for a harness
async fn get_harness_plan(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<Json<crate::models::ExecutionPlan>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    let harness = service
        .get_harness(&harness_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Harness not found".to_string()))?;

    harness.plan
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Plan not yet created".to_string()))
}

/// Get verification result for a harness
async fn get_harness_verification(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<Json<crate::models::VerificationResult>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    let harness = service
        .get_harness(&harness_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Harness not found".to_string()))?;

    harness.verification
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Verification not yet completed".to_string()))
}

/// Get commit info for a harness
async fn get_harness_commit(
    State(state): State<AppState>,
    Path(harness_id): Path<String>,
) -> Result<Json<crate::models::CommitInfo>, (StatusCode, String)> {
    let service = HarnessService::new(state.db.clone(), state.config.clone());

    let harness = service
        .get_harness(&harness_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Harness not found".to_string()))?;

    harness.commit_info
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Commit not yet made".to_string()))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Main harness operations
        .route("/", get(list_harnesses).post(start_harness))
        .route("/issue/{issue_number}", post(start_harness_for_issue))
        .route("/{harness_id}", get(get_harness))
        .route("/{harness_id}/cancel", post(cancel_harness))
        // Harness details
        .route("/{harness_id}/phases", get(get_harness_phases))
        .route("/{harness_id}/plan", get(get_harness_plan))
        .route("/{harness_id}/verification", get(get_harness_verification))
        .route("/{harness_id}/commit", get(get_harness_commit))
        // Statistics
        .route("/stats", get(get_stats))
}
