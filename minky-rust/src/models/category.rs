use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Category model representing the categories table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new category
#[derive(Debug, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<i32>,
}

/// DTO for updating a category
#[derive(Debug, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub parent_id: Option<i32>,
}

/// Category with document count and children
#[derive(Debug, Serialize)]
pub struct CategoryTree {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub document_count: i64,
    pub children: Vec<CategoryTree>,
}

impl CategoryTree {
    /// Build a tree structure from flat categories
    pub fn build_tree(categories: Vec<CategoryWithCount>, parent_id: Option<i32>) -> Vec<Self> {
        categories
            .iter()
            .filter(|c| c.parent_id == parent_id)
            .map(|c| CategoryTree {
                id: c.id,
                name: c.name.clone(),
                parent_id: c.parent_id,
                document_count: c.document_count,
                children: Self::build_tree(categories.clone(), Some(c.id)),
            })
            .collect()
    }
}

/// Category with document count
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CategoryWithCount {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub user_id: i32,
    pub document_count: i64,
}
