use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};

use crate::models::{OcrJob, OcrRequest, OcrResult, OcrSettings};
use crate::services::OcrService;
use crate::AppState;

/// Start OCR job
async fn start_ocr(
    State(_state): State<AppState>,
    Path(attachment_id): Path<uuid::Uuid>,
    Json(request): Json<OcrRequest>,
) -> Result<Json<OcrJob>, (StatusCode, String)> {
    let service = OcrService::new();

    // TODO: Get user_id from auth
    let user_id = 1;

    service
        .start_ocr(attachment_id, user_id, request)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get OCR job status
async fn get_job_status(
    State(_state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<OcrJob>, (StatusCode, String)> {
    let service = OcrService::new();

    service
        .get_job_status(&job_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Job not found".to_string()))
}

/// Get OCR result
async fn get_ocr_result(
    State(_state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<OcrResult>, (StatusCode, String)> {
    // TODO: Implement result storage and retrieval
    Err((StatusCode::NOT_FOUND, "Result not found".to_string()))
}

/// Process image with OCR (synchronous)
async fn process_image(
    State(_state): State<AppState>,
    Json(request): Json<ProcessImageRequest>,
) -> Result<Json<OcrResult>, (StatusCode, String)> {
    let service = OcrService::new();
    let languages = request.languages.unwrap_or_else(|| vec!["eng".to_string()]);

    service
        .process_with_tesseract(&request.image_path, &languages)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Process PDF with OCR
async fn process_pdf(
    State(_state): State<AppState>,
    Json(request): Json<ProcessImageRequest>,
) -> Result<Json<OcrResult>, (StatusCode, String)> {
    let service = OcrService::new();
    let languages = request.languages.unwrap_or_else(|| vec!["eng".to_string()]);

    service
        .process_pdf(&request.image_path, &languages)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get OCR settings
async fn get_settings(
    State(_state): State<AppState>,
) -> Result<Json<OcrSettings>, (StatusCode, String)> {
    let service = OcrService::new();
    Ok(Json(service.get_settings().clone()))
}

/// Update OCR settings
async fn update_settings(
    State(_state): State<AppState>,
    Json(settings): Json<OcrSettings>,
) -> Result<Json<OcrSettings>, (StatusCode, String)> {
    let mut service = OcrService::new();
    service.update_settings(settings.clone());
    Ok(Json(settings))
}

/// Check supported format
async fn check_format(
    State(_state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<Json<FormatCheckResponse>, (StatusCode, String)> {
    let service = OcrService::new();
    let supported = service.is_supported_format(&filename);

    Ok(Json(FormatCheckResponse {
        filename,
        supported,
    }))
}

/// Estimate processing time
async fn estimate_time(
    State(_state): State<AppState>,
    Path(file_size): Path<i64>,
) -> Result<Json<TimeEstimateResponse>, (StatusCode, String)> {
    let service = OcrService::new();
    let estimated_ms = service.estimate_processing_time(file_size);

    Ok(Json(TimeEstimateResponse {
        file_size_bytes: file_size,
        estimated_ms,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct ProcessImageRequest {
    pub image_path: String,
    pub languages: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
pub struct FormatCheckResponse {
    pub filename: String,
    pub supported: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct TimeEstimateResponse {
    pub file_size_bytes: i64,
    pub estimated_ms: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/attachments/{attachment_id}/ocr", post(start_ocr))
        .route("/jobs/{job_id}", get(get_job_status))
        .route("/jobs/{job_id}/result", get(get_ocr_result))
        .route("/process/image", post(process_image))
        .route("/process/pdf", post(process_pdf))
        .route("/settings", get(get_settings).put(update_settings))
        .route("/check-format/{filename}", get(check_format))
        .route("/estimate/{file_size}", get(estimate_time))
}
