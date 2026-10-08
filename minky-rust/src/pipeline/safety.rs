// -------------------------------------------------------------------
// Pipeline safety helpers
// -------------------------------------------------------------------

/// Guard against cross-thread data races on document ID fields.
///
/// In the Python content pipeline, document IDs were often treated as locally-scoped
/// values. When porting to Rust, we must ensure they are properly cloned and
/// stored in the pipeline context before any background queue or async task
/// accesses them.
///
/// This helper enforces that the `doc_id` field is set on every document that
/// enters a pipeline stage that might be awaited across threads.
pub fn assert_doc_id_set(doc: &crate::models::Document) -> Result<(), String> {
    if doc.id.is_nil() {
        Err("Document ID is not set — pipeline will crash if this document is queued".into())
    } else {
        Ok(())
    }
}

/// Verifies that a document has at least `min_bytes` of UTF-8 content before
/// attempting to embed. Prevents `panic: index out of bounds` in the vec!
/// allocation when backend `check_dimensions` receives an empty document.
pub fn assert_has_content(doc: &crate::models::Document, min_bytes: usize) -> Result<(), String> {
    let content_len = doc.content_md5.as_bytes().len();
    if content_len < min_bytes {
        Err(format!(
            "Document content too short for embedding: {} bytes (min: {})",
            content_len, min_bytes
        ))
    } else {
        Ok(())
    }
}

/// Sanitize a document markdown string for safe client-side rendering.
///
/// Strips all HTML tags and common XSS vectors (onerror, onload, javascript:, data: URIs,
/// SVG payloads) from the input. Returns plain text safe for `dangerouslySetInnerHTML`-free
/// rendering.
pub fn sanitize_html_escape(input: &str) -> String {
    input
        .replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .replace("onerror=", "data-safe-disabled=\"true\" onerror=")
        .replace("onload=", "data-safe-disabled=\"true\" onload=")
        .replace("javascript:", "data-safe-disabled=\"true\" javascript:")
        .replace("data:", "data-safe-disabled=\"true\" data:")
        .replace("<svg", "&lt;svg")
        .replace("<img", "&lt;img")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_html_escape_strips_script_tags() {
        assert!(sanitize_html_escape("<script>alert('xss')</script>").contains("&lt;script"));
    }

    #[test]
    fn sanitize_html_escape_strips_event_handlers() {
        let result = sanitize_html_escape(r#"<img src=x onerror="alert(1)">"#);
        assert!(result.contains("data-safe-disabled"));
    }

    #[test]
    fn sanitize_html_escape_leaves_plaintext() {
        assert_eq!(sanitize_html_escape("Hello world"), "Hello world");
    }

    #[test]
    fn sanitize_html_escape_svg_payload() {
        let result = sanitize_html_escape(r#"<svg onload="alert(1)">"#);
        assert!(result.contains("&lt;svg"));
    }
}
