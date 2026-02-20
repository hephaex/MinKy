//! Common types and utilities shared across route handlers.

use axum::{http::StatusCode, Json};
use serde::Serialize;

use crate::error::AppError;

/// Standard API response wrapper for successful responses.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response with the given data.
    pub fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data,
        })
    }
}

/// Convert an AppError into an HTTP error response tuple.
///
/// Returns a tuple of (StatusCode, JSON body) suitable for returning from handlers.
pub fn into_error_response(err: AppError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &err {
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Validation(_) => StatusCode::BAD_REQUEST,
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::Forbidden => StatusCode::FORBIDDEN,
        AppError::Conflict(_) => StatusCode::CONFLICT,
        AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        AppError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::ExternalService(_) => StatusCode::BAD_GATEWAY,
    };

    let body = Json(serde_json::json!({
        "success": false,
        "error": err.to_string(),
    }));

    (status, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok_sets_success_true() {
        let response = ApiResponse::ok("test data");
        assert!(response.success);
        assert_eq!(response.data, "test data");
    }

    #[test]
    fn test_api_response_ok_with_struct() {
        #[derive(Debug, Serialize, PartialEq)]
        struct TestData {
            id: i32,
            name: String,
        }

        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };
        let response = ApiResponse::ok(data);
        assert!(response.success);
        assert_eq!(response.data.id, 1);
        assert_eq!(response.data.name, "Test");
    }

    #[test]
    fn test_into_error_response_not_found() {
        let err = AppError::NotFound("User not found".to_string());
        let (status, body) = into_error_response(err);
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(!body.0["success"].as_bool().unwrap());
    }

    #[test]
    fn test_into_error_response_validation() {
        let err = AppError::Validation("Invalid input".to_string());
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_into_error_response_unauthorized() {
        let err = AppError::Unauthorized;
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_into_error_response_forbidden() {
        let err = AppError::Forbidden;
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_into_error_response_rate_limited() {
        let err = AppError::RateLimited;
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_into_error_response_configuration() {
        let err = AppError::Configuration("Missing config".to_string());
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_into_error_response_external_service() {
        let err = AppError::ExternalService("API error".to_string());
        let (status, _) = into_error_response(err);
        assert_eq!(status, StatusCode::BAD_GATEWAY);
    }
}
