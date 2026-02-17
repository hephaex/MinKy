use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Tag model representing the tags table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating a new tag
#[derive(Debug, Deserialize)]
pub struct CreateTag {
    pub name: String,
}

/// DTO for updating a tag
#[derive(Debug, Deserialize)]
pub struct UpdateTag {
    pub name: Option<String>,
}

/// Tag with document count
#[derive(Debug, Serialize, FromRow)]
pub struct TagWithCount {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub document_count: i64,
    pub created_at: DateTime<Utc>,
}

/// Document-Tag association
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DocumentTag {
    pub document_id: uuid::Uuid,
    pub tag_id: i32,
}
