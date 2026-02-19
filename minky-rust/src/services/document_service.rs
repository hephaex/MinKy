use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{CreateDocument, Document, UpdateDocument},
};

/// Document service for business logic
pub struct DocumentService {
    db: PgPool,
}

impl DocumentService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List documents with pagination
    pub async fn list(
        &self,
        user_id: i32,
        page: i32,
        limit: i32,
        category_id: Option<i32>,
    ) -> Result<(Vec<Document>, i64)> {
        let offset = (page - 1) * limit;

        let total: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM documents
            WHERE user_id = $1 OR is_public = true
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        let documents = if let Some(cat_id) = category_id {
            sqlx::query_as::<_, Document>(
                r#"
                SELECT * FROM documents
                WHERE (user_id = $1 OR is_public = true) AND category_id = $2
                ORDER BY updated_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(user_id)
            .bind(cat_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as::<_, Document>(
                r#"
                SELECT * FROM documents
                WHERE user_id = $1 OR is_public = true
                ORDER BY updated_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        };

        Ok((documents, total.0))
    }

    /// Get a single document by ID
    pub async fn get(&self, id: Uuid, user_id: i32) -> AppResult<Document> {
        let document = sqlx::query_as::<_, Document>(
            r#"
            SELECT * FROM documents
            WHERE id = $1 AND (user_id = $2 OR is_public = true)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

        // Increment view count
        sqlx::query("UPDATE documents SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(document)
    }

    /// Create a new document
    pub async fn create(&self, user_id: i32, data: CreateDocument) -> Result<Document> {
        let id = Uuid::new_v4();
        let is_public = data.is_public.unwrap_or(false);

        let document = sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (id, title, content, category_id, user_id, is_public)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.content)
        .bind(data.category_id)
        .bind(user_id)
        .bind(is_public)
        .fetch_one(&self.db)
        .await?;

        Ok(document)
    }

    /// Update an existing document
    pub async fn update(&self, id: Uuid, user_id: i32, data: UpdateDocument) -> AppResult<Document> {
        // Verify ownership
        let existing = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Document not found or access denied".to_string()))?;

        let title = data.title.unwrap_or(existing.title);
        let content = data.content.unwrap_or(existing.content);
        let category_id = data.category_id.or(existing.category_id);
        let is_public = data.is_public.unwrap_or(existing.is_public);

        let document = sqlx::query_as::<_, Document>(
            r#"
            UPDATE documents
            SET title = $1, content = $2, category_id = $3, is_public = $4, updated_at = NOW()
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(&title)
        .bind(&content)
        .bind(category_id)
        .bind(is_public)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(document)
    }

    /// Delete a document
    pub async fn delete(&self, id: Uuid, user_id: i32) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM documents WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Document not found or access denied".to_string(),
            ));
        }

        Ok(())
    }
}

// ---- Pure helper functions (testable without DB) ----

/// Calculate the offset for pagination from page number and limit
pub fn calc_offset(page: i32, limit: i32) -> i32 {
    ((page - 1).max(0)) * limit.max(1)
}

/// Clamp pagination parameters to safe values
pub fn clamp_page_params(page: i32, limit: i32) -> (i32, i32) {
    let safe_page = page.max(1);
    let safe_limit = limit.clamp(1, 100);
    (safe_page, safe_limit)
}

/// Calculate total pages from total count and page size
pub fn total_pages(total: i64, limit: i32) -> i64 {
    if limit <= 0 {
        return 0;
    }
    (total + limit as i64 - 1) / limit as i64
}

/// Determine whether a user can read a document
pub fn can_read_document(doc_user_id: i32, doc_is_public: bool, requester_id: i32) -> bool {
    doc_is_public || doc_user_id == requester_id
}

/// Determine whether a user can write (modify/delete) a document
pub fn can_write_document(doc_user_id: i32, requester_id: i32) -> bool {
    doc_user_id == requester_id
}

/// Build a search filter SQL snippet (simple pattern)
pub fn build_search_pattern(query: &str) -> String {
    format!("%{}%", query.trim().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_offset_page1() {
        assert_eq!(calc_offset(1, 20), 0);
    }

    #[test]
    fn test_calc_offset_page2() {
        assert_eq!(calc_offset(2, 20), 20);
    }

    #[test]
    fn test_calc_offset_page3() {
        assert_eq!(calc_offset(3, 10), 20);
    }

    #[test]
    fn test_calc_offset_zero_page_clamps() {
        assert_eq!(calc_offset(0, 10), 0);
    }

    #[test]
    fn test_clamp_page_params_valid() {
        assert_eq!(clamp_page_params(2, 25), (2, 25));
    }

    #[test]
    fn test_clamp_page_params_zero_page() {
        assert_eq!(clamp_page_params(0, 20).0, 1);
    }

    #[test]
    fn test_clamp_page_params_overlimit() {
        assert_eq!(clamp_page_params(1, 9999).1, 100);
    }

    #[test]
    fn test_total_pages_exact() {
        assert_eq!(total_pages(100, 10), 10);
    }

    #[test]
    fn test_total_pages_remainder() {
        assert_eq!(total_pages(101, 10), 11);
    }

    #[test]
    fn test_total_pages_zero_total() {
        assert_eq!(total_pages(0, 10), 0);
    }

    #[test]
    fn test_total_pages_zero_limit() {
        assert_eq!(total_pages(10, 0), 0);
    }

    #[test]
    fn test_can_read_document_public() {
        assert!(can_read_document(1, true, 99));
    }

    #[test]
    fn test_can_read_document_owner() {
        assert!(can_read_document(5, false, 5));
    }

    #[test]
    fn test_can_read_document_non_owner_private() {
        assert!(!can_read_document(5, false, 99));
    }

    #[test]
    fn test_can_write_document_owner() {
        assert!(can_write_document(7, 7));
    }

    #[test]
    fn test_can_write_document_non_owner() {
        assert!(!can_write_document(7, 99));
    }

    #[test]
    fn test_build_search_pattern() {
        assert_eq!(build_search_pattern("Rust"), "%rust%");
    }

    #[test]
    fn test_build_search_pattern_trims_whitespace() {
        assert_eq!(build_search_pattern("  Rust  "), "%rust%");
    }
}
