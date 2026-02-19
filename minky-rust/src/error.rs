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

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            AppError::Configuration(msg) => {
                tracing::error!("Configuration error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error".to_string())
            }
            AppError::ExternalService(msg) => {
                tracing::error!("External service error: {}", msg);
                (StatusCode::BAD_GATEWAY, msg.clone())
            }
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string())
            }
        };

        let body = Json(json!({
            "success": false,
            "error": message
        }));

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
        let err = AppError::Conflict("username already taken".to_string());
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
}
