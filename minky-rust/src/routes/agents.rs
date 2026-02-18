use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;

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
) -> Result<Json<Vec<Agent>>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .list_agents(user_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get agent by ID
async fn get_agent(
    State(state): State<AppState>,
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
    Json(create): Json<CreateAgent>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .create_agent(user_id, create)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update agent
async fn update_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<i32>,
    Json(update): Json<UpdateAgent>,
) -> Result<Json<Agent>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .update_agent(user_id, agent_id, update)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete agent
async fn delete_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .delete_agent(user_id, agent_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Execute agent
async fn execute_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<i32>,
    Json(request): Json<ExecuteAgentRequest>,
) -> Result<Json<AgentResult>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .execute_agent(user_id, agent_id, request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// List tasks
async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<AgentTask>>, (StatusCode, String)> {
    let service = AgentService::new(state.db.clone(), state.config.clone());

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .get_tasks(user_id, query.agent_id)
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
