use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Document model representing the documents table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub user_id: i32,
    pub is_public: bool,
    pub view_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new document
#[derive(Debug, Deserialize)]
pub struct CreateDocument {
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

/// DTO for updating a document
#[derive(Debug, Deserialize)]
pub struct UpdateDocument {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

/// Document with related data
#[derive(Debug, Serialize)]
pub struct DocumentWithRelations {
    #[serde(flatten)]
    pub document: Document,
    pub author_name: String,
    pub category_name: Option<String>,
    pub tags: Vec<String>,
}
