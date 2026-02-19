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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_comment(id: i32, content: &str, parent_id: Option<i32>) -> CommentFlat {
        CommentFlat {
            id,
            content: content.to_string(),
            document_id: Uuid::nil(),
            user_id: 1,
            author_name: "Author".to_string(),
            parent_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_build_tree_empty() {
        let tree = CommentWithAuthor::build_tree(vec![], None);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_build_tree_top_level_comments() {
        let comments = vec![
            make_comment(1, "First comment", None),
            make_comment(2, "Second comment", None),
        ];
        let tree = CommentWithAuthor::build_tree(comments, None);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].content, "First comment");
        assert_eq!(tree[1].content, "Second comment");
        assert!(tree[0].replies.is_empty());
    }

    #[test]
    fn test_build_tree_with_replies() {
        let comments = vec![
            make_comment(1, "Top comment", None),
            make_comment(2, "Reply to 1", Some(1)),
            make_comment(3, "Another top", None),
        ];
        let tree = CommentWithAuthor::build_tree(comments, None);

        assert_eq!(tree.len(), 2);
        let top = tree.iter().find(|c| c.id == 1).unwrap();
        assert_eq!(top.replies.len(), 1);
        assert_eq!(top.replies[0].content, "Reply to 1");

        let other = tree.iter().find(|c| c.id == 3).unwrap();
        assert!(other.replies.is_empty());
    }

    #[test]
    fn test_build_tree_nested_replies() {
        let comments = vec![
            make_comment(1, "Root", None),
            make_comment(2, "Child of 1", Some(1)),
            make_comment(3, "Child of 2", Some(2)),
        ];
        let tree = CommentWithAuthor::build_tree(comments, None);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].replies.len(), 1);
        assert_eq!(tree[0].replies[0].replies.len(), 1);
        assert_eq!(tree[0].replies[0].replies[0].content, "Child of 2");
    }
}
