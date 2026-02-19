use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{
        ApplyTemplateRequest, CreateTemplate, Template, TemplatePreview, TemplateQuery,
        UpdateTemplate,
    },
    services::TemplateService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/", post(create_template))
        .route("/{id}", get(get_template))
        .route("/{id}", put(update_template))
        .route("/{id}", delete(delete_template))
        .route("/{id}/preview", post(preview_template))
        .route("/{id}/apply", post(apply_template))
}

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub success: bool,
    pub data: Vec<Template>,
}

async fn list_templates(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<TemplateQuery>,
) -> AppResult<Json<TemplatesResponse>> {
    let service = TemplateService::new(state.db.clone());
    let templates = service.list_templates(auth_user.id, query).await?;

    Ok(Json(TemplatesResponse {
        success: true,
        data: templates,
    }))
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub success: bool,
    pub data: Template,
}

async fn get_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service
        .get_template(auth_user.id, id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Template not found".to_string()))?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

async fn create_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateTemplate>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service.create_template(auth_user.id, payload).await?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

async fn update_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTemplate>,
) -> AppResult<Json<TemplateResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service.update_template(auth_user.id, id, payload).await?;

    Ok(Json(TemplateResponse {
        success: true,
        data: template,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = TemplateService::new(state.db.clone());
    service.delete_template(auth_user.id, id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Template deleted".to_string(),
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct PreviewRequest {
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub success: bool,
    pub data: TemplatePreview,
}

async fn preview_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(payload): Json<PreviewRequest>,
) -> AppResult<Json<PreviewResponse>> {
    let service = TemplateService::new(state.db.clone());
    let template = service
        .get_template(auth_user.id, id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("Template not found".to_string()))?;

    let preview = service.preview_template(&template, payload.variables.as_ref())?;

    Ok(Json(PreviewResponse {
        success: true,
        data: preview,
    }))
}

#[derive(Debug, Serialize)]
pub struct ApplyResponse {
    pub success: bool,
    pub document_id: uuid::Uuid,
}

async fn apply_template(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
    Json(mut payload): Json<ApplyTemplateRequest>,
) -> AppResult<Json<ApplyResponse>> {
    payload.template_id = id;

    let service = TemplateService::new(state.db.clone());
    let document_id = service.apply_template(auth_user.id, payload).await?;

    Ok(Json(ApplyResponse {
        success: true,
        document_id,
    }))
}
