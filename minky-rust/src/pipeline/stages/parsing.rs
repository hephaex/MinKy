//! Parsing stage - converts raw documents to structured text
//!
//! Handles parsing of various document formats:
//! - Markdown
//! - HTML
//! - Plain text

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// ── Static regex helpers (compiled once) ─────────────────────────────────────

fn tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
}

fn entity_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"&\w+;").unwrap())
}

fn heading_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<h([1-6])[^>]*>(.*?)</h[1-6]>").unwrap())
}

fn link_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?is)<a\s[^>]*?href=(?:"([^"]*?)"|'([^']*?)')(?:[^>]*)>(.*?)</a>"#).unwrap()
    })
}

fn script_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap())
}

fn style_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap())
}

fn block_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"</(p|div|h[1-6]|br|li|tr)>").unwrap())
}

fn whitespace_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").unwrap())
}

fn title_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<title>([^<]+)</title>").unwrap())
}

use crate::pipeline::{PipelineContext, PipelineResult, PipelineStage};

use super::ingestion::RawDocument;

/// Parsed document with extracted structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDocument {
    /// Document title
    pub title: String,

    /// Plain text content (stripped of markup)
    pub plain_text: String,

    /// Original content (preserved for storage)
    pub original_content: String,

    /// MIME type of the original document
    pub mime_type: String,

    /// Extracted headings
    pub headings: Vec<Heading>,

    /// Extracted links
    pub links: Vec<Link>,

    /// Extracted code blocks
    pub code_blocks: Vec<CodeBlock>,

    /// Source information
    pub source_type: String,
    pub source_path: Option<String>,
}

/// A heading extracted from the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heading {
    /// Heading level (1-6)
    pub level: u8,

    /// Heading text
    pub text: String,

    /// Position in plain text (character offset)
    pub position: usize,
}

/// A link extracted from the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Link text/title
    pub text: String,

    /// Link URL
    pub url: String,
}

/// A code block extracted from the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Programming language (if specified)
    pub language: Option<String>,

    /// Code content
    pub code: String,

    /// Start position in original content
    pub start_position: usize,
}

/// Parsing stage - converts various document formats to structured text
#[derive(Debug, Clone, Default)]
pub struct ParsingStage;

impl ParsingStage {
    /// Create a new parsing stage
    pub fn new() -> Self {
        Self
    }

