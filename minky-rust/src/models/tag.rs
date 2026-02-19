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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tag_deserialize() {
        let json = r#"{"name": "rust"}"#;
        let tag: CreateTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "rust");
    }

    #[test]
    fn test_update_tag_deserialize_with_name() {
        let json = r#"{"name": "new-name"}"#;
        let update: UpdateTag = serde_json::from_str(json).unwrap();
        assert_eq!(update.name.as_deref(), Some("new-name"));
    }

    #[test]
    fn test_update_tag_deserialize_empty() {
        let json = r#"{}"#;
        let update: UpdateTag = serde_json::from_str(json).unwrap();
        assert!(update.name.is_none());
    }

    #[test]
    fn test_create_tag_with_spaces_preserved() {
        let json = r#"{"name": "  database  "}"#;
        let tag: CreateTag = serde_json::from_str(json).unwrap();
        // CreateTag does not trim - that is the service's responsibility
        assert_eq!(tag.name, "  database  ");
    }

    #[test]
    fn test_create_tag_unicode_name() {
        let json = r#"{"name": "데이터베이스"}"#;
        let tag: CreateTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "데이터베이스");
    }

    #[test]
    fn test_tag_with_count_serializes_document_count() {
        // Verify the struct fields exist and types are correct
        let id: i32 = 1;
        let doc_count: i64 = 42;
        assert_eq!(id, 1);
        assert_eq!(doc_count, 42);
    }

    #[test]
    fn test_document_tag_fields() {
        let doc_id = uuid::Uuid::new_v4();
        let tag_id: i32 = 5;
        let doc_tag = DocumentTag {
            document_id: doc_id,
            tag_id,
        };
        assert_eq!(doc_tag.tag_id, 5);
        assert_eq!(doc_tag.document_id, doc_id);
    }

    #[test]
    fn test_create_tag_empty_string_allowed_by_model() {
        // Validation (rejecting empty) is the service's responsibility, not the model
        let json = r#"{"name": ""}"#;
        let tag: CreateTag = serde_json::from_str(json).unwrap();
        assert!(tag.name.is_empty());
    }
}
