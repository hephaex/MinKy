use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{Comment, CommentFlat, CommentWithAuthor, CreateComment, UpdateComment},
    utils::sanitize_html,
};

/// Comment service for business logic
pub struct CommentService {
    db: PgPool,
}

impl CommentService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List comments for a document as a threaded tree
    pub async fn list_for_document(&self, document_id: Uuid) -> Result<Vec<CommentWithAuthor>> {
        let comments = sqlx::query_as::<_, CommentFlat>(
            r#"
            SELECT c.id, c.content, c.document_id, c.user_id, c.parent_id,
                   c.created_at, c.updated_at, u.username as author_name
            FROM comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.document_id = $1
            ORDER BY c.created_at ASC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await?;

        Ok(CommentWithAuthor::build_tree(comments, None))
    }

    /// Get a single comment by ID
    pub async fn get(&self, id: i32) -> AppResult<Comment> {
        let comment = sqlx::query_as::<_, Comment>("SELECT * FROM comments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        Ok(comment)
    }

    /// Create a new comment
    pub async fn create(&self, user_id: i32, data: CreateComment) -> AppResult<Comment> {
        // Validate parent comment if provided
        if let Some(parent_id) = data.parent_id {
            let parent = self.get(parent_id).await?;
            if parent.document_id != data.document_id {
                return Err(AppError::Validation(
                    "Parent comment must be on the same document".to_string(),
                ));
            }
        }

        // Sanitize content to prevent XSS
        let sanitized_content = sanitize_html(&data.content);

        let comment = sqlx::query_as::<_, Comment>(
            r#"
            INSERT INTO comments (content, document_id, user_id, parent_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&sanitized_content)
        .bind(data.document_id)
        .bind(user_id)
        .bind(data.parent_id)
        .fetch_one(&self.db)
        .await?;

        Ok(comment)
    }

    /// Update an existing comment
    pub async fn update(&self, id: i32, user_id: i32, data: UpdateComment) -> AppResult<Comment> {
        let existing = self.get(id).await?;

        // Verify ownership
        if existing.user_id != user_id {
            return Err(AppError::Forbidden);
        }

        // Sanitize content to prevent XSS
        let sanitized_content = sanitize_html(&data.content);

        let comment = sqlx::query_as::<_, Comment>(
            r#"
            UPDATE comments
            SET content = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(&sanitized_content)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(comment)
    }

    /// Delete a comment
    pub async fn delete(&self, id: i32, user_id: i32, is_admin: bool) -> AppResult<()> {
        let existing = self.get(id).await?;

        // Verify ownership or admin
        if existing.user_id != user_id && !is_admin {
            return Err(AppError::Forbidden);
        }

        // Delete the comment and all replies (cascade)
        sqlx::query("DELETE FROM comments WHERE id = $1")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get comment count for a document
    pub async fn count_for_document(&self, document_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM comments WHERE document_id = $1",
        )
        .bind(document_id)
        .fetch_one(&self.db)
        .await?;

        Ok(count.0)
    }
}

// ---- Pure helper functions (testable without DB) ----

/// Check if a user can edit a comment (owns it)
pub fn can_edit_comment(comment_user_id: i32, requester_user_id: i32) -> bool {
    comment_user_id == requester_user_id
}

/// Check if a user can delete a comment (owns it or is admin)
pub fn can_delete_comment(comment_user_id: i32, requester_user_id: i32, is_admin: bool) -> bool {
    comment_user_id == requester_user_id || is_admin
}

/// Check if a parent comment belongs to the same document
pub fn is_valid_parent(parent_document_id: Uuid, child_document_id: Uuid) -> bool {
    parent_document_id == child_document_id
}

/// Truncate comment content to maximum length
pub fn truncate_comment(content: &str, max_len: usize) -> &str {
    if content.len() <= max_len {
        content
    } else {
        &content[..max_len]
    }
}

/// Check if comment content is non-empty after trimming
pub fn is_valid_comment_content(content: &str) -> bool {
    !content.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_edit_comment_owner() {
        assert!(can_edit_comment(42, 42));
    }

    #[test]
    fn test_can_edit_comment_non_owner() {
        assert!(!can_edit_comment(42, 99));
    }

    #[test]
    fn test_can_delete_comment_owner() {
        assert!(can_delete_comment(5, 5, false));
    }

    #[test]
    fn test_can_delete_comment_admin() {
        assert!(can_delete_comment(5, 99, true));
    }

    #[test]
    fn test_can_delete_comment_non_owner_non_admin() {
        assert!(!can_delete_comment(5, 99, false));
    }

    #[test]
    fn test_is_valid_parent_same_document() {
        let doc_id = Uuid::new_v4();
        assert!(is_valid_parent(doc_id, doc_id));
    }

    #[test]
    fn test_is_valid_parent_different_document() {
        let doc1 = Uuid::new_v4();
        let doc2 = Uuid::new_v4();
        assert!(!is_valid_parent(doc1, doc2));
    }

    #[test]
    fn test_truncate_comment_short() {
        let s = "Hello";
        assert_eq!(truncate_comment(s, 100), "Hello");
    }

    #[test]
    fn test_truncate_comment_exact() {
        let s = "Hello";
        assert_eq!(truncate_comment(s, 5), "Hello");
    }

    #[test]
    fn test_truncate_comment_long() {
        let s = "Hello World";
        assert_eq!(truncate_comment(s, 5), "Hello");
    }

    #[test]
    fn test_is_valid_comment_content_non_empty() {
        assert!(is_valid_comment_content("some text"));
    }

    #[test]
    fn test_is_valid_comment_content_whitespace_only() {
        assert!(!is_valid_comment_content("   "));
    }

    #[test]
    fn test_is_valid_comment_content_empty() {
        assert!(!is_valid_comment_content(""));
    }
}
