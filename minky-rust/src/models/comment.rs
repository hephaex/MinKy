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

    #[test]
    fn test_comment_structure() {
        let doc_id = Uuid::new_v4();
        let comment = Comment {
            id: 1,
            content: "Great document!".to_string(),
            document_id: doc_id,
            user_id: 5,
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(comment.id, 1);
        assert_eq!(comment.user_id, 5);
        assert_eq!(comment.document_id, doc_id);
        assert_eq!(comment.parent_id, None);
    }

    #[test]
    fn test_create_comment_top_level() {
        let doc_id = Uuid::new_v4();
        let create = CreateComment {
            content: "New comment".to_string(),
            document_id: doc_id,
            parent_id: None,
        };

        assert_eq!(create.content, "New comment");
        assert_eq!(create.document_id, doc_id);
        assert_eq!(create.parent_id, None);
    }

    #[test]
    fn test_create_comment_reply() {
        let create = CreateComment {
            content: "This is a reply".to_string(),
            document_id: Uuid::new_v4(),
            parent_id: Some(42),
        };

        assert_eq!(create.parent_id, Some(42));
    }

    #[test]
    fn test_update_comment() {
        let update = UpdateComment {
            content: "Updated content".to_string(),
        };

        assert_eq!(update.content, "Updated content");
    }

    #[test]
    fn test_comment_flat_structure() {
        let flat = CommentFlat {
            id: 5,
            content: "Test comment".to_string(),
            document_id: Uuid::nil(),
            user_id: 10,
            author_name: "John Doe".to_string(),
            parent_id: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(flat.id, 5);
        assert_eq!(flat.author_name, "John Doe");
        assert_eq!(flat.user_id, 10);
        assert_eq!(flat.parent_id, Some(1));
    }

    #[test]
    fn test_comment_with_author_structure() {
        let author_comment = CommentWithAuthor {
            id: 1,
            content: "Authored comment".to_string(),
            document_id: Uuid::nil(),
            user_id: 1,
            author_name: "Alice".to_string(),
            parent_id: None,
            replies: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(author_comment.author_name, "Alice");
        assert!(author_comment.replies.is_empty());
    }

    #[test]
    fn test_build_tree_multiple_roots_with_replies() {
        let comments = vec![
            make_comment(1, "First root", None),
            make_comment(2, "Reply to 1", Some(1)),
            make_comment(3, "Second root", None),
            make_comment(4, "Reply to 3", Some(3)),
            make_comment(5, "Another reply to 3", Some(3)),
        ];
        let tree = CommentWithAuthor::build_tree(comments, None);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].replies.len(), 1);
        assert_eq!(tree[1].replies.len(), 2);
    }

    #[test]
    fn test_comment_timestamps_preserved() {
        let now = Utc::now();
        let flat = CommentFlat {
            id: 1,
            content: "Test".to_string(),
            document_id: Uuid::nil(),
            user_id: 1,
            author_name: "Test".to_string(),
            parent_id: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(flat.created_at, now);
        assert_eq!(flat.updated_at, now);
    }

    #[test]
    fn test_build_tree_preserves_all_fields() {
        let now = Utc::now();
        let doc_id = Uuid::new_v4();
        let comments = vec![CommentFlat {
            id: 99,
            content: "Test content".to_string(),
            document_id: doc_id,
            user_id: 42,
            author_name: "TestAuthor".to_string(),
            parent_id: None,
            created_at: now,
            updated_at: now,
        }];

        let tree = CommentWithAuthor::build_tree(comments, None);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 99);
        assert_eq!(tree[0].user_id, 42);
        assert_eq!(tree[0].document_id, doc_id);
        assert_eq!(tree[0].created_at, now);
    }
}
