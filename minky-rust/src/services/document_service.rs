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
