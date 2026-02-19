use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{error::AppResult, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .route("/refresh", post(refresh_token))
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
    State(_state): State<AppState>,
    Json(_payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // TODO: Implement login logic
    Ok(Json(AuthResponse {
        success: true,
        access_token: Some("placeholder".to_string()),
        refresh_token: Some("placeholder".to_string()),
        user: None,
    }))
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
    State(_state): State<AppState>,
    Json(_payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    // TODO: Implement registration logic
    Ok(Json(AuthResponse {
        success: true,
        access_token: None,
        refresh_token: None,
        user: None,
    }))
}

/// Refresh token request (used when token refresh is implemented)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

async fn refresh_token(
    State(_state): State<AppState>,
    Json(_payload): Json<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    // TODO: Implement token refresh logic
    Ok(Json(AuthResponse {
        success: true,
        access_token: Some("placeholder".to_string()),
        refresh_token: Some("placeholder".to_string()),
        user: None,
    }))
}