    /// Parse Markdown content
    fn parse_markdown(&self, raw: &RawDocument) -> PipelineResult<ParsedDocument> {
        let mut plain_text = String::new();
        let mut headings = Vec::new();
        let mut links = Vec::new();
        let mut code_blocks = Vec::new();

        // Use pulldown-cmark for proper Markdown parsing
        use pulldown_cmark::{Event, Parser, Tag};

        let parser = Parser::new(&raw.content);

        let mut current_heading_level: Option<u8> = None;
        let mut current_heading_text = String::new();
        let mut current_code_language: Option<String> = None;
        let mut current_code = String::new();
        let mut in_code_block = false;
        let mut link_text = String::new();
        let mut link_url = String::new();
        let mut in_link = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading(level, ..)) => {
                    current_heading_level = Some(level as u8);
                    current_heading_text.clear();
                }
                Event::End(Tag::Heading(..)) => {
                    if let Some(level) = current_heading_level {
                        headings.push(Heading {
                            level,
                            text: current_heading_text.clone(),
                            position: plain_text.len(),
                        });
                        plain_text.push_str(&current_heading_text);
                        plain_text.push('\n');
                    }
                    current_heading_level = None;
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    current_code.clear();
                    current_code_language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.to_string();
                            if lang_str.is_empty() {
                                None
                            } else {
                                Some(lang_str)
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                }
                Event::End(Tag::CodeBlock(_)) => {
                    code_blocks.push(CodeBlock {
                        language: current_code_language.take(),
                        code: current_code.clone(),
                        start_position: plain_text.len(),
                    });
                    plain_text.push_str(&current_code);
                    plain_text.push('\n');
                    in_code_block = false;
                }
                Event::Start(Tag::Link(_, dest, _)) => {
                    in_link = true;
                    link_url = dest.to_string();
                    link_text.clear();
                }
                Event::End(Tag::Link(_, _, _)) => {
                    links.push(Link {
                        text: link_text.clone(),
                        url: link_url.clone(),
                    });
                    plain_text.push_str(&link_text);
                    in_link = false;
                }
                Event::Text(text) => {
                    let text_str = text.to_string();
                    if in_code_block {
                        current_code.push_str(&text_str);
                    } else if current_heading_level.is_some() {
                        current_heading_text.push_str(&text_str);
                    } else if in_link {
                        link_text.push_str(&text_str);
                    } else {
                        plain_text.push_str(&text_str);
                    }
                }
                Event::Code(code) => {
                    plain_text.push_str(&code);
                }
                Event::SoftBreak | Event::HardBreak => {
                    plain_text.push('\n');
                }
                _ => {}
            }
        }

        Ok(ParsedDocument {
            title: raw.title.clone(),
            plain_text: plain_text.trim().to_string(),
            original_content: raw.content.clone(),
            mime_type: raw.mime_type.clone(),
            headings,
            links,
            code_blocks,
            source_type: raw.source_type.clone(),
            source_path: raw.source_path.clone(),
        })
    }

    /// Parse HTML content
    fn parse_html(&self, raw: &RawDocument) -> PipelineResult<ParsedDocument> {
        // Simple HTML stripping using regex
        // For production, consider using scraper crate

        // ── Extract headings before stripping ────────────────────────────────
        // Match <h1>…</h1> through <h6>…</h6>; strip inner tags from text.
        // `position` is the byte offset of the opening tag in `raw.content`
        // (offset into original HTML, not into plain_text).
        let headings: Vec<Heading> = heading_regex()
            .captures_iter(&raw.content)
            .map(|cap| {
                let level = cap[1].parse::<u8>().unwrap_or(1);
                let text = tag_regex().replace_all(&cap[2], "").trim().to_string();
                let position = cap.get(0).map(|m| m.start()).unwrap_or(0);
                Heading { level, text, position }
            })
            .collect();

        // ── Extract links before stripping ───────────────────────────────────
        // Match <a href="…"> or <a href='…'> and capture URL + link text.
        let links: Vec<Link> = link_regex()
            .captures_iter(&raw.content)
            .map(|cap| {
                let url = cap
                    .get(1)
                    .or_else(|| cap.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let text = tag_regex().replace_all(&cap[3], "").trim().to_string();
                Link { text, url }
            })
            .collect();

        let mut plain_text = raw.content.clone();

        // Remove script and style tags with content
        plain_text = script_regex().replace_all(&plain_text, "").to_string();
        plain_text = style_regex().replace_all(&plain_text, "").to_string();

        // Replace block elements with newlines
        plain_text = block_regex().replace_all(&plain_text, "\n").to_string();

        // Remove remaining tags
        plain_text = tag_regex().replace_all(&plain_text, "").to_string();

        // Decode common entities
        plain_text = entity_regex()
            .replace_all(&plain_text, |caps: &regex::Captures| {
                match &caps[0] {
                    "&nbsp;" => " ",
                    "&lt;" => "<",
                    "&gt;" => ">",
                    "&amp;" => "&",
                    "&quot;" => "\"",
                    _ => "",
                }
                .to_string()
            })
            .to_string();

        // Normalize whitespace
        plain_text = whitespace_regex().replace_all(&plain_text, " ").to_string();

        // Extract title from <title> tag if present
        let title = title_regex()
            .captures(&raw.content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| raw.title.clone());

        Ok(ParsedDocument {
            title,
            plain_text: plain_text.trim().to_string(),
            original_content: raw.content.clone(),
            mime_type: raw.mime_type.clone(),
            headings,
            links,
            code_blocks: Vec::new(),
            source_type: raw.source_type.clone(),
            source_path: raw.source_path.clone(),
        })
    }

    /// Parse plain text content (passthrough)
    fn parse_plain_text(&self, raw: &RawDocument) -> PipelineResult<ParsedDocument> {
        Ok(ParsedDocument {
            title: raw.title.clone(),
            plain_text: raw.content.clone(),
            original_content: raw.content.clone(),
            mime_type: raw.mime_type.clone(),
            headings: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            source_type: raw.source_type.clone(),
            source_path: raw.source_path.clone(),
        })
    }
}

