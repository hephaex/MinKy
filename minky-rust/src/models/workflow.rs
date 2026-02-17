use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

/// Workflow status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "workflow_status", rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    PendingReview,
    InReview,
    Approved,
    Rejected,
    Published,
    Archived,
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl ToString for WorkflowStatus {
    fn to_string(&self) -> String {
        match self {
            Self::Draft => "draft".to_string(),
            Self::PendingReview => "pending_review".to_string(),
            Self::InReview => "in_review".to_string(),
            Self::Approved => "approved".to_string(),
            Self::Rejected => "rejected".to_string(),
            Self::Published => "published".to_string(),
            Self::Archived => "archived".to_string(),
        }
    }
}

/// Workflow model for document state management
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Workflow {
    pub id: i32,
    pub document_id: Uuid,
    pub status: String,
    pub assigned_to: Option<i32>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: i32,
    pub notes: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow transition history
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct WorkflowHistory {
    pub id: i32,
    pub workflow_id: i32,
    pub from_status: String,
    pub to_status: String,
    pub changed_by: i32,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a workflow
#[derive(Debug, Deserialize)]
pub struct CreateWorkflow {
    pub document_id: Uuid,
    pub assigned_to: Option<i32>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
    pub notes: Option<String>,
}

/// DTO for updating workflow status
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowStatus {
    pub status: WorkflowStatus,
    pub comment: Option<String>,
}

/// DTO for updating workflow assignment
#[derive(Debug, Deserialize)]
pub struct UpdateWorkflowAssignment {
    pub assigned_to: Option<i32>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: Option<i32>,
    pub notes: Option<String>,
}

/// Workflow with document and user info
#[derive(Debug, Serialize)]
pub struct WorkflowWithDetails {
    pub id: i32,
    pub document_id: Uuid,
    pub document_title: String,
    pub status: String,
    pub assigned_to: Option<i32>,
    pub assignee_name: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub priority: i32,
    pub notes: Option<String>,
    pub created_by: i32,
    pub creator_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Valid workflow transitions
pub fn get_valid_transitions(current: &WorkflowStatus) -> Vec<WorkflowStatus> {
    match current {
        WorkflowStatus::Draft => vec![WorkflowStatus::PendingReview, WorkflowStatus::Archived],
        WorkflowStatus::PendingReview => vec![
            WorkflowStatus::InReview,
            WorkflowStatus::Draft,
            WorkflowStatus::Archived,
        ],
        WorkflowStatus::InReview => vec![
            WorkflowStatus::Approved,
            WorkflowStatus::Rejected,
            WorkflowStatus::PendingReview,
        ],
        WorkflowStatus::Approved => vec![WorkflowStatus::Published, WorkflowStatus::InReview],
        WorkflowStatus::Rejected => vec![WorkflowStatus::Draft, WorkflowStatus::Archived],
        WorkflowStatus::Published => vec![WorkflowStatus::Archived],
        WorkflowStatus::Archived => vec![WorkflowStatus::Draft],
    }
}

/// Check if a transition is valid
pub fn is_valid_transition(from: &WorkflowStatus, to: &WorkflowStatus) -> bool {
    get_valid_transitions(from).contains(to)
}
