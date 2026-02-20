use axum::{
    extract::State,
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::{AppError, AppResult},
    middleware::AuthUser,
    models::{User, UserResponse},
    services::AuthService,
    AppState,
};

/// Cookie configuration constants
const ACCESS_TOKEN_COOKIE: &str = "access_token";
const REFRESH_TOKEN_COOKIE: &str = "refresh_token";
const ACCESS_TOKEN_MAX_AGE: i64 = 900; // 15 minutes
const REFRESH_TOKEN_MAX_AGE: i64 = 604800; // 7 days

/// Build HttpOnly cookie header value
fn build_cookie(name: &str, value: &str, max_age: i64, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!(
        "{}={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}{}",
        name, value, max_age, secure_flag
    )
}

/// Build cookie deletion header (for logout)
fn build_delete_cookie(name: &str) -> String {
    format!(
        "{}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        name
    )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub user: Option<UserInfo>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i32,
    pub email: String,
    pub username: String,
    pub role: String,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<(HeaderMap, Json<AuthResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let auth_service = AuthService::new(state.db.clone(), state.config.clone());

    let user = auth_service
        .find_user_by_email(&payload.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Check account is active
    if !user.is_active {
        return Err(AppError::Forbidden);
    }

    // Check account is not locked
    if let Some(locked_until) = user.locked_until {
        if locked_until > Utc::now() {
            return Err(AppError::Forbidden);
        }
    }

    // Verify password
    let password_valid = auth_service
        .verify_password(&payload.password, &user.password_hash)?;

    if !password_valid {
        // Record failed attempt (ignore errors here)
        let _ = auth_service.record_failed_login(user.id).await;
        return Err(AppError::Unauthorized);
    }

    // Reset failed attempts on success (ignore errors here)
    let _ = auth_service.reset_login_attempts(user.id).await;

    let access_token = auth_service
        .generate_access_token(&user)?;

    let refresh_token = auth_service
        .generate_refresh_token(&user)?;

    // Set HttpOnly cookies
    let is_secure = state.config.environment != "development";
    let mut headers = HeaderMap::new();

    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            ACCESS_TOKEN_COOKIE,
            &access_token,
            ACCESS_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            REFRESH_TOKEN_COOKIE,
            &refresh_token,
            REFRESH_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );

    Ok((
        headers,
        Json(AuthResponse {
            success: true,
            // Tokens no longer sent in body for security (HttpOnly cookies instead)
            access_token: None,
            refresh_token: None,
            user: Some(UserInfo {
                id: user.id,
                email: user.email,
                username: user.username,
                role: format!("{:?}", user.role).to_lowercase(),
            }),
        }),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    pub username: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<(StatusCode, HeaderMap, Json<AuthResponse>)> {
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let auth_service = AuthService::new(state.db.clone(), state.config.clone());

    // Check if email already exists
    let existing = auth_service
        .find_user_by_email(&payload.email)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("Email already registered".to_string()));
    }

    let password_hash = auth_service
        .hash_password(&payload.password)?;

    let user: User = sqlx::query_as(
        r#"INSERT INTO users (email, username, password_hash)
           VALUES ($1, $2, $3)
           RETURNING id, email, username, password_hash, role, is_active,
                     failed_login_attempts, locked_until, created_at, updated_at"#,
    )
    .bind(&payload.email)
    .bind(&payload.username)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await?;

    let access_token = auth_service
        .generate_access_token(&user)?;

    let refresh_token = auth_service
        .generate_refresh_token(&user)?;

    // Set HttpOnly cookies
    let is_secure = state.config.environment != "development";
    let mut headers = HeaderMap::new();

    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            ACCESS_TOKEN_COOKIE,
            &access_token,
            ACCESS_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            REFRESH_TOKEN_COOKIE,
            &refresh_token,
            REFRESH_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(AuthResponse {
            success: true,
            // Tokens no longer sent in body for security (HttpOnly cookies instead)
            access_token: None,
            refresh_token: None,
            user: Some(UserInfo {
                id: user.id,
                email: user.email,
                username: user.username,
                role: format!("{:?}", user.role).to_lowercase(),
            }),
        }),
    ))
}

/// Refresh token request (for backward compatibility, can also use cookie)
#[derive(Debug, Deserialize, Default)]
pub struct RefreshRequest {
    pub refresh_token: Option<String>,
}

/// Extract refresh token from cookie header
fn extract_cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
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

async fn refresh_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<(HeaderMap, Json<AuthResponse>)> {
    let auth_service = AuthService::new(state.db.clone(), state.config.clone());

    // Try to get refresh token from body first, then from cookie
    let token = payload
        .refresh_token
        .or_else(|| extract_cookie_value(&headers, REFRESH_TOKEN_COOKIE))
        .ok_or(AppError::Unauthorized)?;

    let claims = auth_service
        .validate_token(&token)
        .map_err(|_| AppError::Unauthorized)?;

    // Load user to verify still active
    let user: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE id = $1 AND is_active = true")
            .bind(claims.sub)
            .fetch_optional(&state.db)
            .await?;

    let user = user.ok_or(AppError::Unauthorized)?;

    let new_access_token = auth_service
        .generate_access_token(&user)?;

    let new_refresh_token = auth_service
        .generate_refresh_token(&user)?;

    // Set HttpOnly cookies
    let is_secure = state.config.environment != "development";
    let mut response_headers = HeaderMap::new();

    response_headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            ACCESS_TOKEN_COOKIE,
            &new_access_token,
            ACCESS_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );
    response_headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_cookie(
            REFRESH_TOKEN_COOKIE,
            &new_refresh_token,
            REFRESH_TOKEN_MAX_AGE,
            is_secure,
        ))
        .unwrap(),
    );

    Ok((
        response_headers,
        Json(AuthResponse {
            success: true,
            // Tokens no longer sent in body for security (HttpOnly cookies instead)
            access_token: None,
            refresh_token: None,
            user: Some(UserInfo {
                id: user.id,
                email: user.email,
                username: user.username,
                role: format!("{:?}", user.role).to_lowercase(),
            }),
        }),
    ))
}

/// Logout - clears HttpOnly cookies
async fn logout() -> (HeaderMap, Json<serde_json::Value>) {
    let mut headers = HeaderMap::new();

    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_delete_cookie(ACCESS_TOKEN_COOKIE)).unwrap(),
    );
    headers.append(
        SET_COOKIE,
        HeaderValue::from_str(&build_delete_cookie(REFRESH_TOKEN_COOKIE)).unwrap(),
    );

    (
        headers,
        Json(serde_json::json!({
            "success": true,
            "message": "Logged out successfully"
        })),
    )
}

/// Get current authenticated user info
async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let user: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE id = $1 AND is_active = true")
            .bind(auth_user.id)
            .fetch_optional(&state.db)
            .await?;

    let user = user.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let user_response: UserResponse = user.into();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": user_response
    })))
}
