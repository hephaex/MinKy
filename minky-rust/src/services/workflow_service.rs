use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        is_valid_transition, CreateWorkflow, UpdateWorkflowAssignment, UpdateWorkflowStatus,
        Workflow, WorkflowHistory, WorkflowStatus,
    },
};

/// Workflow service for document state management
pub struct WorkflowService {
    db: PgPool,
}

impl WorkflowService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get workflow for a document
    pub async fn get_by_document(&self, document_id: Uuid) -> AppResult<Workflow> {
        let workflow = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM workflows WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Workflow not found".to_string()))?;

        Ok(workflow)
    }

    /// Get workflow by ID
    pub async fn get(&self, id: i32) -> AppResult<Workflow> {
        let workflow = sqlx::query_as::<_, Workflow>("SELECT * FROM workflows WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Workflow not found".to_string()))?;

        Ok(workflow)
    }

    /// Create a new workflow for a document
    pub async fn create(&self, user_id: i32, data: CreateWorkflow) -> AppResult<Workflow> {
        // Check if workflow already exists
        let existing = sqlx::query_as::<_, Workflow>(
            "SELECT * FROM workflows WHERE document_id = $1",
        )
        .bind(data.document_id)
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(
                "Workflow already exists for this document".to_string(),
            ));
        }

        let workflow = sqlx::query_as::<_, Workflow>(
            r#"
            INSERT INTO workflows (document_id, status, assigned_to, due_date, priority, notes, created_by)
            VALUES ($1, 'draft', $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.document_id)
        .bind(data.assigned_to)
        .bind(data.due_date)
        .bind(data.priority.unwrap_or(0))
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(workflow)
    }

    /// Update workflow status with validation
    pub async fn update_status(
        &self,
        id: i32,
        user_id: i32,
        data: UpdateWorkflowStatus,
    ) -> AppResult<Workflow> {
        let existing = self.get(id).await?;

        // Parse current status
        let current_status = match existing.status.as_str() {
            "draft" => WorkflowStatus::Draft,
            "pending_review" => WorkflowStatus::PendingReview,
            "in_review" => WorkflowStatus::InReview,
            "approved" => WorkflowStatus::Approved,
            "rejected" => WorkflowStatus::Rejected,
            "published" => WorkflowStatus::Published,
            "archived" => WorkflowStatus::Archived,
            _ => return Err(AppError::Internal(anyhow::anyhow!("Invalid status"))),
        };

        // Validate transition
        if !is_valid_transition(&current_status, &data.status) {
            return Err(AppError::Validation(format!(
                "Cannot transition from '{}' to '{}'",
                current_status,
                data.status
            )));
        }

        // Record history
        sqlx::query(
            r#"
            INSERT INTO workflow_history (workflow_id, from_status, to_status, changed_by, comment)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(id)
        .bind(&existing.status)
        .bind(data.status.to_string())
        .bind(user_id)
        .bind(&data.comment)
        .execute(&self.db)
        .await?;

        // Update workflow
        let workflow = sqlx::query_as::<_, Workflow>(
            r#"
            UPDATE workflows
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(data.status.to_string())
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(workflow)
    }

    /// Update workflow assignment
    pub async fn update_assignment(
        &self,
        id: i32,
        data: UpdateWorkflowAssignment,
    ) -> AppResult<Workflow> {
        let existing = self.get(id).await?;

        let assigned_to = data.assigned_to.or(existing.assigned_to);
        let due_date = data.due_date.or(existing.due_date);
        let priority = data.priority.unwrap_or(existing.priority);
        let notes = data.notes.or(existing.notes);

        let workflow = sqlx::query_as::<_, Workflow>(
            r#"
            UPDATE workflows
            SET assigned_to = $1, due_date = $2, priority = $3, notes = $4, updated_at = NOW()
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(assigned_to)
        .bind(due_date)
        .bind(priority)
        .bind(&notes)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(workflow)
    }

    /// Get workflow history
    pub async fn get_history(&self, workflow_id: i32) -> Result<Vec<WorkflowHistory>> {
        let history = sqlx::query_as::<_, WorkflowHistory>(
            r#"
            SELECT * FROM workflow_history
            WHERE workflow_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(workflow_id)
        .fetch_all(&self.db)
        .await?;

        Ok(history)
    }

    /// List workflows assigned to a user
    pub async fn list_assigned(&self, user_id: i32) -> Result<Vec<Workflow>> {
        let workflows = sqlx::query_as::<_, Workflow>(
            r#"
            SELECT * FROM workflows
            WHERE assigned_to = $1
            ORDER BY priority DESC, due_date ASC NULLS LAST
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(workflows)
    }

    /// List workflows by status
    pub async fn list_by_status(&self, status: WorkflowStatus) -> Result<Vec<Workflow>> {
        let workflows = sqlx::query_as::<_, Workflow>(
            r#"
            SELECT * FROM workflows
            WHERE status = $1
            ORDER BY priority DESC, created_at ASC
            "#,
        )
        .bind(status.to_string())
        .fetch_all(&self.db)
        .await?;

        Ok(workflows)
    }

    /// Get overdue workflows
    pub async fn get_overdue(&self) -> Result<Vec<Workflow>> {
        let workflows = sqlx::query_as::<_, Workflow>(
            r#"
            SELECT * FROM workflows
            WHERE due_date < NOW() AND status NOT IN ('published', 'archived')
            ORDER BY due_date ASC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(workflows)
    }

    /// Delete workflow
    pub async fn delete(&self, id: i32) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM workflows WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Workflow not found".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WorkflowStatus;

    #[tokio::test]
    async fn test_workflow_service_new_creates_instance() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test_db").unwrap();
        let service = WorkflowService::new(pool);
        drop(service);
    }

    #[test]
    fn test_workflow_status_to_string_draft() {
        let status = WorkflowStatus::Draft;
        assert_eq!(status.to_string(), "draft");
    }

    #[test]
    fn test_workflow_status_to_string_pending_review() {
        let status = WorkflowStatus::PendingReview;
        assert_eq!(status.to_string(), "pending_review");
    }

    #[test]
    fn test_workflow_status_to_string_in_review() {
        let status = WorkflowStatus::InReview;
        assert_eq!(status.to_string(), "in_review");
    }

    #[test]
    fn test_workflow_status_to_string_approved() {
        let status = WorkflowStatus::Approved;
        assert_eq!(status.to_string(), "approved");
    }

    #[test]
    fn test_workflow_status_to_string_rejected() {
        let status = WorkflowStatus::Rejected;
        assert_eq!(status.to_string(), "rejected");
    }

    #[test]
    fn test_workflow_status_to_string_published() {
        let status = WorkflowStatus::Published;
        assert_eq!(status.to_string(), "published");
    }

    #[test]
    fn test_workflow_status_to_string_archived() {
        let status = WorkflowStatus::Archived;
        assert_eq!(status.to_string(), "archived");
    }

    #[test]
    fn test_is_valid_transition_draft_to_pending_review() {
        assert!(is_valid_transition(
            &WorkflowStatus::Draft,
            &WorkflowStatus::PendingReview
        ));
    }

    #[test]
    fn test_is_valid_transition_pending_review_to_in_review() {
        assert!(is_valid_transition(
            &WorkflowStatus::PendingReview,
            &WorkflowStatus::InReview
        ));
    }

    #[test]
    fn test_is_valid_transition_in_review_to_approved() {
        assert!(is_valid_transition(
            &WorkflowStatus::InReview,
            &WorkflowStatus::Approved
        ));
    }

    #[test]
    fn test_is_valid_transition_in_review_to_rejected() {
        assert!(is_valid_transition(
            &WorkflowStatus::InReview,
            &WorkflowStatus::Rejected
        ));
    }

    #[test]
    fn test_is_valid_transition_approved_to_published() {
        assert!(is_valid_transition(
            &WorkflowStatus::Approved,
            &WorkflowStatus::Published
        ));
    }

    #[test]
    fn test_is_valid_transition_published_to_archived() {
        assert!(is_valid_transition(
            &WorkflowStatus::Published,
            &WorkflowStatus::Archived
        ));
    }

    #[test]
    fn test_is_invalid_transition_draft_to_published() {
        // Cannot skip directly to published
        assert!(!is_valid_transition(
            &WorkflowStatus::Draft,
            &WorkflowStatus::Published
        ));
    }

    #[test]
    fn test_is_valid_transition_archived_to_draft() {
        // Can restore from archived to draft
        assert!(is_valid_transition(
            &WorkflowStatus::Archived,
            &WorkflowStatus::Draft
        ));
    }

    #[test]
    fn test_is_invalid_transition_archived_to_published() {
        // Cannot go directly from archived to published
        assert!(!is_valid_transition(
            &WorkflowStatus::Archived,
            &WorkflowStatus::Published
        ));
    }

    #[test]
    fn test_is_valid_transition_rejected_to_draft() {
        // Can revise after rejection
        assert!(is_valid_transition(
            &WorkflowStatus::Rejected,
            &WorkflowStatus::Draft
        ));
    }
}
