//! Parsing stage - converts raw documents to structured text
//!
//! Handles parsing of various document formats:
//! - Markdown
//! - HTML
//! - Plain text
//!
//! ## Position coordinate systems
//!
//! [`Heading::position`] and [`CodeBlock::start_position`] use **different**
//! coordinate systems depending on the source format:
//! - **HTML**: byte offset of the opening tag (`<h1>`, `<pre>`) within `raw.content`
//! - **Markdown**: character offset into the rendered `plain_text` at the point
//!   the element is emitted by pulldown-cmark
//!
//! Consumers that need a cross-format offset must re-derive position from the
//! canonical `plain_text` field rather than relying on this value directly.

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
    // Matches named entities (&amp;), decimal numeric (&#39;), and hex numeric
    // (&#x27; or &#X27;).  Order is significant: hex (#[xX]...) must precede
    // decimal (#[0-9]+) in the alternation — \w cannot match '#' so named
    // entities never conflict.  Length bounds prevent pathological input:
    // hex ≤8 digits (covers full u32), decimal ≤10, named ≤32 chars.
    RE.get_or_init(|| {
        Regex::new(r"&(?:#[xX][0-9a-fA-F]{1,8}|#[0-9]{1,10}|\w{1,32});").unwrap()
    })
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
    // \p{White_Space} covers all Unicode whitespace, including U+00A0 (NBSP)
    // decoded from &nbsp;, ensuring body normalization is consistent whether
    // content used ASCII spaces or non-breaking spaces in the source HTML.
    RE.get_or_init(|| Regex::new(r"\p{White_Space}+").unwrap())
}

fn title_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<title>([^<]+)</title>").unwrap())
}

fn pre_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?is)<pre[^>]*>(.*?)</pre>").unwrap())
}

fn code_tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // No leading `^` or trailing `$` anchors.
    //   - No `$`: the lazy `(.*?)` stops at the FIRST `</code>` — adding `$`
    //     forces it to extend to the last `</code>` (merges siblings).
    //   - No `^`: allows `captures_iter` to find every `<code>` sibling inside
    //     a `<pre>` block, not just the one anchored at the start.
    RE.get_or_init(|| Regex::new(r"(?is)<code([^>]*)>(.*?)</code>").unwrap())
}

fn code_language_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Anchored to a class attribute so data-href or id values that happen to
    // contain "language-" or "lang-" are not misidentified as language tokens.
    // Accepts both `language-xxx` (HTML5/GitHub) and `lang-xxx` (Prettify/legacy).
    RE.get_or_init(|| {
        Regex::new(r#"(?i)class\s*=\s*["'][^"']*?(?:language-|lang-)([^\s"']+)"#).unwrap()
    })
}

