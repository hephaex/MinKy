use anyhow::Result;
use sqlx::PgPool;

use crate::{
    error::{AppError, AppResult},
    models::{Category, CategoryTree, CategoryWithCount, CreateCategory, UpdateCategory},
};

/// Category service for business logic
pub struct CategoryService {
    db: PgPool,
}

impl CategoryService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List all categories for a user as a tree
    pub async fn list_tree(&self, user_id: i32) -> Result<Vec<CategoryTree>> {
        let categories = sqlx::query_as::<_, CategoryWithCount>(
            r#"
            SELECT c.id, c.name, c.parent_id, c.user_id,
                   COUNT(d.id) as document_count
            FROM categories c
            LEFT JOIN documents d ON c.id = d.category_id
            WHERE c.user_id = $1
            GROUP BY c.id
            ORDER BY c.name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(CategoryTree::build_tree(categories, None))
    }

    /// List all categories flat
    pub async fn list_flat(&self, user_id: i32) -> Result<Vec<CategoryWithCount>> {
        let categories = sqlx::query_as::<_, CategoryWithCount>(
            r#"
            SELECT c.id, c.name, c.parent_id, c.user_id,
                   COUNT(d.id) as document_count
            FROM categories c
            LEFT JOIN documents d ON c.id = d.category_id
            WHERE c.user_id = $1
            GROUP BY c.id
            ORDER BY c.name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(categories)
    }

    /// Get a single category by ID
    pub async fn get(&self, id: i32, user_id: i32) -> AppResult<Category> {
        let category = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Category not found".to_string()))?;

        Ok(category)
    }

    /// Create a new category
    pub async fn create(&self, user_id: i32, data: CreateCategory) -> AppResult<Category> {
        // Validate parent category if provided
        if let Some(parent_id) = data.parent_id {
            self.get(parent_id, user_id).await?;
        }

        let category = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, parent_id, user_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(&data.name)
        .bind(data.parent_id)
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(category)
    }

    /// Update an existing category
    pub async fn update(&self, id: i32, user_id: i32, data: UpdateCategory) -> AppResult<Category> {
        let existing = self.get(id, user_id).await?;

        // Prevent circular reference
        if let Some(parent_id) = data.parent_id {
            if parent_id == id {
                return Err(AppError::Validation(
                    "Category cannot be its own parent".to_string(),
                ));
            }
            // Check if new parent is a descendant of this category
            if self.is_descendant(parent_id, id, user_id).await? {
                return Err(AppError::Validation(
                    "Cannot set descendant as parent".to_string(),
                ));
            }
        }

        let name = data.name.unwrap_or(existing.name);
        let parent_id = data.parent_id.or(existing.parent_id);

        let category = sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories
            SET name = $1, parent_id = $2, updated_at = NOW()
            WHERE id = $3
            RETURNING *
            "#,
        )
        .bind(&name)
        .bind(parent_id)
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        Ok(category)
    }

    /// Delete a category
    pub async fn delete(&self, id: i32, user_id: i32) -> AppResult<()> {
        // Check if category has documents
        let doc_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM documents WHERE category_id = $1",
        )
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        if doc_count.0 > 0 {
            return Err(AppError::Conflict(
                "Cannot delete category with documents".to_string(),
            ));
        }

        // Move child categories to parent
        let category = self.get(id, user_id).await?;
        sqlx::query(
            "UPDATE categories SET parent_id = $1 WHERE parent_id = $2 AND user_id = $3",
        )
        .bind(category.parent_id)
        .bind(id)
        .bind(user_id)
        .execute(&self.db)
        .await?;

        let result = sqlx::query("DELETE FROM categories WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Category not found".to_string()));
        }

        Ok(())
    }

    /// Check if a category is a descendant of another
    async fn is_descendant(&self, category_id: i32, ancestor_id: i32, user_id: i32) -> Result<bool> {
        let mut current_id = Some(category_id);

        while let Some(id) = current_id {
            if id == ancestor_id {
                return Ok(true);
            }

            let category = sqlx::query_as::<_, Category>(
                "SELECT * FROM categories WHERE id = $1 AND user_id = $2",
            )
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.db)
            .await?;

            current_id = category.and_then(|c| c.parent_id);
        }

        Ok(false)
    }
}
