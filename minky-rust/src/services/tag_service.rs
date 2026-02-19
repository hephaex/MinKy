use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{CreateTag, Tag, TagWithCount, UpdateTag},
};

/// Tag service for business logic
pub struct TagService {
    db: PgPool,
}

impl TagService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List all tags for a user with document counts
    pub async fn list(&self, user_id: i32) -> Result<Vec<TagWithCount>> {
        let tags = sqlx::query_as::<_, TagWithCount>(
            r#"
            SELECT t.id, t.name, t.user_id, t.created_at,
                   COUNT(dt.document_id) as document_count
            FROM tags t
            LEFT JOIN document_tags dt ON t.id = dt.tag_id
            WHERE t.user_id = $1
            GROUP BY t.id
            ORDER BY t.name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(tags)
    }

    /// Get a single tag by ID
    pub async fn get(&self, id: i32, user_id: i32) -> AppResult<Tag> {
        let tag = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Tag not found".to_string()))?;

        Ok(tag)
    }

    /// Create a new tag
    pub async fn create(&self, user_id: i32, data: CreateTag) -> AppResult<Tag> {
        // Check for duplicate tag name
        let existing = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE name = $1 AND user_id = $2",
        )
        .bind(&data.name)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        if existing.is_some() {
            return Err(AppError::Conflict("Tag already exists".to_string()));
        }

        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (name, user_id)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(tag)
    }

    /// Update an existing tag
    pub async fn update(&self, id: i32, user_id: i32, data: UpdateTag) -> AppResult<Tag> {
        let existing = self.get(id, user_id).await?;

        let name = data.name.unwrap_or(existing.name);

        // Check for duplicate tag name
        let duplicate = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE name = $1 AND user_id = $2 AND id != $3",
        )
        .bind(&name)
        .bind(user_id)
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        if duplicate.is_some() {
            return Err(AppError::Conflict("Tag name already exists".to_string()));
        }

        let tag = sqlx::query_as::<_, Tag>(
            r#"
            UPDATE tags SET name = $1 WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(&name)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(tag)
    }

    /// Delete a tag
    pub async fn delete(&self, id: i32, user_id: i32) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM tags WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Tag not found".to_string()));
        }

        Ok(())
    }

    /// Add tags to a document
    pub async fn add_to_document(&self, document_id: Uuid, tag_ids: Vec<i32>, user_id: i32) -> Result<()> {
        for tag_id in tag_ids {
            // Verify tag ownership
            let _ = self.get(tag_id, user_id).await?;

            sqlx::query(
                r#"
                INSERT INTO document_tags (document_id, tag_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(document_id)
            .bind(tag_id)
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    /// Remove tags from a document
    pub async fn remove_from_document(&self, document_id: Uuid, tag_ids: Vec<i32>) -> Result<()> {
        for tag_id in tag_ids {
            sqlx::query("DELETE FROM document_tags WHERE document_id = $1 AND tag_id = $2")
                .bind(document_id)
                .bind(tag_id)
                .execute(&self.db)
                .await?;
        }

        Ok(())
    }

    /// Get tags for a document
    pub async fn get_document_tags(&self, document_id: Uuid) -> Result<Vec<Tag>> {
        let tags = sqlx::query_as::<_, Tag>(
            r#"
            SELECT t.* FROM tags t
            JOIN document_tags dt ON t.id = dt.tag_id
            WHERE dt.document_id = $1
            ORDER BY t.name
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.db)
        .await?;

        Ok(tags)
    }
}

// ---- Pure helper functions (testable without DB) ----

/// Validate a tag name: non-empty, max 100 chars, trimmed
pub fn validate_tag_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Tag name cannot be empty".to_string());
    }
    if trimmed.len() > 100 {
        return Err("Tag name exceeds 100 characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Normalize a tag name to lowercase
pub fn normalize_tag_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Check if two tag names are equivalent (case-insensitive)
pub fn tags_are_duplicate(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// Sort tag names alphabetically
pub fn sort_tag_names(names: &mut [String]) {
    names.sort_by_key(|a| a.to_lowercase());
}

/// Deduplicate a list of tag IDs preserving order
pub fn dedup_tag_ids(ids: Vec<i32>) -> Vec<i32> {
    let mut seen = std::collections::HashSet::new();
    ids.into_iter().filter(|id| seen.insert(*id)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tag_name_valid() {
        assert_eq!(validate_tag_name("rust"), Ok("rust".to_string()));
    }

    #[test]
    fn test_validate_tag_name_trims() {
        assert_eq!(validate_tag_name("  rust  "), Ok("rust".to_string()));
    }

    #[test]
    fn test_validate_tag_name_empty() {
        assert!(validate_tag_name("").is_err());
    }

    #[test]
    fn test_validate_tag_name_whitespace_only() {
        assert!(validate_tag_name("   ").is_err());
    }

    #[test]
    fn test_validate_tag_name_too_long() {
        let long_name = "a".repeat(101);
        assert!(validate_tag_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_tag_name_exactly_100() {
        let name = "a".repeat(100);
        assert!(validate_tag_name(&name).is_ok());
    }

    #[test]
    fn test_normalize_tag_name() {
        assert_eq!(normalize_tag_name("  Rust  "), "rust");
    }

    #[test]
    fn test_tags_are_duplicate_case_insensitive() {
        assert!(tags_are_duplicate("Rust", "rust"));
    }

    #[test]
    fn test_tags_are_duplicate_different() {
        assert!(!tags_are_duplicate("rust", "python"));
    }

    #[test]
    fn test_sort_tag_names() {
        let mut names = vec!["Zebra".to_string(), "apple".to_string(), "Mango".to_string()];
        sort_tag_names(&mut names);
        assert_eq!(names, vec!["apple", "Mango", "Zebra"]);
    }

    #[test]
    fn test_dedup_tag_ids_removes_duplicates() {
        let ids = vec![1, 2, 1, 3, 2];
        let result = dedup_tag_ids(ids);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_dedup_tag_ids_preserves_order() {
        let ids = vec![5, 3, 1, 3, 5];
        let result = dedup_tag_ids(ids);
        assert_eq!(result, vec![5, 3, 1]);
    }
}
