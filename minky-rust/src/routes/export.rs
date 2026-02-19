use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::{ExportFormat, ExportJob, ExportRequest, ImportJob, ImportRequest},
    services::ExportService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(start_export))
        .route("/download", get(download_export))
        .route("/status/{id}", get(get_export_status))
        .route("/import", post(start_import))
}

#[derive(Debug, Serialize)]
pub struct ExportJobResponse {
    pub success: bool,
    pub data: ExportJob,
}

async fn start_export(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<ExportRequest>,
) -> AppResult<Json<ExportJobResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service.start_export(auth_user.id, payload).await?;

    Ok(Json(ExportJobResponse {
        success: true,
        data: job,
    }))
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub format: Option<ExportFormat>,
    pub document_ids: Option<String>,
    pub category_id: Option<i32>,
}

async fn download_export(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<DownloadQuery>,
) -> impl IntoResponse {
    let service = ExportService::new(state.db.clone());

    let request = ExportRequest {
        document_ids: query.document_ids.map(|s| {
            s.split(',')
                .filter_map(|id| uuid::Uuid::parse_str(id.trim()).ok())
                .collect()
        }),
        category_id: query.category_id,
        format: query.format.clone(),
        include_metadata: Some(true),
        include_attachments: Some(false),
        include_comments: Some(false),
        include_versions: Some(false),
    };

    let documents = match service.export_documents(auth_user.id, &request).await {
        Ok(docs) => docs,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Export failed: {}", e),
            )
                .into_response();
        }
    };

    let format = query.format.unwrap_or_default();
    let (content, content_type, extension) = match format {
        ExportFormat::Json => {
            let json = service.to_json(&documents).unwrap_or_default();
            (json, "application/json", "json")
        }
        ExportFormat::Csv => {
            let csv = service.to_csv(&documents).unwrap_or_default();
            (csv, "text/csv", "csv")
        }
        ExportFormat::Markdown => {
            let md = service.to_markdown(&documents).unwrap_or_default();
            (md, "text/markdown", "md")
        }
        _ => {
            let json = service.to_json(&documents).unwrap_or_default();
            (json, "application/json", "json")
        }
    };

    let filename = format!(
        "export_{}.{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S"),
        extension
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", filename)
            .parse()
            .unwrap(),
    );

    (headers, content).into_response()
}

#[derive(Debug, Serialize)]
pub struct ExportStatusResponse {
    pub success: bool,
    pub data: Option<ExportJob>,
}

async fn get_export_status(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<String>,
) -> AppResult<Json<ExportStatusResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service.get_export_status(&id).await?;

    Ok(Json(ExportStatusResponse {
        success: true,
        data: job,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ImportPayload {
    pub content: String,
    #[serde(flatten)]
    pub request: ImportRequest,
}

#[derive(Debug, Serialize)]
pub struct ImportJobResponse {
    pub success: bool,
    pub data: ImportJob,
}

async fn start_import(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<ImportPayload>,
) -> AppResult<Json<ImportJobResponse>> {
    let service = ExportService::new(state.db.clone());
    let job = service
        .import_from_json(auth_user.id, &payload.content, payload.request.category_id)
        .await?;

    Ok(Json(ImportJobResponse {
        success: true,
        data: job,
    }))
}
