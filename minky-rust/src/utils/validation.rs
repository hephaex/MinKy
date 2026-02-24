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
    fn test_sanitize_html_script_tag() {
        assert_eq!(
            sanitize_html("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_sanitize_html_ampersand() {
        assert_eq!(sanitize_html("rock & roll"), "rock &amp; roll");
    }

    #[test]
    fn test_sanitize_html_double_quotes() {
        assert_eq!(sanitize_html("say \"hello\""), "say &quot;hello&quot;");
    }

    #[test]
    fn test_sanitize_html_no_special_chars() {
        let plain = "Hello, World!";
        assert_eq!(sanitize_html(plain), plain);
    }

    #[test]
    fn test_sanitize_html_empty_string() {
        assert_eq!(sanitize_html(""), "");
    }

    #[test]
    fn test_sanitize_title_trims_whitespace() {
        assert_eq!(sanitize_title("  Hello World  "), "Hello World");
    }

    #[test]
    fn test_sanitize_title_removes_null_byte() {
        assert_eq!(sanitize_title("Test\x00Title"), "TestTitle");
    }

    #[test]
    fn test_sanitize_title_allows_newline() {
        // \n is explicitly allowed in the filter
        let result = sanitize_title("Line1\nLine2");
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_sanitize_title_allows_tab() {
        // \t is explicitly allowed in the filter
        let result = sanitize_title("Col1\tCol2");
        assert!(result.contains('\t'));
    }

    #[test]
    fn test_sanitize_title_empty() {
        assert_eq!(sanitize_title(""), "");
    }

    #[test]
    fn test_sanitize_html_single_quote() {
        assert_eq!(sanitize_html("it's"), "it&#x27;s");
    }

    #[test]
    fn test_sanitize_html_multiple_special_chars() {
        let input = "<b>Tom & Jerry</b>";
        let output = sanitize_html(input);
        assert!(output.contains("&lt;b&gt;"));
        assert!(output.contains("&amp;"));
        assert!(output.contains("&lt;/b&gt;"));
    }

    #[test]
    fn test_sanitize_html_preserves_normal_text() {
        let plain = "Hello World 123";
        assert_eq!(sanitize_html(plain), plain);
    }

    #[test]
    fn test_sanitize_title_preserves_unicode() {
        let title = "한국어 타이틀";
        assert_eq!(sanitize_title(title), title);
    }

    #[test]
    fn test_sanitize_title_removes_bell_char() {
        // \x07 is BEL (control character) and should be removed
        let title = "Title\x07Name";
        assert_eq!(sanitize_title(title), "TitleName");
    }

    #[test]
    fn test_sanitize_html_greater_than_sign() {
        assert_eq!(sanitize_html("5 > 3"), "5 &gt; 3");
    }

    #[test]
    fn test_sanitize_html_less_than_sign() {
        assert_eq!(sanitize_html("3 < 5"), "3 &lt; 5");
    }

    // Additional validate_request tests using a test struct
    #[derive(Debug, Validate)]
    struct TestValidation {
        #[validate(length(min = 1, max = 10))]
        name: String,
        #[validate(range(min = 0, max = 100))]
        age: i32,
    }

    #[test]
    fn test_validate_request_valid_data() {
        let data = TestValidation {
            name: "Alice".to_string(),
            age: 25,
        };
        let result = validate_request(&data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_request_empty_name_fails() {
        let data = TestValidation {
            name: "".to_string(),
            age: 25,
        };
        let result = validate_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_name_too_long_fails() {
        let data = TestValidation {
            name: "ThisNameIsTooLong".to_string(),
            age: 25,
        };
        let result = validate_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_negative_age_fails() {
        let data = TestValidation {
            name: "Bob".to_string(),
            age: -5,
        };
        let result = validate_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_age_too_high_fails() {
        let data = TestValidation {
            name: "Bob".to_string(),
            age: 150,
        };
        let result = validate_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_request_boundary_values_ok() {
        let data = TestValidation {
            name: "A".to_string(), // min length
            age: 0,               // min value
        };
        assert!(validate_request(&data).is_ok());

        let data2 = TestValidation {
            name: "1234567890".to_string(), // max length
            age: 100,                       // max value
        };
        assert!(validate_request(&data2).is_ok());
    }

    #[test]
    fn test_validate_request_error_message_format() {
        let data = TestValidation {
            name: "".to_string(),
            age: 25,
        };
        let result = validate_request(&data);
        assert!(result.is_err());
        if let Err(AppError::Validation(msg)) = result {
            assert!(msg.contains("name"));
        } else {
            panic!("Expected AppError::Validation");
        }
    }

    // Additional sanitize edge cases
    #[test]
    fn test_sanitize_html_nested_tags() {
        let input = "<div><p>Hello</p></div>";
        let output = sanitize_html(input);
        assert_eq!(output, "&lt;div&gt;&lt;p&gt;Hello&lt;/p&gt;&lt;/div&gt;");
    }

    #[test]
    fn test_sanitize_html_attribute_injection() {
        let input = "onload=\"alert('xss')\"";
        let output = sanitize_html(input);
        assert!(output.contains("&quot;"));
        assert!(!output.contains("\"alert"));
    }

    #[test]
    fn test_sanitize_html_url_encoded_already() {
        // Already encoded entities should be double-encoded
        let input = "&lt;script&gt;";
        let output = sanitize_html(input);
        assert_eq!(output, "&amp;lt;script&amp;gt;");
    }

    #[test]
    fn test_sanitize_html_mixed_content() {
        let input = "Hello <b>World</b> & 'everyone'";
        let output = sanitize_html(input);
        assert!(output.contains("&lt;b&gt;"));
        assert!(output.contains("&amp;"));
        assert!(output.contains("&#x27;"));
    }

    #[test]
    fn test_sanitize_title_only_whitespace() {
        let title = "   \t   ";
        assert_eq!(sanitize_title(title), "");
    }

    #[test]
    fn test_sanitize_title_carriage_return_removed() {
        // \r is a control character not in the allow list
        let title = "Line1\r\nLine2";
        let result = sanitize_title(title);
        // \r should be removed, \n should remain
        assert!(!result.contains('\r'));
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_sanitize_title_escape_sequence() {
        // ESC (0x1B) should be removed
        let title = "Test\x1B[31mRed";
        let result = sanitize_title(title);
        assert!(!result.contains('\x1B'));
    }

    #[test]
    fn test_sanitize_title_form_feed_removed() {
        // Form feed (0x0C) is control character
        let title = "Page1\x0CPage2";
        let result = sanitize_title(title);
        assert!(!result.contains('\x0C'));
        assert_eq!(result, "Page1Page2");
    }

    #[test]
    fn test_sanitize_html_emoji_preserved() {
        let input = "Hello 👋 World 🌍";
        assert_eq!(sanitize_html(input), input);
    }

    #[test]
    fn test_sanitize_title_emoji_preserved() {
        let title = "📝 My Document";
        assert_eq!(sanitize_title(title), title);
    }
}
