use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::middleware::AuthUser;
use crate::models::{
    CreateSkill, ExecuteSkillRequest, Skill, SkillExecutionHistory, SkillRegistry, SkillResult,
    SkillStats, SkillType, UpdateSkill,
};
use crate::services::SkillService;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i32>,
}

/// Get skill registry (all available skills)
async fn get_registry(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<SkillRegistry>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .get_registry()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get skill by ID
async fn get_skill(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(skill_id): Path<String>,
) -> Result<Json<Skill>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .get_skill(&skill_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Skill not found".to_string()))
}

/// Get skill by type
async fn get_skill_by_type(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(skill_type): Path<String>,
) -> Result<Json<Skill>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    let skill_type: SkillType = serde_json::from_str(&format!("\"{}\"", skill_type))
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill type".to_string()))?;

    service
        .get_skill_by_type(&skill_type)
        .cloned()
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Skill type not found".to_string()))
}

/// Execute skill
async fn execute_skill(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<ExecuteSkillRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .execute_skill(auth_user.id, request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Execute skill by type (shorthand)
async fn execute_skill_by_type(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(skill_type): Path<String>,
    Json(mut request): Json<ExecuteSkillRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    let parsed_type: SkillType = serde_json::from_str(&format!("\"{}\"", skill_type))
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid skill type".to_string()))?;

    request.skill_type = Some(parsed_type);

    service
        .execute_skill(auth_user.id, request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create custom skill
async fn create_skill(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(create): Json<CreateSkill>,
) -> Result<Json<Skill>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .create_skill(auth_user.id, create)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update skill
async fn update_skill(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(skill_id): Path<String>,
    Json(update): Json<UpdateSkill>,
) -> Result<Json<Skill>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .update_skill(&skill_id, update)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete skill
async fn delete_skill(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .delete_skill(&skill_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get skill stats
async fn get_stats(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<SkillStats>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    service
        .get_stats()
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get execution history
async fn get_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<SkillExecutionHistory>>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());
    let limit = query.limit.unwrap_or(50);

    service
        .get_history(auth_user.id, limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Quick execute endpoints for common skills
async fn review_code(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::CodeReviewer, request).await
}

async fn debug_code(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::Debugger, request).await
}

async fn refactor_code(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::Refactorer, request).await
}

async fn generate_tests(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::TestGenerator, request).await
}

async fn security_review(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::SecurityReviewer, request).await
}

async fn plan_feature(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(request): Json<QuickExecuteRequest>,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    execute_quick_skill(state, auth_user.id, SkillType::Planner, request).await
}

async fn execute_quick_skill(
    state: AppState,
    user_id: i32,
    skill_type: SkillType,
    request: QuickExecuteRequest,
) -> Result<Json<SkillResult>, (StatusCode, String)> {
    let service = SkillService::new(state.db.clone(), state.config.clone());

    let exec_request = ExecuteSkillRequest {
        skill_id: None,
        skill_type: Some(skill_type),
        input: request.input,
        context: request.context,
        options: None,
    };

    service
        .execute_skill(user_id, exec_request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct QuickExecuteRequest {
    pub input: String,
    pub context: Option<crate::models::SkillContext>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Registry and CRUD
        .route("/", get(get_registry).post(create_skill))
        .route("/{skill_id}", get(get_skill).put(update_skill).delete(delete_skill))
        .route("/type/{skill_type}", get(get_skill_by_type))
        // Execution
        .route("/execute", post(execute_skill))
        .route("/execute/{skill_type}", post(execute_skill_by_type))
        // Quick execute endpoints
        .route("/review", post(review_code))
        .route("/debug", post(debug_code))
        .route("/refactor", post(refactor_code))
        .route("/test", post(generate_tests))
        .route("/security", post(security_review))
        .route("/plan", post(plan_feature))
        // Stats and history
        .route("/stats", get(get_stats))
        .route("/history", get(get_history))
}
