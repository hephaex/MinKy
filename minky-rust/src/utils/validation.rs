use validator::Validate;

use crate::error::{AppError, AppResult};

/// Validate a struct and return AppError on failure
pub fn validate_request<T: Validate>(data: &T) -> AppResult<()> {
    data.validate().map_err(|e| {
        let messages: Vec<String> = e
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |error| {
                    format!(
                        "{}: {}",
                        field,
                        error.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| "invalid".to_string())
                    )
                })
            })
            .collect();
        AppError::Validation(messages.join(", "))
    })
}

/// Sanitize HTML content to prevent XSS
pub fn sanitize_html(input: &str) -> String {
    // Basic HTML entity encoding for XSS prevention
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Validate and sanitize a title
pub fn sanitize_title(title: &str) -> String {
    let trimmed = title.trim();
    // Remove any control characters
    trimmed
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_html() {
        assert_eq!(
            sanitize_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_sanitize_title() {
        assert_eq!(sanitize_title("  Hello World  "), "Hello World");
        assert_eq!(sanitize_title("Test\x00Title"), "TestTitle");
    }
}
