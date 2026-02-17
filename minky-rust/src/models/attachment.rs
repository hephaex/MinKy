use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Attachment model representing uploaded files
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Attachment {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub document_id: Uuid,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating an attachment
#[derive(Debug, Deserialize)]
pub struct CreateAttachment {
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub document_id: Uuid,
}

/// Attachment with uploader info
#[derive(Debug, Serialize, FromRow)]
pub struct AttachmentWithUploader {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub document_id: Uuid,
    pub user_id: i32,
    pub uploader_name: String,
    pub created_at: DateTime<Utc>,
}

/// Allowed file types for upload
pub const ALLOWED_MIME_TYPES: &[&str] = &[
    // Images
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    // Documents
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    // Text
    "text/plain",
    "text/markdown",
    "text/csv",
    // Archives
    "application/zip",
    "application/x-tar",
    "application/gzip",
];

/// Maximum file size (50MB)
pub const MAX_FILE_SIZE: i64 = 50 * 1024 * 1024;

/// Validate file upload
pub fn validate_upload(mime_type: &str, file_size: i64) -> Result<(), String> {
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        return Err(format!("File type '{}' is not allowed", mime_type));
    }

    if file_size > MAX_FILE_SIZE {
        return Err(format!(
            "File size {} exceeds maximum allowed size of {} bytes",
            file_size, MAX_FILE_SIZE
        ));
    }

    if file_size == 0 {
        return Err("File is empty".to_string());
    }

    Ok(())
}

/// Generate safe filename
pub fn sanitize_filename(filename: &str) -> String {
    let safe_chars: String = filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Prevent directory traversal
    safe_chars.replace("..", "_").trim_matches('_').to_string()
}

/// Get file extension from filename
pub fn get_extension(filename: &str) -> Option<&str> {
    filename.rsplit('.').next()
}