/// Decode HTML entities in a string.
///
/// Handles named entities (`&amp;`, `&rsquo;`, etc.), decimal numeric
/// entities (`&#39;`), and hex numeric entities (`&#x27;`).  Unknown named
/// entities are preserved verbatim so no content is silently deleted.
///
/// Shared by all extraction paths (heading, link, code-block, plain-text body)
/// to ensure consistent decoding across formats.
fn decode_html_entities(s: &str) -> String {
    entity_regex()
        .replace_all(s, |caps: &regex::Captures| {
            match &caps[0] {
                // Basic HTML (RFC 1866 / HTML4)
                "&nbsp;" => "\u{00A0}".to_string(),
                "&lt;" => "<".to_string(),
                "&gt;" => ">".to_string(),
                "&amp;" => "&".to_string(),
                "&quot;" => "\"".to_string(),
                "&apos;" => "'".to_string(),
                // Quotation marks
                "&lsquo;" => "\u{2018}".to_string(),
                "&rsquo;" => "\u{2019}".to_string(),
                "&ldquo;" => "\u{201C}".to_string(),
                "&rdquo;" => "\u{201D}".to_string(),
                "&laquo;" => "\u{00AB}".to_string(),
                "&raquo;" => "\u{00BB}".to_string(),
                // Dashes and ellipsis
                "&mdash;" => "\u{2014}".to_string(),
                "&ndash;" => "\u{2013}".to_string(),
                "&minus;" => "\u{2212}".to_string(),
                "&hellip;" => "\u{2026}".to_string(),
                // Symbols
                "&copy;" => "\u{00A9}".to_string(),
                "&reg;" => "\u{00AE}".to_string(),
                "&trade;" => "\u{2122}".to_string(),
                "&euro;" => "\u{20AC}".to_string(),
                "&pound;" => "\u{00A3}".to_string(),
                "&yen;" => "\u{00A5}".to_string(),
                "&cent;" => "\u{00A2}".to_string(),
                // Punctuation
                "&bull;" => "\u{2022}".to_string(),
                "&middot;" => "\u{00B7}".to_string(),
                // Math / typography
                "&times;" => "\u{00D7}".to_string(),
                "&divide;" => "\u{00F7}".to_string(),
                "&plusmn;" => "\u{00B1}".to_string(),
                "&deg;" => "\u{00B0}".to_string(),
                "&frac12;" => "\u{00BD}".to_string(),
                "&frac14;" => "\u{00BC}".to_string(),
                "&frac34;" => "\u{00BE}".to_string(),
                // Hex numeric entities: &#xNN; or &#XNN;
                other if other.starts_with("&#x") || other.starts_with("&#X") => {
                    let hex = &other[3..other.len() - 1];
                    u32::from_str_radix(hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| other.to_string())
                }
                // Decimal numeric entities: &#NN;
                other if other.starts_with("&#") => {
                    let dec = &other[2..other.len() - 1];
                    dec.parse::<u32>()
                        .ok()
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| other.to_string())
                }
                // Preserve unknown named entities verbatim
                other => other.to_string(),
            }
        })
        .to_string()
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

    /// Byte offset of the opening `<h…>` tag in `raw.content` (HTML), or
    /// character offset into `plain_text` at event-emission time (Markdown).
    /// See module-level docs for coordinate-system details.
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

    /// Byte offset of this code block in the source.
    ///
    /// For Markdown: offset into `plain_text` at the time the block is
    /// emitted (character count of preceding rendered text).
    /// For HTML: byte offset of the opening `<pre>` tag in `raw.content`.
    ///
    /// Note: these are different coordinate systems. Consumers that need
    /// cross-format consistency should re-derive position from the canonical
    /// plain-text offset rather than relying on this field directly.
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
                let stripped = tag_regex().replace_all(&cap[2], " ");
                let collapsed = whitespace_regex().replace_all(stripped.as_ref(), " ");
                let text = decode_html_entities(collapsed.trim());
                let position = cap.get(0).map(|m| m.start()).unwrap_or(0);
                Heading { level, text, position }
            })
            .collect();

        // ── Extract code blocks before stripping ─────────────────────────────
        // Match <pre>…</pre>; each <code> sibling inside a <pre> becomes its
        // own CodeBlock.  Multiple siblings (e.g. output vs input panes) are
        // all extracted rather than only the first.  Bare <pre> blocks with no
        // <code> tag fall back to tag-stripped, language=None.
        let code_blocks: Vec<CodeBlock> = pre_regex()
            .captures_iter(&raw.content)
            .flat_map(|cap| {
                let start_position = cap.get(0).map(|m| m.start()).unwrap_or(0);
                let inner = cap[1].trim();
                // Collect all <code> siblings inside this <pre>.
                let siblings: Vec<CodeBlock> = code_tag_regex()
                    .captures_iter(inner)
                    .map(|cc| {
                        let language = code_language_regex()
                            .captures(&cc[1])
                            .and_then(|m| m.get(1))
                            .map(|m| m.as_str().to_string());
                        let code = decode_html_entities(&cc[2]);
                        CodeBlock { language, code, start_position }
                    })
                    .collect();
                if siblings.is_empty() {
                    // Bare <pre> without any <code> tag — strip residual HTML.
                    let stripped = tag_regex().replace_all(inner, "");
                    let code = decode_html_entities(&stripped);
                    vec![CodeBlock { language: None, code, start_position }]
                } else {
                    siblings
                }
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
                let stripped = tag_regex().replace_all(&cap[3], " ");
                let collapsed = whitespace_regex().replace_all(stripped.as_ref(), " ");
                let text = decode_html_entities(collapsed.trim());
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
        plain_text = decode_html_entities(&plain_text);

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
            code_blocks,
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

    // ── S27-01: heading/link text normalization (M3 + M4) ────────────────────

    /// Inner tags separated by no whitespace must produce a space between
    /// their text: `<h2>foo<b>bar</b></h2>` → "foo bar", not "foobar".
    #[test]
    fn html_heading_inner_tags_produce_space() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h2>foo<b>bar</b></h2>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "foo bar",
            "adjacent inner tags must be separated by a space"
        );
    }

    /// HTML entities inside heading text must be decoded:
    /// `<h1>AT&amp;T</h1>` → "AT&T".
    #[test]
    fn html_heading_entity_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>AT&amp;T &lt;Corp&gt;</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "AT&T <Corp>",
            "HTML entities in heading text must be decoded"
        );
    }

    /// Combined: inner tag with entity in heading text.
    #[test]
    fn html_heading_inner_tag_and_entity_combined() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1><em>Foo</em> &amp; Bar</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings[0].text, "Foo & Bar");
    }

    /// Inner tags in link text must produce a space (same fix as headings).
    #[test]
    fn html_link_inner_tags_produce_space() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href="https://example.com">foo<b>bar</b></a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.links[0].text,
            "foo bar",
            "adjacent inner tags in link text must be separated by a space"
        );
    }

    /// HTML entities inside link text must be decoded.
    #[test]
    fn html_link_entity_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<a href="https://example.com">Q&amp;A Guide</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.links[0].text,
            "Q&A Guide",
            "HTML entities in link text must be decoded"
        );
    }

    // ── S27-02: HTML code_blocks extraction ───────────────────────────────────

    #[test]
    fn html_code_block_with_language() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<pre><code class="language-rust">fn main() {}</code></pre>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(
            parsed.code_blocks[0].language,
            Some("rust".to_string())
        );
        assert!(parsed.code_blocks[0].code.contains("fn main()"));
    }

    #[test]
    fn html_code_block_without_language() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre><code>plain code here</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(parsed.code_blocks[0].language, None);
        assert!(parsed.code_blocks[0].code.contains("plain code here"));
    }

    #[test]
    fn html_code_block_bare_pre() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre>bare preformatted text</pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(parsed.code_blocks[0].language, None);
        assert!(parsed.code_blocks[0].code.contains("bare preformatted text"));
    }

    #[test]
    fn html_code_block_multiple_blocks() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<pre><code class="language-python">x = 1</code></pre>
               <pre><code class="language-rust">let x = 1;</code></pre>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 2);
        assert_eq!(
            parsed.code_blocks[0].language,
            Some("python".to_string())
        );
        assert_eq!(
            parsed.code_blocks[1].language,
            Some("rust".to_string())
        );
    }

    #[test]
    fn html_no_code_blocks_returns_empty() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<p>No code here.</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(parsed.code_blocks.is_empty());
    }

    #[test]
    fn html_code_block_entity_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre><code>a &lt; b &amp;&amp; c &gt; 0</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(parsed.code_blocks[0].code, "a < b && c > 0");
    }

    // ── Review fixes (C1, C2, C3, H5) ────────────────────────────────────────

    /// C1 regression: truly unknown entities must be preserved verbatim, not dropped.
    #[test]
    fn html_heading_unknown_entity_preserved() {
        let stage = ParsingStage::new();
        // &fakeentity; is not in any decode table and must survive verbatim.
        let raw = make_raw_doc("<h1>Hello &fakeentity; World</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(
            parsed.headings[0].text.contains("&fakeentity;"),
            "unknown entity must be kept verbatim: got {:?}",
            parsed.headings[0].text
        );
    }

    /// C1: &apos; is now decoded to '.
    #[test]
    fn html_heading_apos_entity_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>It&apos;s fine</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings[0].text, "It's fine");
    }

    /// C2 / S28-02: two <code> siblings inside one <pre> must each become a
    /// separate CodeBlock (not merged, not dropped).
    #[test]
    fn html_code_block_two_code_siblings_in_one_pre() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre><code class=\"language-rust\">fn a() {}</code>\n<code class=\"language-rust\">fn b() {}</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.code_blocks.len(),
            2,
            "each <code> sibling must become a separate CodeBlock"
        );
        assert!(parsed.code_blocks[0].code.contains("fn a()"));
        assert!(parsed.code_blocks[1].code.contains("fn b()"));
        for block in &parsed.code_blocks {
            assert!(
                !block.code.contains("</code>"),
                "code block must not contain a literal </code>: {:?}",
                block.code
            );
        }
    }

    /// M2: <pre> with attributes must still be matched and language extracted.
    #[test]
    fn html_code_block_pre_with_attributes() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<pre class="highlight" data-lang="ignored"><code class="language-go">package main</code></pre>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(
            parsed.code_blocks[0].language,
            Some("go".to_string())
        );
    }

    // ── S28-01: expanded entity decoding ─────────────────────────────────────

    /// &rsquo; (right single quotation mark) is now in the decode table.
    #[test]
    fn html_heading_rsquo_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>What&rsquo;s new</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "What\u{2019}s new",
            "&rsquo; must decode to U+2019 right single quotation mark"
        );
    }

    /// &mdash; (em dash) is now in the decode table.
    #[test]
    fn html_heading_mdash_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>foo&mdash;bar</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "foo\u{2014}bar",
            "&mdash; must decode to U+2014 em dash"
        );
    }

    /// &#39; (decimal numeric apostrophe) must be decoded.
    #[test]
    fn html_heading_numeric_decimal_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>&#39;hello&#39;</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "'hello'",
            "decimal numeric entity &#39; must decode to apostrophe"
        );
    }

    /// &#x27; (hex numeric apostrophe) must be decoded.
    #[test]
    fn html_heading_numeric_hex_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>&#x27;world&#x27;</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "'world'",
            "hex numeric entity &#x27; must decode to apostrophe"
        );
    }

    /// &hellip; (ellipsis) and &copy; (copyright) are now decoded.
    #[test]
    fn html_heading_hellip_copy_decoded() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>Read more&hellip; &copy;2024</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "Read more\u{2026} \u{00A9}2024",
            "&hellip; and &copy; must decode correctly"
        );
    }

    // ── S28-02: multi-sibling <code> extraction ───────────────────────────────

    /// Three <code> siblings in one <pre> must each become a separate CodeBlock.
    #[test]
    fn html_code_block_three_code_siblings() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<pre><code class="language-python">x = 1</code><code class="language-rust">let x = 1;</code><code>plain</code></pre>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 3);
        assert_eq!(parsed.code_blocks[0].language, Some("python".to_string()));
        assert_eq!(parsed.code_blocks[1].language, Some("rust".to_string()));
        assert_eq!(parsed.code_blocks[2].language, None);
        assert!(parsed.code_blocks[0].code.contains("x = 1"));
        assert!(parsed.code_blocks[1].code.contains("let x = 1;"));
        assert!(parsed.code_blocks[2].code.contains("plain"));
    }

    /// All siblings share the same start_position (byte offset of their parent <pre>).
    #[test]
    fn html_code_block_siblings_share_start_position() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<pre><code>a</code><code>b</code></pre>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 2);
        assert_eq!(
            parsed.code_blocks[0].start_position,
            parsed.code_blocks[1].start_position,
            "siblings in the same <pre> must share start_position"
        );
        assert_eq!(parsed.code_blocks[0].start_position, 0);
    }

    /// L2: siblings' start_position is the byte offset of the parent <pre>, not 0.
    #[test]
    fn html_code_block_siblings_position_is_pre_offset() {
        let stage = ParsingStage::new();
        // "<p>x</p>" is 8 bytes; <pre> begins at offset 8.
        let raw = make_raw_doc(
            "<p>x</p><pre><code>a</code><code>b</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 2);
        assert_eq!(
            parsed.code_blocks[0].start_position,
            8,
            "start_position must be the byte offset of the opening <pre> tag"
        );
        assert_eq!(parsed.code_blocks[1].start_position, 8);
    }

    // ── L1: invalid numeric entity fallback ───────────────────────────────────

    /// Surrogate code point (U+D800) is invalid for char::from_u32 — must be
    /// preserved verbatim, not silently deleted.
    #[test]
    fn html_invalid_hex_entity_surrogate_preserved() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>x&#xD800;y</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(
            parsed.headings[0].text.contains("&#xD800;"),
            "surrogate hex entity must be preserved verbatim: got {:?}",
            parsed.headings[0].text
        );
    }

    /// &nbsp; decoded to U+00A0 must be collapsed to a plain space by
    /// whitespace_regex in the plain_text body path.
    #[test]
    fn html_nbsp_collapses_in_plain_text() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<p>foo&nbsp;bar</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.plain_text,
            "foo bar",
            "&nbsp; must collapse to a single space in plain_text"
        );
    }
}