#[async_trait]
impl PipelineStage<RawDocument, ParsedDocument> for ParsingStage {
    fn name(&self) -> &'static str {
        "parsing"
    }

    async fn process(
        &self,
        input: RawDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<ParsedDocument> {
        let parsed = match input.mime_type.as_str() {
            "text/markdown" | "text/x-markdown" => self.parse_markdown(&input)?,
            "text/html" | "application/xhtml+xml" => self.parse_html(&input)?,
            _ => {
                // Check file extension for markdown
                if input.source_path.as_ref().is_some_and(|p| {
                    p.ends_with(".md") || p.ends_with(".markdown")
                }) {
                    self.parse_markdown(&input)?
                } else {
                    self.parse_plain_text(&input)?
                }
            }
        };

        // Record extraction stats
        context.set_metadata("headings_count", parsed.headings.len());
        context.set_metadata("links_count", parsed.links.len());
        context.set_metadata("code_blocks_count", parsed.code_blocks.len());

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_raw_doc(content: &str, mime_type: &str) -> RawDocument {
        RawDocument {
            title: "Test".to_string(),
            content: content.to_string(),
            mime_type: mime_type.to_string(),
            source_type: "test".to_string(),
            source_path: None,
        }
    }

    #[tokio::test]
    async fn test_parse_markdown() {
        let stage = ParsingStage::new();
        let mut context = PipelineContext::new();

        let raw = make_raw_doc(
            r#"# Hello

This is a paragraph.

## Section

More text here.

```rust
fn main() {}
```
"#,
            "text/markdown",
        );

        let result = stage.process(raw, &mut context).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert!(parsed.plain_text.contains("Hello"));
        assert!(parsed.plain_text.contains("This is a paragraph"));
        assert_eq!(parsed.headings.len(), 2);
        assert_eq!(parsed.headings[0].level, 1);
        assert_eq!(parsed.headings[0].text, "Hello");
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(parsed.code_blocks[0].language, Some("rust".to_string()));
    }

    #[tokio::test]
    async fn test_parse_html() {
        let stage = ParsingStage::new();
        let mut context = PipelineContext::new();

        let raw = make_raw_doc(
            "<html><head><title>My Page</title></head><body><p>Hello world</p></body></html>",
            "text/html",
        );

        let result = stage.process(raw, &mut context).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.title, "My Page");
        assert!(parsed.plain_text.contains("Hello world"));
        assert!(!parsed.plain_text.contains("<p>"));
    }

    #[tokio::test]
    async fn test_parse_plain_text() {
        let stage = ParsingStage::new();
        let mut context = PipelineContext::new();

        let raw = make_raw_doc("Just plain text here.", "text/plain");

        let result = stage.process(raw, &mut context).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.plain_text, "Just plain text here.");
    }

    #[test]
    fn test_markdown_link_extraction() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("[Example](https://example.com)", "text/markdown");

        let parsed = stage.parse_markdown(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].text, "Example");
        assert_eq!(parsed.links[0].url, "https://example.com");
    }

    // ── S26-01: HTML heading + link extraction ────────────────────────────────

    #[test]
    fn html_heading_extraction_single() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<html><body><h1>Main Title</h1><p>Content</p></body></html>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings.len(), 1);
        assert_eq!(parsed.headings[0].level, 1);
        assert_eq!(parsed.headings[0].text, "Main Title");
    }

    #[test]
    fn html_heading_extraction_multiple_levels() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<h1>Top</h1><h2>Sub</h2><h3>SubSub</h3>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings.len(), 3);
        assert_eq!(parsed.headings[0].level, 1);
        assert_eq!(parsed.headings[0].text, "Top");
        assert_eq!(parsed.headings[1].level, 2);
        assert_eq!(parsed.headings[1].text, "Sub");
        assert_eq!(parsed.headings[2].level, 3);
        assert_eq!(parsed.headings[2].text, "SubSub");
    }

    #[test]
    fn html_heading_strips_inner_tags() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h2><em>Styled</em> Heading</h2>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings.len(), 1);
        assert_eq!(parsed.headings[0].text, "Styled Heading");
    }

    #[test]
    fn html_no_headings_returns_empty() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<p>Just a paragraph.</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(parsed.headings.is_empty());
    }

    #[test]
    fn html_link_extraction_double_quotes() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href="https://example.com">Example</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].url, "https://example.com");
        assert_eq!(parsed.links[0].text, "Example");
    }

    #[test]
    fn html_link_extraction_single_quotes() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href='https://example.org'>Visit</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].url, "https://example.org");
        assert_eq!(parsed.links[0].text, "Visit");
    }

    #[test]
    fn html_link_extraction_multiple_links() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href="https://a.com">A</a> and <a href="https://b.com">B</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 2);
        assert_eq!(parsed.links[0].url, "https://a.com");
        assert_eq!(parsed.links[1].url, "https://b.com");
    }

    #[test]
    fn html_link_strips_inner_tags_from_text() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href="https://example.com"><strong>Bold</strong> link</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].text, "Bold link");
    }

    #[test]
    fn html_no_links_returns_empty() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<p>No links here.</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(parsed.links.is_empty());
    }

    // ── M6: flag-exercising tests ─────────────────────────────────────────────

    /// Headings that span multiple lines must be found — exercises the `s`
    /// (dotall) flag, which makes `.` match newlines.
    #[test]
    fn html_heading_extraction_multiline() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>\n  Multi\n  Line\n</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings.len(),
            1,
            "dotall flag must match headings spanning newlines"
        );
        assert_eq!(parsed.headings[0].level, 1);
    }

    /// Uppercase `<H1>` tags must be matched — exercises the `i`
    /// (case-insensitive) flag.
    #[test]
    fn html_heading_extraction_uppercase_tag() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<H1>Upper</H1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings.len(),
            1,
            "case-insensitive flag must match uppercase heading tags"
        );
        assert_eq!(parsed.headings[0].text, "Upper");
    }

    /// Heading `position` must be the byte offset of the opening `<h…>` tag
    /// within `raw.content`, not zero.
    #[test]
    fn html_heading_position_is_byte_offset() {
        let stage = ParsingStage::new();
        // "<p>intro</p>" is 12 bytes before the heading.
        let raw = make_raw_doc("<p>intro</p><h1>Title</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings.len(), 1);
        assert_eq!(
            parsed.headings[0].position,
            12,
            "position must be byte offset of <h1> in raw.content"
        );
    }
}
