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
