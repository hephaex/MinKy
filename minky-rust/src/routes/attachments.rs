use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::path::PathBuf;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    middleware::AuthUser,
    models::{validate_upload, AttachmentWithUploader, CreateAttachment},
    services::AttachmentService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/document/{document_id}", get(list_attachments).post(upload_attachment))
        .route("/{id}", get(download_attachment).delete(delete_attachment))
        .route("/{id}/info", get(get_attachment_info))
}

#[derive(Debug, Serialize)]
pub struct AttachmentListResponse {
    pub success: bool,
    pub data: Vec<AttachmentWithUploader>,
    pub total_size: i64,
}

async fn list_attachments(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<AttachmentListResponse>> {
    let upload_dir = PathBuf::from("./uploads");
    let service = AttachmentService::new(state.db.clone(), &upload_dir);

    let attachments = service.list_for_document(document_id).await?;
    let total_size = service.get_document_storage(document_id).await?;

    Ok(Json(AttachmentListResponse {
        success: true,
        data: attachments,
        total_size,
    }))
}

#[derive(Debug, Serialize)]
pub struct AttachmentResponse {
    pub success: bool,
    pub data: AttachmentData,
}

#[derive(Debug, Serialize)]
pub struct AttachmentData {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub document_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn upload_attachment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(document_id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<AttachmentResponse>> {
    let upload_dir = PathBuf::from("./uploads");

    // Ensure upload directory exists
    tokio::fs::create_dir_all(&upload_dir).await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to create upload directory: {}", e))
    })?;

    let service = AttachmentService::new(state.db.clone(), &upload_dir);

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::Validation(format!("Failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let original_filename = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let content_type = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let data = field.bytes().await.map_err(|e| {
                AppError::Validation(format!("Failed to read file data: {}", e))
            })?;

            let file_size = data.len() as i64;

            // Validate upload
            validate_upload(&content_type, file_size)
                .map_err(AppError::Validation)?;

            // Generate storage filename
            let storage_filename = AttachmentService::generate_storage_filename(&original_filename);
            let file_path = upload_dir.join(&storage_filename);

            // Write file to disk
            tokio::fs::write(&file_path, &data).await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to write file: {}", e))
            })?;

            // Create database record
            let attachment = service
                .create(
                    auth_user.id,
                    CreateAttachment {
                        filename: storage_filename,
                        original_filename: original_filename.clone(),
                        mime_type: content_type,
                        file_size,
                        document_id,
                    },
                )
                .await?;

            return Ok(Json(AttachmentResponse {
                success: true,
                data: AttachmentData {
                    id: attachment.id,
                    filename: attachment.filename,
                    original_filename: attachment.original_filename,
                    mime_type: attachment.mime_type,
                    file_size: attachment.file_size,
                    document_id: attachment.document_id,
                    created_at: attachment.created_at,
                },
            }));
        }
    }

    Err(AppError::Validation("No file provided".to_string()))
}

async fn download_attachment(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let upload_dir = PathBuf::from("./uploads");
    let service = AttachmentService::new(state.db.clone(), &upload_dir);

    let attachment = service.get(id).await?;
    let file_path = service.get_file_path(&attachment.filename);

    if !file_path.exists() {
        return Err(AppError::NotFound("File not found on disk".to_string()));
    }

    let data = tokio::fs::read(&file_path).await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to read file: {}", e))
    })?;

    let headers = [
        (header::CONTENT_TYPE, attachment.mime_type),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", attachment.original_filename),
        ),
    ];

    Ok((StatusCode::OK, headers, data))
}

async fn get_attachment_info(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AttachmentResponse>> {
    let upload_dir = PathBuf::from("./uploads");
    let service = AttachmentService::new(state.db.clone(), &upload_dir);

    let attachment = service.get(id).await?;

    Ok(Json(AttachmentResponse {
        success: true,
        data: AttachmentData {
            id: attachment.id,
            filename: attachment.filename,
            original_filename: attachment.original_filename,
            mime_type: attachment.mime_type,
            file_size: attachment.file_size,
            document_id: attachment.document_id,
            created_at: attachment.created_at,
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_attachment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DeleteResponse>> {
    let upload_dir = PathBuf::from("./uploads");

    let service = AttachmentService::new(state.db.clone(), &upload_dir);
    service.delete(id, auth_user.id, auth_user.is_admin()).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Attachment deleted successfully".to_string(),
    }))
}
