use axum::{
    extract::{ConnectInfo, FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;
use std::{net::SocketAddr, sync::OnceLock};

use crate::{
    services::{ApiKeyIdentity, AuthService, SecurityService},
    AppState,
};

/// Returns true when TRUSTED_PROXY=true env var is set.
/// Same semantics as rate_limit.rs — cached after first read.
fn is_trusted_proxy() -> bool {
    static TRUSTED: OnceLock<bool> = OnceLock::new();
    *TRUSTED.get_or_init(|| {
        std::env::var("TRUSTED_PROXY")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

/// Returns true when BYPASS_AUTH=true env var is set.
/// Intended for internal-only deployments where auth is not required.
/// All requests are treated as the service admin user (id=1).
fn is_bypass_auth() -> bool {
    static BYPASS: OnceLock<bool> = OnceLock::new();
    *BYPASS.get_or_init(|| {
        std::env::var("BYPASS_AUTH")
            .map(|v| v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    })
}

fn bypass_admin_user() -> AuthUser {
    AuthUser {
        id: 1,
        email: "service@internal".to_string(),
        role: "admin".to_string(),
    }
}

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
                .and_then(|s| s.split_once('=').map(|x| x.1))
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
        if is_bypass_auth() {
            return Ok(bypass_admin_user());
        }

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

        if let (Some(jti), Some(redis)) = (&claims.jti, &state.redis_client) {
            if AuthService::is_token_revoked(jti, redis).await {
                return Err(AuthError("Token has been revoked".to_string()));
            }
        }

        let id = claims
            .sub
            .parse::<i32>()
            .map_err(|_| AuthError("Invalid user ID in token".to_string()))?;

        Ok(AuthUser {
            id,
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
        if is_bypass_auth() {
            return Ok(OptionalAuthUser(Some(bypass_admin_user())));
        }

        let state = AppState::from_ref(state);

        // Try Authorization header first, then fall back to HttpOnly cookie
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer ").map(|s| s.to_string()))
            .or_else(|| extract_cookie_value(&parts.headers, "access_token"));

        let auth_user = token
            .and_then(|t| {
                let auth_service = AuthService::new(state.db.clone(), state.config.clone());
                auth_service.validate_token(&t).ok()
            })
            .and_then(|claims| {
                let id = claims.sub.parse::<i32>().ok()?;
                Some(AuthUser {
                    id,
                    email: claims.email,
                    role: claims.role,
                })
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

/// Authenticated principal: either an interactive end user (JWT/cookie or
/// BYPASS_AUTH admin) or a service identity backed by an API key.
///
/// Extraction order mirrors the rest of the auth stack:
///   1. `BYPASS_AUTH` short-circuit → `User(bypass_admin_user())`
///   2. Bearer JWT / `access_token` cookie (existing `AuthUser` logic) → `User`
///   3. `X-API-Key` header validated against `api_keys` → `Service`
///   4. otherwise → `401 Unauthorized`
#[derive(Debug, Clone)]
pub enum AuthPrincipal {
    User(AuthUser),
    Service(ApiKeyIdentity),
}

impl AuthPrincipal {
    /// Effective acting user id for the principal.
    ///
    /// Service principals resolve to `api_keys.user_id`, which is `NOT NULL`.
    pub fn effective_user_id(&self) -> i32 {
        match self {
            AuthPrincipal::User(user) => user.id,
            AuthPrincipal::Service(identity) => identity.user_id,
        }
    }
}

impl<S> FromRequestParts<S> for AuthPrincipal
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // (1) BYPASS_AUTH and (2) Bearer JWT / cookie are both handled by the
        // existing AuthUser extractor, which short-circuits on BYPASS_AUTH before
        // attempting JWT/cookie validation. This preserves the required ordering:
        // BYPASS → Bearer JWT → X-API-Key → 401.
        if let Ok(user) = AuthUser::from_request_parts(parts, state).await {
            return Ok(AuthPrincipal::User(user));
        }

        // (3) X-API-Key header → validate against api_keys.
        let app_state = AppState::from_ref(state);
        if let Some(raw_key) = parts.headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
            let security = SecurityService::new(app_state.db.clone());
            match security.validate_api_key(raw_key).await {
                Ok(Some(identity)) => return Ok(AuthPrincipal::Service(identity)),
                Ok(None) => {}
                Err(e) => {
                    // Fail closed (401), but keep infrastructure failures
                    // distinguishable from bad credentials in the logs.
                    tracing::warn!(error = %e, "api key validation failed due to backend error");
                }
            }
        }

        // (4) No valid credential of any kind.
        Err(AuthError("Authentication required".to_string()))
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

    #[test]
    fn test_effective_user_id_for_user_principal() {
        let principal = AuthPrincipal::User(make_user("user"));
        assert_eq!(principal.effective_user_id(), 1);
    }

    #[test]
    fn test_effective_user_id_for_service_principal() {
        let principal = AuthPrincipal::Service(ApiKeyIdentity {
            key_id: 42,
            user_id: 7,
            name: "ci-bot".to_string(),
        });
        // Service principals resolve to api_keys.user_id, not the key id.
        assert_eq!(principal.effective_user_id(), 7);
    }

    #[test]
    fn test_auth_error_responds_401() {
        // The AuthPrincipal rejection path returns AuthError, which must map to
        // HTTP 401 for anonymous requests to protected endpoints.
        let resp = AuthError("Authentication required".to_string()).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
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
        // Real peer IP from TCP connection (always available, not attacker-controllable)
        let peer_ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip().to_string());

        let ip_address = if is_trusted_proxy() {
            // Behind a trusted reverse proxy: honour forwarded headers, fall back to peer
            parts
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
                })
                .or(peer_ip)
        } else {
            // Direct connection: use only the real TCP peer address
            peer_ip
        };

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
