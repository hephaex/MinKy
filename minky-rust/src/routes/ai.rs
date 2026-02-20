use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{EmbeddingResponse, SuggestionRequest, SuggestionResponse, SuggestionType},
    services::AIService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/suggest", post(generate_suggestion))
        .route("/suggest/title", post(suggest_title))
        .route("/suggest/summary", post(suggest_summary))
        .route("/suggest/tags", post(suggest_tags))
        .route("/improve", post(improve_text))
        .route("/embedding", post(generate_embedding))
}

#[derive(Debug, Deserialize, Validate)]
pub struct SuggestionRequestBody {
    #[validate(length(min = 1, max = 50000))]
    pub content: String,
    pub suggestion_type: SuggestionType,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestionResponseBody {
    pub success: bool,
    pub data: SuggestionResponse,
}

async fn generate_suggestion(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<SuggestionRequestBody>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: payload.suggestion_type,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct ContentRequest {
    #[validate(length(min = 1, max = 50000))]
    pub content: String,
    pub context: Option<String>,
}

async fn suggest_title(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Title,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn suggest_summary(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Summary,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn suggest_tags(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Tags,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

async fn improve_text(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<ContentRequest>,
) -> AppResult<Json<SuggestionResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service
        .generate_suggestion(SuggestionRequest {
            content: payload.content,
            suggestion_type: SuggestionType::Improve,
            context: payload.context,
        })
        .await?;

    Ok(Json(SuggestionResponseBody {
        success: true,
        data: response,
    }))
}

#[derive(Debug, Deserialize, Validate)]
pub struct EmbeddingRequest {
    #[validate(length(min = 1, max = 10000))]
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingResponseBody {
    pub success: bool,
    pub data: EmbeddingResponse,
}

async fn generate_embedding(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<EmbeddingRequest>,
) -> AppResult<Json<EmbeddingResponseBody>> {
    let service = AIService::new(state.config.clone());

    let response = service.generate_embedding(&payload.text).await?;

    Ok(Json(EmbeddingResponseBody {
        success: true,
        data: response,
    }))
}
