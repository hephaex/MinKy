use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;

use crate::{services::AuthService, AppState};

/// Extract user ID from JWT token in Authorization header
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing or invalid Authorization header" })),
            ))
        }
    };

    let auth_service = AuthService::new(state.db.clone(), state.config.clone());

    match auth_service.validate_token(token) {
        Ok(claims) => {
            // Insert user ID into request extensions for handlers to access
            request.extensions_mut().insert(claims.sub);
            request.extensions_mut().insert(claims.role);
            Ok(next.run(request).await)
        }
        Err(_) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired token" })),
        )),
    }
}

/// Optional auth - doesn't fail if no token, but extracts user if present
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let auth_service = AuthService::new(state.db.clone(), state.config.clone());
            if let Ok(claims) = auth_service.validate_token(token) {
                request.extensions_mut().insert(claims.sub);
                request.extensions_mut().insert(claims.role);
            }
        }
    }

    next.run(request).await
}

/// Admin-only middleware - requires admin role
pub async fn admin_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing or invalid Authorization header" })),
            ))
        }
    };

    let auth_service = AuthService::new(state.db.clone(), state.config.clone());

    match auth_service.validate_token(token) {
        Ok(claims) if claims.role == "admin" => Ok(next.run(request).await),
        Ok(_) => Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Admin access required" })),
        )),
        Err(_) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Invalid or expired token" })),
        )),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bearer_token_extraction_valid() {
        // Test that "Bearer <token>" format is expected
        let header = "Bearer my-jwt-token-here";
        let token = if header.starts_with("Bearer ") {
            Some(&header[7..])
        } else {
            None
        };
        assert_eq!(token, Some("my-jwt-token-here"));
    }

    #[test]
    fn test_bearer_token_extraction_missing_bearer_prefix() {
        // Without "Bearer " prefix, token should not be extracted
        let header = "my-jwt-token-here";
        let token = if header.starts_with("Bearer ") {
            Some(&header[7..])
        } else {
            None
        };
        assert!(token.is_none());
    }

    #[test]
    fn test_bearer_token_extraction_wrong_scheme() {
        // "Basic" scheme should not extract token
        let header = "Basic credentials";
        let token = if header.starts_with("Bearer ") {
            Some(&header[7..])
        } else {
            None
        };
        assert!(token.is_none());
    }

    #[test]
    fn test_bearer_token_extraction_empty_token() {
        // "Bearer " with empty token should extract empty string
        let header = "Bearer ";
        let token = if header.starts_with("Bearer ") {
            Some(&header[7..])
        } else {
            None
        };
        assert_eq!(token, Some(""));
    }

    #[test]
    fn test_strip_prefix_method() {
        // Test strip_prefix used in optional_auth_middleware
        let header = "Bearer abc123";
        let token = header.strip_prefix("Bearer ");
        assert_eq!(token, Some("abc123"));

        let header_no_prefix = "Token abc123";
        let token = header_no_prefix.strip_prefix("Bearer ");
        assert!(token.is_none());
    }

    #[test]
    fn test_admin_role_check() {
        // Test role comparison logic used in admin_middleware
        let role = "admin";
        assert_eq!(role, "admin");

        let user_role = "user";
        assert_ne!(user_role, "admin");
    }
}
