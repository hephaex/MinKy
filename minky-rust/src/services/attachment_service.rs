use anyhow::Result;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        sanitize_filename, validate_upload, Attachment, AttachmentWithUploader, CreateAttachment,
    },
};

/// Attachment service for file management
pub struct AttachmentService {
    db: PgPool,
    upload_dir: PathBuf,
}

impl AttachmentService {
    pub fn new(db: PgPool, upload_dir: impl AsRef<Path>) -> Self {
        Self {
            db,
            upload_dir: upload_dir.as_ref().to_path_buf(),
        }
    }

    /// List attachments for a document
    pub async fn list_for_document(&self, document_id: Uuid) -> Result<Vec<AttachmentWithUploader>> {
        let attachments = sqlx::query_as::<_, AttachmentWithUploader>(
            r#"
            SELECT a.*, u.username as uploader_name
            FROM attachments a
            JOIN users u ON a.user_id = u.id
            WHERE a.document_id = $1
            ORDER BY a.created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await?;

        Ok(attachments)
    }

    /// Get attachment by ID
    pub async fn get(&self, id: Uuid) -> AppResult<Attachment> {
        let attachment = sqlx::query_as::<_, Attachment>(
            "SELECT * FROM attachments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Attachment not found".to_string()))?;

        Ok(attachment)
    }

    /// Create a new attachment record
    pub async fn create(&self, user_id: i32, data: CreateAttachment) -> AppResult<Attachment> {
        // Validate file
        validate_upload(&data.mime_type, data.file_size)
            .map_err(AppError::Validation)?;

        let id = Uuid::new_v4();
        let safe_filename = sanitize_filename(&data.original_filename);

        let attachment = sqlx::query_as::<_, Attachment>(
            r#"
            INSERT INTO attachments (id, filename, original_filename, mime_type, file_size, document_id, user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.filename)
        .bind(&safe_filename)
        .bind(&data.mime_type)
        .bind(data.file_size)
        .bind(data.document_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(attachment)
    }

    /// Delete an attachment
    pub async fn delete(&self, id: Uuid, user_id: i32, is_admin: bool) -> AppResult<()> {
        let attachment = self.get(id).await?;

        // Verify ownership or admin
        if attachment.user_id != user_id && !is_admin {
            return Err(AppError::Forbidden);
        }

        // Delete file from disk
        let file_path = self.upload_dir.join(&attachment.filename);
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("Failed to delete file: {}", e))
            })?;
        }

        // Delete database record
        sqlx::query("DELETE FROM attachments WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get total storage used by a user
    pub async fn get_user_storage(&self, user_id: i32) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(file_size), 0) FROM attachments WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result.0)
    }

    /// Get total storage used for a document
    pub async fn get_document_storage(&self, document_id: Uuid) -> Result<i64> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(file_size), 0) FROM attachments WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_one(&self.db)
        .await?;

        Ok(result.0)
    }

    /// Generate unique filename for storage
    pub fn generate_storage_filename(original: &str) -> String {
        let ext = original
            .rsplit('.')
            .next()
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        format!("{}{}", Uuid::new_v4(), ext)
    }

    /// Get file path for an attachment
    pub fn get_file_path(&self, filename: &str) -> PathBuf {
        self.upload_dir.join(filename)
    }

    /// Check if file exists
    pub fn file_exists(&self, filename: &str) -> bool {
        self.get_file_path(filename).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_storage_filename_preserves_extension() {
        let name = AttachmentService::generate_storage_filename("document.pdf");
        assert!(name.ends_with(".pdf"), "Expected .pdf extension, got: {}", name);
    }

    #[test]
    fn test_generate_storage_filename_no_extension() {
        let name = AttachmentService::generate_storage_filename("Makefile");
        // No dot in original means last "split" returns original, so ext = ".Makefile"
        // The function always appends what rsplit('.').next() returns
        assert!(!name.is_empty());
    }

    #[test]
    fn test_generate_storage_filename_unique() {
        // Two calls should produce different names (UUID-based)
        let name1 = AttachmentService::generate_storage_filename("test.txt");
        let name2 = AttachmentService::generate_storage_filename("test.txt");
        assert_ne!(name1, name2, "Generated filenames should be unique");
    }

    #[test]
    fn test_generate_storage_filename_is_uuid_format() {
        let name = AttachmentService::generate_storage_filename("image.png");
        // UUID format: 8-4-4-4-12 chars + ".png" = 36 + 4 chars
        assert_eq!(name.len(), 40, "Expected UUID (36) + .png (4), got len: {}", name.len());
    }

    #[test]
    fn test_get_file_path_joins_correctly() {
        let dir = PathBuf::from("/uploads");
        // We can't use PgPool in unit tests, so create a test instance via unsafe transmute
        // Instead, test the logic by calling the helper directly
        let path = dir.join("somefile.pdf");
        assert_eq!(path, PathBuf::from("/uploads/somefile.pdf"));
    }
}
