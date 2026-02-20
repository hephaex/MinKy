use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

use crate::{services::AuthService, AppState};

/// Authenticated user extracted from JWT token
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i32,
    pub email: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// Extractor rejection type
#[derive(Debug)]
pub struct AuthError(pub String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": self.0 })),
        )
            .into_response()
    }
}

/// Extract cookie value by name from Cookie header
fn extract_cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .map(|s| s.trim())
                .find(|s| s.starts_with(&format!("{}=", name)))
                .and_then(|s| s.split('=').nth(1))
                .map(|s| s.to_string())
        })
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        // Try Authorization header first, then fall back to HttpOnly cookie
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()))
            .or_else(|| extract_cookie_value(&parts.headers, "access_token"))
            .ok_or_else(|| AuthError("Missing authentication token".to_string()))?;

        let auth_service = AuthService::new(state.db.clone(), state.config.clone());

        let claims = auth_service
            .validate_token(&token)
            .map_err(|_| AuthError("Invalid or expired token".to_string()))?;

        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

/// Optional authenticated user (doesn't fail if no token)
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        // Try Authorization header first, then fall back to HttpOnly cookie
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()))
            .or_else(|| extract_cookie_value(&parts.headers, "access_token"));

        let auth_user = token.and_then(|t| {
            let auth_service = AuthService::new(state.db.clone(), state.config.clone());
            auth_service.validate_token(&t).ok()
        })
        .map(|claims| AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        });

        Ok(OptionalAuthUser(auth_user))
    }
}

/// Admin-only user extractor
#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user: AuthUser = AuthUser::from_request_parts(parts, state).await?;

        if !auth_user.is_admin() {
            return Err(AuthError("Admin access required".to_string()));
        }

        Ok(AdminUser(auth_user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(role: &str) -> AuthUser {
        AuthUser {
            id: 1,
            email: "test@example.com".to_string(),
            role: role.to_string(),
        }
    }

    #[test]
    fn test_is_admin_returns_true_for_admin_role() {
        let user = make_user("admin");
        assert!(user.is_admin());
    }

    #[test]
    fn test_is_admin_returns_false_for_user_role() {
        let user = make_user("user");
        assert!(!user.is_admin());
    }

    #[test]
    fn test_is_admin_returns_false_for_empty_role() {
        let user = make_user("");
        assert!(!user.is_admin());
    }

    #[test]
    fn test_is_admin_is_case_sensitive() {
        // "Admin" (capital A) is not the same as "admin"
        let user = make_user("Admin");
        assert!(!user.is_admin(), "Role check should be case-sensitive");
    }
}

/// Request metadata for audit logging
#[derive(Debug, Clone, Serialize)]
pub struct RequestMeta {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl<S> FromRequestParts<S> for RequestMeta
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ip_address = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .or_else(|| {
                parts
                    .headers
                    .get("x-real-ip")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string())
            });

        let user_agent = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        Ok(RequestMeta {
            ip_address,
            user_agent,
        })
    }
}
