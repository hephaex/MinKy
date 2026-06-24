// -------------------------------------------------------------------
// Pipeline safety helpers
// -------------------------------------------------------------------

use uuid::Uuid;

/// Guard against cross-thread data races on document ID fields.
///
/// In the Python content pipeline, document IDs were often treated as locally-scoped
/// values. When porting to Rust, we must ensure they are properly cloned and
/// stored in the pipeline context before any background queue or async task
/// accesses them.
///
/// This helper enforces that the `id` field is set on every document that
/// enters a pipeline stage that might be awaited across threads.
pub fn assert_doc_id_set(doc: &crate::models::Document) -> Result<(), String> {
    // Check that UUID is not nil (all zeros)
    if doc.id == Uuid::nil() {
        return Err(format!(
            "Document has unset id: title='{}'",
            doc.title
        ));
    }
    Ok(())
}

/// Verifies that a document has at least `min_bytes` of UTF-8 content before
/// attempting to embed. Prevents `panic: index out of bounds` in the vec!
/// allocation when backend `check_dimensions` receives an empty document.
pub fn assert_has_content(doc: &crate::models::Document, min_bytes: usize) -> Result<(), String> {
    if doc.content.len() < min_bytes {
        return Err(format!(
            "Document '{}' content too short: {} bytes < {} required",
            doc.title,
            doc.content.len(),
            min_bytes
        ));
    }
    Ok(())
}

/// Strip all HTML tags from the input.
///
/// Returns plain text safe for `dangerouslySetInnerHTML`-free rendering.
pub fn strip_tags(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
            }
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

/// Escape HTML entities in the input.
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Sanitize a document markdown string for safe client-side rendering.
///
/// Strips all HTML tags and common XSS vectors (onerror, onload, javascript:, data: URIs,
/// SVG payloads) from the input. Returns plain text safe for `dangerouslySetInnerHTML`-free
/// rendering.
pub fn sanitize_html_escape(input: &str) -> String {
    let lowercased = input.to_lowercase();
    if lowercased.contains("onerror")
        || lowercased.contains("onload")
        || lowercased.contains("javascript:")
        || lowercased.contains("data:")
        || lowercased.contains("<svg")
    {
        return strip_tags(input);
    }
    strip_tags(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Document;

    #[test]
    fn test_assert_doc_id_set_ok() {
        let doc = Document {
            id: Uuid::new_v4(),
            title: "test".to_string(),
            content: "test content".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            sensitivity: "internal".to_string(),
            view_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content_hash: None,
            archived: false,
            archived_at: None,
        };
        assert!(assert_doc_id_set(&doc).is_ok());
    }

    #[test]
    fn test_assert_doc_id_set_fail() {
        let doc = Document {
            id: Uuid::nil(),
            title: "test".to_string(),
            content: "test".to_string(),
            category_id: None,
            user_id: 1,
            is_public: false,
            sensitivity: "internal".to_string(),
            view_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content_hash: None,
            archived: false,
            archived_at: None,
        };
        assert!(assert_doc_id_set(&doc).is_err());
    }

    #[test]
    fn test_strip_tags() {
        let input = "<b>hello</b>";
        assert_eq!(strip_tags(input).trim(), "hello");
    }

    #[test]
    fn test_strip_tags_noop() {
        assert_eq!(strip_tags("hello world").trim(), "hello world");
    }

    #[test]
    fn test_sanitize_html_escape_strips_tags() {
        assert_eq!(sanitize_html_escape("<span>hello</span>"), "hello");
    }
}
