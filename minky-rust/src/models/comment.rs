use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Comment model representing the comments table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Comment {
    pub id: i32,
    pub content: String,
    pub document_id: Uuid,
    pub user_id: i32,
    pub parent_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating a new comment
#[derive(Debug, Deserialize)]
pub struct CreateComment {
    pub content: String,
    pub document_id: Uuid,
    pub parent_id: Option<i32>,
}

/// DTO for updating a comment
#[derive(Debug, Deserialize)]
pub struct UpdateComment {
    pub content: String,
}

/// Comment with author information
#[derive(Debug, Serialize)]
pub struct CommentWithAuthor {
    pub id: i32,
    pub content: String,
    pub document_id: Uuid,
    pub user_id: i32,
    pub author_name: String,
    pub parent_id: Option<i32>,
    pub replies: Vec<CommentWithAuthor>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CommentWithAuthor {
    /// Build a threaded comment tree from flat comments
    pub fn build_tree(comments: Vec<CommentFlat>, parent_id: Option<i32>) -> Vec<Self> {
        comments
            .iter()
            .filter(|c| c.parent_id == parent_id)
            .map(|c| CommentWithAuthor {
                id: c.id,
                content: c.content.clone(),
                document_id: c.document_id,
                user_id: c.user_id,
                author_name: c.author_name.clone(),
                parent_id: c.parent_id,
                replies: Self::build_tree(comments.clone(), Some(c.id)),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect()
    }
}

/// Flat comment with author name (from JOIN query)
#[derive(Debug, Clone, FromRow)]
pub struct CommentFlat {
    pub id: i32,
    pub content: String,
    pub document_id: Uuid,
    pub user_id: i32,
    pub author_name: String,
    pub parent_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
