use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Application error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication required")]
    Unauthorized,

    #[error("Access denied")]
    Forbidden,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    /// 409 conflict used for duplicate detection.  When a content_hash is available,
    /// the response emits the spec-compliant `duplicate_detected` JSON; otherwise it
    /// uses a generic conflict error message.
    #[error("{0}")]
    Conflict(String, Option<String>),

    /// 409 for upload-path duplicates — includes enough info for the frontend
    /// to show a direct link to the existing document.
    #[error("Duplicate document: content already uploaded")]
    DuplicateDocument {
        content_hash: String,
        existing_id: uuid::Uuid,
        existing_title: String,
        existing_created_at: chrono::DateTime<chrono::Utc>,
    },

    /// 409 for losing the (user_id, source_path) active-unique race to a
    /// concurrent upload. `existing_id` is the winning document (looked up
    /// right after the race is detected) so callers can treat this the same
    /// way as `DuplicateDocument` — top-level `existing_id`, same field name
    /// the sync client already reads (2026-07-17 R3).
    #[error("Active document already exists for source_path '{source_path}'")]
    SourcePathConflict {
        source_path: String,
        existing_id: Option<uuid::Uuid>,
    },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Invalidation failure ({0}): {1}")]
    Invalidation(String, String),

    #[error("Redirect to {0}")]
    Redirect(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body): (StatusCode, Json<serde_json::Value>) = match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": "Authentication required"}))),
            Self::Forbidden => (StatusCode::FORBIDDEN, Json(json!({"success": false, "error": "Access denied"}))),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, Json(json!({"success": false, "error": msg}))),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, Json(json!({"success": false, "error": msg}))),
            Self::Conflict(msg, maybe_content_hash) => {
                if let Some(content_hash) = maybe_content_hash {
                    return (StatusCode::CONFLICT, Json(json!({
                        "error": "duplicate_detected",
                        "content_hash": content_hash,
                    }))).into_response();
                }
                (StatusCode::CONFLICT, Json(json!({ "success": false, "error": msg })))
            }
            Self::DuplicateDocument { content_hash, existing_id, existing_title, existing_created_at } => {
                return (StatusCode::CONFLICT, Json(json!({
                    "error": "duplicate_detected",
                    "content_hash": content_hash,
                    "existing_id": existing_id,
                    "existing_title": existing_title,
                    "existing_created_at": existing_created_at,
                }))).into_response();
            }
            Self::SourcePathConflict { source_path, existing_id } => {
                return (StatusCode::CONFLICT, Json(json!({
                    "error": "source_path_conflict",
                    "source_path": source_path,
                    "existing_id": existing_id,
                }))).into_response();
            }
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, Json(json!({"success": false, "error": "Rate limit exceeded"}))),
            Self::Invalidation(source, detail) => (StatusCode::CONFLICT, Json(json!({"success": false, "error": format!("Invalidation failure ({}): {}", source, detail)}))),
            Self::Redirect(url) => (StatusCode::SEE_OTHER, Json(json!({ "success": false, "redirect": url }))),
            Self::Configuration(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": "Configuration error" })))
            }
            Self::ExternalService(msg) => {
                tracing::error!("External service error: {}", msg);
                (StatusCode::BAD_GATEWAY, Json(json!({ "success": false, "error": "External service error" })))
            }
            Self::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": "Database error" })))
            }
            Self::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": "Internal error" })))
            }
        };

        (status, body).into_response()
    }
}

/// Type alias for Results with AppError
pub type AppResult<T> = std::result::Result<T, AppError>;

/// Convenient Result type alias
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unauthorized_display_message() {
        let err = AppError::Unauthorized;
        assert_eq!(err.to_string(), "Authentication required");
    }

    #[test]
    fn test_forbidden_display_message() {
        let err = AppError::Forbidden;
        assert_eq!(err.to_string(), "Access denied");
    }

    #[test]
    fn test_not_found_includes_resource_name() {
        let err = AppError::NotFound("document".to_string());
        assert!(err.to_string().contains("document"), "NotFound message should include resource name");
    }

    #[test]
    fn test_validation_includes_message() {
        let err = AppError::Validation("email is required".to_string());
        assert!(err.to_string().contains("email is required"));
    }

    #[test]
    fn test_conflict_includes_message() {
        let err = AppError::Conflict("username already taken".to_string(), None);
        assert!(err.to_string().contains("username already taken"));
    }

    #[test]
    fn test_rate_limited_display_message() {
        let err = AppError::RateLimited;
        assert_eq!(err.to_string(), "Rate limit exceeded");
    }

    #[test]
    fn test_configuration_includes_detail() {
        let err = AppError::Configuration("missing API key".to_string());
        assert!(err.to_string().contains("missing API key"));
    }

    #[test]
    fn test_external_service_includes_detail() {
        let err = AppError::ExternalService("timeout".to_string());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_invalidations_display_detail() {
        let err = AppError::Invalidation("etag".to_string(), "stale hash".to_string());
        assert!(err.to_string().contains("etag"));
        assert!(err.to_string().contains("stale hash"));
    }

    #[test]
    fn test_redirect_display_detail() {
        let err = AppError::Redirect("https://example.com/callback".to_string());
        assert!(err.to_string().contains("https://example.com/callback"));
    }

    #[test]
    fn test_duplicate_document_display() {
        let id = uuid::Uuid::new_v4();
        let err = AppError::DuplicateDocument {
            content_hash: "abc".to_string(),
            existing_id: id,
            existing_title: "Test Doc".to_string(),
            existing_created_at: chrono::Utc::now(),
        };
        assert!(err.to_string().contains("Duplicate document"));
    }
}
