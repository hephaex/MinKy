use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{CreateVersion, Version, VersionWithAuthor},
};

/// Version service for document version management
pub struct VersionService {
    db: PgPool,
}

impl VersionService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List versions for a document
    pub async fn list_for_document(&self, document_id: Uuid) -> Result<Vec<VersionWithAuthor>> {
        let versions = sqlx::query_as::<_, VersionWithAuthor>(
            r#"
            SELECT v.*, u.username as author_name
            FROM document_versions v
            JOIN users u ON v.created_by = u.id
            WHERE v.document_id = $1
            ORDER BY v.version_number DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await?;

        Ok(versions)
    }

    /// Get a specific version
    pub async fn get(&self, id: i32) -> AppResult<Version> {
        let version = sqlx::query_as::<_, Version>(
            "SELECT * FROM document_versions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Version not found".to_string()))?;

        Ok(version)
    }

    /// Get version by document and version number
    pub async fn get_by_number(&self, document_id: Uuid, version_number: i32) -> AppResult<Version> {
        let version = sqlx::query_as::<_, Version>(
            "SELECT * FROM document_versions WHERE document_id = $1 AND version_number = $2",
        )
        .bind(document_id)
        .bind(version_number)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Version not found".to_string()))?;

        Ok(version)
    }

    /// Get latest version for a document
    pub async fn get_latest(&self, document_id: Uuid) -> AppResult<Version> {
        let version = sqlx::query_as::<_, Version>(
            r#"
            SELECT * FROM document_versions
            WHERE document_id = $1
            ORDER BY version_number DESC
            LIMIT 1
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("No versions found".to_string()))?;

        Ok(version)
    }

    /// Create a new version (auto-increment version number)
    pub async fn create(&self, user_id: i32, data: CreateVersion) -> Result<Version> {
        // Get next version number
        let last_version: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT MAX(version_number) FROM document_versions WHERE document_id = $1
            "#,
        )
        .bind(data.document_id)
        .fetch_optional(&self.db)
        .await?;

        let next_version = last_version.map(|v| v.0 + 1).unwrap_or(1);

        let version = sqlx::query_as::<_, Version>(
            r#"
            INSERT INTO document_versions (document_id, content, version_number, created_by)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(data.document_id)
        .bind(&data.content)
        .bind(next_version)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(version)
    }

    /// Restore document to a specific version
    pub async fn restore(&self, document_id: Uuid, version_number: i32, user_id: i32) -> AppResult<Version> {
        let old_version = self.get_by_number(document_id, version_number).await?;

        // Create new version with old content
        let new_version = self
            .create(
                user_id,
                CreateVersion {
                    document_id,
                    content: old_version.content.clone(),
                },
            )
            .await?;

        // Update document content
        sqlx::query(
            r#"
            UPDATE documents
            SET content = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(&old_version.content)
        .bind(document_id)
        .execute(&self.db)
        .await?;

        Ok(new_version)
    }

    /// Get version count for a document
    pub async fn count_for_document(&self, document_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM document_versions WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_one(&self.db)
        .await?;

        Ok(count.0)
    }

    /// Delete old versions (keep latest N versions)
    pub async fn cleanup_old_versions(&self, document_id: Uuid, keep_count: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM document_versions
            WHERE document_id = $1
            AND version_number NOT IN (
                SELECT version_number FROM document_versions
                WHERE document_id = $1
                ORDER BY version_number DESC
                LIMIT $2
            )
            "#,
        )
        .bind(document_id)
        .bind(keep_count)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Compare two versions (simple diff)
    pub fn compare_versions(old_content: &str, new_content: &str) -> VersionDiff {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let mut additions = 0;
        let mut deletions = 0;

        // Simple line-by-line comparison
        let max_len = old_lines.len().max(new_lines.len());

        for i in 0..max_len {
            let old_line = old_lines.get(i);
            let new_line = new_lines.get(i);

            match (old_line, new_line) {
                (Some(o), Some(n)) if o != n => {
                    deletions += 1;
                    additions += 1;
                }
                (Some(_), None) => deletions += 1,
                (None, Some(_)) => additions += 1,
                _ => {}
            }
        }

        VersionDiff {
            additions,
            deletions,
            total_changes: additions + deletions,
        }
    }
}

/// Simple version diff result
#[derive(Debug, serde::Serialize)]
pub struct VersionDiff {
    pub additions: i32,
    pub deletions: i32,
    pub total_changes: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_identical_content() {
        let diff = VersionService::compare_versions("hello\nworld", "hello\nworld");
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.total_changes, 0);
    }

    #[test]
    fn test_compare_empty_to_content() {
        let diff = VersionService::compare_versions("", "line1\nline2");
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.total_changes, 2);
    }

    #[test]
    fn test_compare_content_to_empty() {
        let diff = VersionService::compare_versions("line1\nline2", "");
        assert_eq!(diff.additions, 0);
        assert_eq!(diff.deletions, 2);
        assert_eq!(diff.total_changes, 2);
    }

    #[test]
    fn test_compare_modified_lines() {
        let diff = VersionService::compare_versions("old line\nsame", "new line\nsame");
        assert_eq!(diff.additions, 1);
        assert_eq!(diff.deletions, 1);
        assert_eq!(diff.total_changes, 2);
    }

    #[test]
    fn test_compare_added_lines() {
        // new content has more lines
        let diff = VersionService::compare_versions("line1", "line1\nline2\nline3");
        assert_eq!(diff.additions, 2);
        assert_eq!(diff.deletions, 0);
        assert_eq!(diff.total_changes, 2);
    }

    #[test]
    fn test_compare_total_equals_sum() {
        let diff = VersionService::compare_versions("a\nb\nc", "a\nx\ny\nz");
        assert_eq!(diff.total_changes, diff.additions + diff.deletions);
    }
}
