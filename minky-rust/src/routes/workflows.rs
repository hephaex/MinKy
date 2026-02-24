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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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
    _auth_user: AuthUser,
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

async fn list_overdue(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> AppResult<Json<WorkflowListResponse>> {
    let service = WorkflowService::new(state.db.clone());
    let workflows = service.get_overdue().await?;

    Ok(Json(WorkflowListResponse {
        success: true,
        data: workflows,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    // CreateWorkflowRequest validation tests
    #[test]
    fn test_create_workflow_request_all_none() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: None,
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_workflow_request_with_all_fields() {
        let req = CreateWorkflowRequest {
            assigned_to: Some(5),
            due_date: Some(Utc::now()),
            priority: Some(3),
            notes: Some("Review this document".to_string()),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_workflow_request_priority_min() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: Some(0),
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_workflow_request_priority_max() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: Some(10),
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_workflow_request_priority_too_high() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: Some(11),
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_workflow_request_priority_negative() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: Some(-1),
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_create_workflow_request_notes_max_length() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: None,
            notes: Some("x".repeat(1000)),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_workflow_request_notes_too_long() {
        let req = CreateWorkflowRequest {
            assigned_to: None,
            due_date: None,
            priority: None,
            notes: Some("x".repeat(1001)),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // UpdateStatusRequest validation tests
    #[test]
    fn test_update_status_request_valid() {
        let req = UpdateStatusRequest {
            status: WorkflowStatus::PendingReview,
            comment: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_status_request_with_comment() {
        let req = UpdateStatusRequest {
            status: WorkflowStatus::Approved,
            comment: Some("Looks good!".to_string()),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_status_request_comment_max_length() {
        let req = UpdateStatusRequest {
            status: WorkflowStatus::Rejected,
            comment: Some("x".repeat(500)),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_status_request_comment_too_long() {
        let req = UpdateStatusRequest {
            status: WorkflowStatus::InReview,
            comment: Some("x".repeat(501)),
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // UpdateAssignmentRequest validation tests
    #[test]
    fn test_update_assignment_request_all_none() {
        let req = UpdateAssignmentRequest {
            assigned_to: None,
            due_date: None,
            priority: None,
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_assignment_request_with_values() {
        let req = UpdateAssignmentRequest {
            assigned_to: Some(10),
            due_date: Some(Utc::now()),
            priority: Some(5),
            notes: Some("Urgent review needed".to_string()),
        };
        let result = req.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_assignment_request_priority_too_high() {
        let req = UpdateAssignmentRequest {
            assigned_to: None,
            due_date: None,
            priority: Some(15),
            notes: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    // WorkflowResponse tests
    #[test]
    fn test_workflow_response_creation() {
        let workflow = Workflow {
            id: 1,
            document_id: Uuid::new_v4(),
            status: "draft".to_string(),
            assigned_to: Some(5),
            due_date: None,
            priority: 3,
            notes: None,
            created_by: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = WorkflowResponse {
            success: true,
            data: workflow,
        };
        assert!(response.success);
        assert_eq!(response.data.id, 1);
    }

    // WorkflowListResponse tests
    #[test]
    fn test_workflow_list_response_empty() {
        let response = WorkflowListResponse {
            success: true,
            data: vec![],
        };
        assert!(response.data.is_empty());
    }

    #[test]
    fn test_workflow_list_response_multiple() {
        let workflows = vec![
            Workflow {
                id: 1,
                document_id: Uuid::new_v4(),
                status: "draft".to_string(),
                assigned_to: None,
                due_date: None,
                priority: 0,
                notes: None,
                created_by: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Workflow {
                id: 2,
                document_id: Uuid::new_v4(),
                status: "in_review".to_string(),
                assigned_to: Some(2),
                due_date: None,
                priority: 5,
                notes: None,
                created_by: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        let response = WorkflowListResponse {
            success: true,
            data: workflows,
        };
        assert_eq!(response.data.len(), 2);
    }

    // WorkflowHistoryResponse tests
    #[test]
    fn test_workflow_history_response_empty() {
        let response = WorkflowHistoryResponse {
            success: true,
            data: vec![],
        };
        assert!(response.data.is_empty());
    }

    #[test]
    fn test_workflow_history_response_with_entries() {
        let history = vec![WorkflowHistory {
            id: 1,
            workflow_id: 1,
            from_status: "draft".to_string(),
            to_status: "pending_review".to_string(),
            changed_by: 5,
            comment: Some("Ready for review".to_string()),
            created_at: Utc::now(),
        }];
        let response = WorkflowHistoryResponse {
            success: true,
            data: history,
        };
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].from_status, "draft");
    }

    // WorkflowStatus parsing tests
    #[test]
    fn test_status_string_to_enum_draft() {
        let status_str = "draft";
        let status = match status_str {
            "draft" => WorkflowStatus::Draft,
            _ => panic!("Unknown status"),
        };
        assert!(matches!(status, WorkflowStatus::Draft));
    }

    #[test]
    fn test_status_string_to_enum_all() {
        let cases = vec![
            ("draft", WorkflowStatus::Draft),
            ("pending_review", WorkflowStatus::PendingReview),
            ("in_review", WorkflowStatus::InReview),
            ("approved", WorkflowStatus::Approved),
            ("rejected", WorkflowStatus::Rejected),
            ("published", WorkflowStatus::Published),
            ("archived", WorkflowStatus::Archived),
        ];
        for (s, expected) in cases {
            let parsed = match s {
                "draft" => WorkflowStatus::Draft,
                "pending_review" => WorkflowStatus::PendingReview,
                "in_review" => WorkflowStatus::InReview,
                "approved" => WorkflowStatus::Approved,
                "rejected" => WorkflowStatus::Rejected,
                "published" => WorkflowStatus::Published,
                "archived" => WorkflowStatus::Archived,
                _ => panic!("Unknown status"),
            };
            assert!(std::mem::discriminant(&parsed) == std::mem::discriminant(&expected));
        }
    }
}
