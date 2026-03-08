//! Pipeline error types
//!
//! Defines errors that can occur during pipeline execution, with clear
//! categorization for different failure modes.

use crate::error::AppError;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during pipeline execution
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Stage execution failed
    #[error("Stage '{stage}' failed: {message}")]
    StageFailure {
        stage: &'static str,
        message: String,
        document_id: Option<Uuid>,
    },

    /// Input validation error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(Uuid),

    /// External service error (embedding API, Claude API, etc.)
    #[error("External service error: {0}")]
    ExternalService(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Pipeline was cancelled
    #[error("Pipeline cancelled at stage '{0}'")]
    Cancelled(&'static str),

    /// Pipeline timeout
    #[error("Pipeline timed out at stage '{stage}' after {elapsed_secs}s")]
    Timeout {
        stage: &'static str,
        elapsed_secs: u64,
    },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PipelineError {
    /// Create a stage failure error
    pub fn stage_failure(stage: &'static str, message: impl Into<String>) -> Self {
        Self::StageFailure {
            stage,
            message: message.into(),
            document_id: None,
        }
    }

    /// Create a stage failure error with document ID
    pub fn stage_failure_with_doc(
        stage: &'static str,
        message: impl Into<String>,
        document_id: Uuid,
    ) -> Self {
        Self::StageFailure {
            stage,
            message: message.into(),
            document_id: Some(document_id),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ExternalService(_) | Self::Timeout { .. } | Self::Database(_)
        )
    }

    /// Get the stage name if this is a stage-related error
    pub fn stage_name(&self) -> Option<&'static str> {
        match self {
            Self::StageFailure { stage, .. } => Some(stage),
            Self::Cancelled(stage) => Some(stage),
            Self::Timeout { stage, .. } => Some(stage),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for PipelineError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl From<AppError> for PipelineError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(msg) => Self::InvalidInput(msg),
            AppError::Configuration(msg) => Self::Configuration(msg),
            AppError::ExternalService(msg) => Self::ExternalService(msg),
            AppError::Database(e) => Self::Database(e.to_string()),
            AppError::Validation(msg) => Self::InvalidInput(msg),
            _ => Self::Internal(err.to_string()),
        }
    }
}

impl From<PipelineError> for AppError {
    fn from(err: PipelineError) -> Self {
        match err {
            PipelineError::InvalidInput(msg) => AppError::Validation(msg),
            PipelineError::DocumentNotFound(id) => {
                AppError::NotFound(format!("Document {} not found", id))
            }
            PipelineError::Configuration(msg) => AppError::Configuration(msg),
            PipelineError::ExternalService(msg) => AppError::ExternalService(msg),
            PipelineError::Database(msg) => {
                AppError::Internal(anyhow::anyhow!("Database error: {}", msg))
            }
            PipelineError::StageFailure { stage, message, .. } => {
                AppError::Internal(anyhow::anyhow!("Stage '{}' failed: {}", stage, message))
            }
            PipelineError::Cancelled(stage) => {
                AppError::Internal(anyhow::anyhow!("Pipeline cancelled at stage '{}'", stage))
            }
            PipelineError::Timeout { stage, elapsed_secs } => AppError::Internal(anyhow::anyhow!(
                "Pipeline timed out at stage '{}' after {}s",
                stage,
                elapsed_secs
            )),
            PipelineError::Internal(msg) => AppError::Internal(anyhow::anyhow!(msg)),
        }
    }
}

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_failure_creation() {
        let err = PipelineError::stage_failure("parsing", "Invalid markdown");
        assert!(matches!(err, PipelineError::StageFailure { stage: "parsing", .. }));
        assert_eq!(err.stage_name(), Some("parsing"));
    }

    #[test]
    fn test_stage_failure_with_doc() {
        let doc_id = Uuid::new_v4();
        let err = PipelineError::stage_failure_with_doc("embedding", "API error", doc_id);
        if let PipelineError::StageFailure { document_id, .. } = err {
            assert_eq!(document_id, Some(doc_id));
        } else {
            panic!("Expected StageFailure");
        }
    }

    #[test]
    fn test_retryable_errors() {
        assert!(PipelineError::ExternalService("timeout".into()).is_retryable());
        assert!(PipelineError::Database("connection lost".into()).is_retryable());
        assert!(PipelineError::Timeout {
            stage: "embedding",
            elapsed_secs: 30
        }
        .is_retryable());

        assert!(!PipelineError::InvalidInput("bad input".into()).is_retryable());
        assert!(!PipelineError::Configuration("missing key".into()).is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = PipelineError::stage_failure("parsing", "Invalid format");
        assert!(err.to_string().contains("parsing"));
        assert!(err.to_string().contains("Invalid format"));
    }
}
