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

impl Document {
    /// Returns true when the document has enough content to be indexed.
    /// An empty title or very short content is not worth embedding.
    pub fn is_indexable(&self) -> bool {
        !self.title.trim().is_empty() && self.content.trim().len() >= 10
    }

    /// Produce a plain-text representation suitable for embedding generation.
    pub fn to_index_text(&self) -> String {
        format!("{}\n\n{}", self.title.trim(), self.content.trim())
    }

    /// Determine whether `user_id` is allowed to view this document.
    pub fn is_readable_by(&self, user_id: i32) -> bool {
        self.is_public || self.user_id == user_id
    }

    /// Determine whether `user_id` can modify this document.
    pub fn is_writable_by(&self, user_id: i32) -> bool {
        self.user_id == user_id
    }
}

/// DTO for creating a new document
#[derive(Debug, Deserialize)]
pub struct CreateDocument {
    pub title: String,
    pub content: String,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

impl CreateDocument {
    /// Return the effective visibility, defaulting to private.
    pub fn effective_is_public(&self) -> bool {
        self.is_public.unwrap_or(false)
    }

    /// Validate that the request has required fields.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.trim().is_empty() {
            return Err("title must not be empty");
        }
        if self.content.trim().is_empty() {
            return Err("content must not be empty");
        }
        Ok(())
    }
}

/// DTO for updating a document
#[derive(Debug, Deserialize)]
pub struct UpdateDocument {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category_id: Option<i32>,
    pub is_public: Option<bool>,
}

impl UpdateDocument {
    /// Returns true when at least one field is set (i.e. this is a non-empty update).
    pub fn has_changes(&self) -> bool {
        self.title.is_some()
            || self.content.is_some()
            || self.category_id.is_some()
            || self.is_public.is_some()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_doc() -> Document {
        Document {
            id: Uuid::new_v4(),
            title: "Test Document".to_string(),
            content: "This is some content with enough length.".to_string(),
            category_id: None,
            user_id: 42,
            is_public: false,
            view_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // Document::is_indexable

    #[test]
    fn test_is_indexable_with_valid_content() {
        let doc = sample_doc();
        assert!(doc.is_indexable());
    }

    #[test]
    fn test_is_indexable_empty_title_returns_false() {
        let mut doc = sample_doc();
        doc.title = "   ".to_string();
        assert!(!doc.is_indexable());
    }

    #[test]
    fn test_is_indexable_short_content_returns_false() {
        let mut doc = sample_doc();
        doc.content = "hi".to_string(); // < 10 chars
        assert!(!doc.is_indexable());
    }

    #[test]
    fn test_is_indexable_exactly_ten_chars_is_ok() {
        let mut doc = sample_doc();
        doc.content = "0123456789".to_string(); // exactly 10
        assert!(doc.is_indexable());
    }

    // Document::to_index_text

    #[test]
    fn test_to_index_text_contains_title_and_content() {
        let doc = sample_doc();
        let text = doc.to_index_text();
        assert!(text.starts_with("Test Document"));
        assert!(text.contains("This is some content"));
    }

    #[test]
    fn test_to_index_text_trims_whitespace() {
        let mut doc = sample_doc();
        doc.title = "  Padded Title  ".to_string();
        doc.content = "  padded content  ".to_string();
        let text = doc.to_index_text();
        assert!(text.starts_with("Padded Title"));
        assert!(!text.starts_with("  "));
    }

    // Document::is_readable_by

    #[test]
    fn test_readable_by_owner() {
        let doc = sample_doc(); // user_id = 42, is_public = false
        assert!(doc.is_readable_by(42));
    }

    #[test]
    fn test_not_readable_by_other_when_private() {
        let doc = sample_doc();
        assert!(!doc.is_readable_by(99));
    }

    #[test]
    fn test_readable_by_anyone_when_public() {
        let mut doc = sample_doc();
        doc.is_public = true;
        assert!(doc.is_readable_by(99));
    }

    // Document::is_writable_by

    #[test]
    fn test_writable_by_owner() {
        let doc = sample_doc();
        assert!(doc.is_writable_by(42));
    }

    #[test]
    fn test_not_writable_by_other_even_when_public() {
        let mut doc = sample_doc();
        doc.is_public = true;
        assert!(!doc.is_writable_by(99));
    }

    // CreateDocument::effective_is_public

    #[test]
    fn test_effective_is_public_defaults_to_false() {
        let req = CreateDocument {
            title: "T".to_string(),
            content: "C".to_string(),
            category_id: None,
            is_public: None,
        };
        assert!(!req.effective_is_public());
    }

    #[test]
    fn test_effective_is_public_respects_explicit_true() {
        let req = CreateDocument {
            title: "T".to_string(),
            content: "C".to_string(),
            category_id: None,
            is_public: Some(true),
        };
        assert!(req.effective_is_public());
    }

    // CreateDocument::validate

    #[test]
    fn test_validate_accepts_valid_request() {
        let req = CreateDocument {
            title: "Valid".to_string(),
            content: "Valid content".to_string(),
            category_id: None,
            is_public: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_empty_title() {
        let req = CreateDocument {
            title: "   ".to_string(),
            content: "Valid content".to_string(),
            category_id: None,
            is_public: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_empty_content() {
        let req = CreateDocument {
            title: "Title".to_string(),
            content: "  ".to_string(),
            category_id: None,
            is_public: None,
        };
        assert!(req.validate().is_err());
    }

    // UpdateDocument::has_changes

    #[test]
    fn test_has_changes_all_none_returns_false() {
        let req = UpdateDocument {
            title: None,
            content: None,
            category_id: None,
            is_public: None,
        };
        assert!(!req.has_changes());
    }

    #[test]
    fn test_has_changes_with_title_returns_true() {
        let req = UpdateDocument {
            title: Some("New title".to_string()),
            content: None,
            category_id: None,
            is_public: None,
        };
        assert!(req.has_changes());
    }

    #[test]
    fn test_has_changes_with_is_public_only_returns_true() {
        let req = UpdateDocument {
            title: None,
            content: None,
            category_id: None,
            is_public: Some(false),
        };
        assert!(req.has_changes());
    }
}
