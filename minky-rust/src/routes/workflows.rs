use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{CreateWorkflow, UpdateWorkflowAssignment, UpdateWorkflowStatus, Workflow, WorkflowHistory, WorkflowStatus},
    services::WorkflowService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/document/{document_id}", get(get_workflow_by_document).post(create_workflow))
        .route("/{id}", get(get_workflow).put(update_assignment))
        .route("/{id}/status", put(update_status))
        .route("/{id}/history", get(get_history))
        .route("/assigned", get(list_assigned))
        .route("/status/{status}", get(list_by_status))
        .route("/overdue", get(list_overdue))
}

#[derive(Debug, Serialize)]
pub struct WorkflowResponse {
    pub success: bool,
    pub data: Workflow,
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<WorkflowResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let workflow = service.get(id).await?;

    Ok(Json(WorkflowResponse {
        success: true,
        data: workflow,
    }))
}

async fn get_workflow_by_document(
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<WorkflowResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let workflow = service.get_by_document(document_id).await?;

    Ok(Json(WorkflowResponse {
        success: true,
        data: workflow,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWorkflowRequest {
    pub assigned_to: Option<i32>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    #[validate(range(min = 0, max = 10))]
    pub priority: Option<i32>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

async fn create_workflow(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> AppResult<(StatusCode, Json<WorkflowResponse>)> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = WorkflowService::new(state.db.clone());
    let workflow = service
        .create(
            auth_user.id,
            CreateWorkflow {
                document_id,
                assigned_to: payload.assigned_to,
                due_date: payload.due_date,
                priority: payload.priority,
                notes: payload.notes,
            },
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(WorkflowResponse {
            success: true,
            data: workflow,
        }),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStatusRequest {
    pub status: WorkflowStatus,
    #[validate(length(max = 500))]
    pub comment: Option<String>,
}

async fn update_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStatusRequest>,
) -> AppResult<Json<WorkflowResponse>> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = WorkflowService::new(state.db.clone());
    let workflow = service
        .update_status(
            id,
            auth_user.id,
            UpdateWorkflowStatus {
                status: payload.status,
                comment: payload.comment,
            },
        )
        .await?;

    Ok(Json(WorkflowResponse {
        success: true,
        data: workflow,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAssignmentRequest {
    pub assigned_to: Option<i32>,
    pub due_date: Option<chrono::DateTime<chrono::Utc>>,
    #[validate(range(min = 0, max = 10))]
    pub priority: Option<i32>,
    #[validate(length(max = 1000))]
    pub notes: Option<String>,
}

async fn update_assignment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAssignmentRequest>,
) -> AppResult<Json<WorkflowResponse>> {
    payload
        .validate()
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;

    let service = WorkflowService::new(state.db.clone());
    let workflow = service
        .update_assignment(
            id,
            UpdateWorkflowAssignment {
                assigned_to: payload.assigned_to,
                due_date: payload.due_date,
                priority: payload.priority,
                notes: payload.notes,
            },
        )
        .await?;

    Ok(Json(WorkflowResponse {
        success: true,
        data: workflow,
    }))
}

#[derive(Debug, Serialize)]
pub struct WorkflowHistoryResponse {
    pub success: bool,
    pub data: Vec<WorkflowHistory>,
}

async fn get_history(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<WorkflowHistoryResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let history = service.get_history(id).await?;

    Ok(Json(WorkflowHistoryResponse {
        success: true,
        data: history,
    }))
}

#[derive(Debug, Serialize)]
pub struct WorkflowListResponse {
    pub success: bool,
    pub data: Vec<Workflow>,
}

async fn list_assigned(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<WorkflowListResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let workflows = service.list_assigned(auth_user.id).await?;

    Ok(Json(WorkflowListResponse {
        success: true,
        data: workflows,
    }))
}

async fn list_by_status(
    State(state): State<AppState>,
    Path(status): Path<String>,
) -> AppResult<Json<WorkflowListResponse>> {
    let status = match status.as_str() {
        "draft" => WorkflowStatus::Draft,
        "pending_review" => WorkflowStatus::PendingReview,
        "in_review" => WorkflowStatus::InReview,
        "approved" => WorkflowStatus::Approved,
        "rejected" => WorkflowStatus::Rejected,
        "published" => WorkflowStatus::Published,
        "archived" => WorkflowStatus::Archived,
        _ => return Err(crate::error::AppError::Validation("Invalid status".to_string())),
    };

    let service = WorkflowService::new(state.db.clone());
    let workflows = service.list_by_status(status).await?;

    Ok(Json(WorkflowListResponse {
        success: true,
        data: workflows,
    }))
}

async fn list_overdue(State(state): State<AppState>) -> AppResult<Json<WorkflowListResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let workflows = service.get_overdue().await?;

    Ok(Json(WorkflowListResponse {
        success: true,
        data: workflows,
    }))
}
