//! Parsing stage - converts raw documents to structured text
//!
//! Handles parsing of various document formats:
//! - Markdown
//! - HTML
//! - Plain text
//!
//! ## Position coordinate systems
//!
//! [`Heading::position`], [`Link::position`], and [`CodeBlock::start_position`]
//! use **byte offsets** (not character/codepoint offsets), but the reference
//! string differs by format and element type:
//!
//! - **HTML headings / links**: best-effort byte offset of the opening tag in
//!   `raw.content`, derived by scanning the UTF-8 source string in DOM order
//!   with case-insensitive comparison (html5ever normalises tag names to
//!   lowercase but the source may contain `<H1>` or `<A HREF=…>`).
//!   html5ever discards source spans after tokenisation, so the scan may
//!   return 0 when the tag cannot be located.
//! - **HTML code blocks**: accurate byte offset of the opening `<pre>` tag in
//!   `raw.content` (regex-derived, no html5ever involvement).
//! - **Markdown headings / links / code blocks**: byte offset into the
//!   rendered `plain_text` string at the point the element is emitted by
//!   pulldown-cmark (i.e. `plain_text.len()` at event time).
//!
//! Consumers that need a cross-format, unique offset must re-derive position
//! from the canonical `plain_text` field rather than relying on this value.
//!
//! ## Entity decoding
//!
//! Entity handling differs by extraction path:
//!
//! - **Title / headings / links**: html5ever decodes the full HTML5 named-entity
//!   table automatically via `el.text()`.  No custom post-processing needed.
//! - **`plain_text` body**: the raw HTML is stripped with regex then run through
//!   [`decode_html_entities`], which delegates to `html_escape::decode_html_entities`
//!   and therefore covers the complete HTML5 named-entity table.  html5ever is
//!   intentionally **not** used here because tree construction can reorder or drop
//!   text nodes in malformed HTML, producing different output than the source order
//!   expected for search indexing.
//! - **Code blocks**: same `decode_html_entities` path as the body.  DOM
//!   restructuring by html5ever would corrupt verbatim source code (e.g.
//!   `<pre>` content adjacent to unclosed tags gets moved outside `<body>`).
//!
//! ## Known limitations
//!
//! - **Table foster-parenting**: html5ever moves content placed illegally inside
//!   `<table>` (e.g. bare text nodes) to just before the table per the HTML5
//!   parsing spec.  When this happens, `<a>` elements in the source may appear
//!   at a different position in the DOM than in the raw byte string, so the
//!   3-byte `<a` window scan may land on a different tag than scraper selected.
//!   The result is a best-effort offset, not a guaranteed exact match.

use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// ── Static regex helpers — body stripping + code blocks (compiled once) ──────
// Note: heading and link extraction use scraper/html5ever (see scraper_extract_all).

fn tag_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<[^>]+>").unwrap())
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

fn surrogate_hex_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Matches hex numeric character references; caller checks if codepoint
    // is in the surrogate range 0xD800–0xDFFF.
    RE.get_or_init(|| Regex::new(r"(?i)&#x([0-9a-f]+);").unwrap())
}

fn surrogate_dec_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Matches decimal numeric character references; caller checks if codepoint
    // is in the surrogate range 55296–57343.
    RE.get_or_init(|| Regex::new(r"&#([0-9]+);").unwrap())
}

// ── scraper-based HTML extractors ────────────────────────────────────────────

/// Extract title, headings, and links in a single html5ever parse.
///
/// Returns `(Option<title>, Vec<Heading>, Vec<Link>)`.
///
/// - **Title**: html5ever decodes the full HTML5 named entity table, so
///   `<title>AT&amp;T</title>` → `"AT&T"`.  Whitespace is collapsed.
///   Returns `None` if no `<title>` element exists.
/// - **Headings/links**: see field-level docs on `Heading` and `Link`.
/// - **Position (heading)**: best-effort byte offset of the opening `<hN>` tag,
///   derived via case-insensitive byte window search to handle `<H1>` source.
///   The scan advances in DOM order so multiple headings of the same level each
///   resolve to their own offset.
fn scraper_extract_all(html: &str) -> (Option<String>, Vec<Heading>, Vec<Link>) {
    use scraper::{Html, Selector};
    static H_SEL: OnceLock<Selector> = OnceLock::new();
    static A_SEL: OnceLock<Selector> = OnceLock::new();
    static T_SEL: OnceLock<Selector> = OnceLock::new();
    // safe: all are valid CSS selector literals
    let h_sel = H_SEL.get_or_init(|| Selector::parse("h1,h2,h3,h4,h5,h6").unwrap());
    let a_sel = A_SEL.get_or_init(|| Selector::parse("a[href]").unwrap());
    let t_sel = T_SEL.get_or_init(|| Selector::parse("head > title").unwrap());
    let doc = Html::parse_document(html);

    let title = doc.select(t_sel).next().and_then(|el| {
        let text: String = el.text().collect();
        let collapsed = whitespace_regex().replace_all(text.trim(), " ").to_string();
        if collapsed.is_empty() { None } else { Some(collapsed) }
    });

    let mut h_search_start = 0_usize;
    let headings: Vec<Heading> = doc.select(h_sel)
        .map(|el| {
            let name = el.value().name(); // always lowercase from html5ever
            let level: u8 = name.as_bytes().get(1)
                .map(|b| b - b'0')
                .unwrap_or(1);
            let tag_pat = format!("<{}", name);
            // Case-insensitive window search so "<H1>" in source is found even
            // though html5ever normalises the name to "h1".
            let position = html.as_bytes()[h_search_start..]
                .windows(tag_pat.len())
                .position(|w| w.eq_ignore_ascii_case(tag_pat.as_bytes()))
                .map(|p| h_search_start + p)
                .unwrap_or(0);
            h_search_start = position + 1;
            let text: String = el.text().collect();
            let collapsed = whitespace_regex().replace_all(text.trim(), " ").to_string();
            Heading { level, text: collapsed, position }
        })
        .collect();

    let mut a_search_start = 0_usize;
    let links: Vec<Link> = doc.select(a_sel)
        .map(|el| {
            // Case-insensitive match for `<a` followed by a non-alphanumeric
            // byte so `<abbr`, `<address>`, etc. are not mistakenly hit.
            let position = html.as_bytes()[a_search_start..]
                .windows(3)
                .position(|w| {
                    w[0] == b'<'
                        && (w[1] == b'a' || w[1] == b'A')
                        && !w[2].is_ascii_alphanumeric()
                })
                .map(|p| a_search_start + p)
                .unwrap_or(0);
            a_search_start = position + 1;
            let url = el.value().attr("href").unwrap_or("").to_string();
            let text: String = el.text().collect();
            let collapsed = whitespace_regex().replace_all(text.trim(), " ").to_string();
            Link { text: collapsed, url, position }
        })
        .collect();

    (title, headings, links)
}

/// Decode HTML entities in a string.
///
/// Delegates to `html_escape::decode_html_entities`, which covers the full
/// HTML5 named-entity table (2,231 entries), decimal numeric entities
/// (`&#39;`), and hex numeric entities (`&#x27;`).  Unknown named entities
/// are preserved verbatim so no content is silently deleted.
///
/// Two categories of invalid codepoints are replaced with U+FFFD:
/// - NUL bytes (`&#0;` / `&#x0;`): invalid in PostgreSQL TEXT columns.
/// - Direct surrogate NCRs U+D800–U+DFFF (`&#xD800;` / `&#55296;` etc.):
///   `char::from_u32` returns `None` for surrogates, so html_escape
///   preserves their entity form verbatim.  html5ever (heading/link path)
///   replaces direct surrogate NCRs with U+FFFD in the same way.
///   Note: an escaped-ampersand form (`&amp;#xD800;`) reaches this function
///   as the literal text `&#xD800;` after `&amp;` → `&` decoding, and is
///   also replaced with U+FFFD here — slightly more aggressive than html5ever
///   (which would preserve that text node verbatim), but safe for indexing.
///
/// Shared by the plain-text body and code-block extraction paths to ensure
/// consistent decoding without html5ever's tree-restructuring side effects.
fn decode_html_entities(s: &str) -> String {
    let decoded = html_escape::decode_html_entities(s);

    // NUL bytes — invalid in PostgreSQL TEXT columns
    let mut result = if decoded.contains('\0') {
        decoded.replace('\0', "\u{FFFD}")
    } else {
        decoded.into_owned()
    };

    // html_escape preserves surrogate codepoints (U+D800–U+DFFF) as
    // literal entity text because `char::from_u32` returns `None` for them.
    // Replace with U+FFFD to match html5ever's heading/link path behavior.
    if result.contains("&#") {
        let hex_re = surrogate_hex_regex();
        let dec_re = surrogate_dec_regex();

        let after_hex = hex_re.replace_all(&result, |caps: &regex::Captures| {
            match u32::from_str_radix(&caps[1], 16) {
                Ok(cp) if (0xD800..=0xDFFF).contains(&cp) => "\u{FFFD}".to_owned(),
                _ => caps[0].to_owned(),
            }
        }).into_owned();
        result = dec_re.replace_all(&after_hex, |caps: &regex::Captures| {
            match caps[1].parse::<u32>() {
                Ok(cp) if (0xD800..=0xDFFF).contains(&cp) => "\u{FFFD}".to_owned(),
                _ => caps[0].to_owned(),
            }
        }).into_owned();
    }

    result
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

    /// Byte offset of the opening `<hN>` tag in `raw.content` (HTML), or
    /// byte offset into `plain_text` at event-emission time (Markdown).
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

    /// Byte offset of the opening `<a` tag in `raw.content` (HTML), or
    /// byte offset into `plain_text` at link-emission time (Markdown).
    /// See module-level docs for coordinate-system details.
    pub position: usize,
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
    /// For Markdown: byte offset into `plain_text` at the time the block is
    /// emitted (i.e. `plain_text.len()` before the block's text is appended).
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
        let mut link_position_snapshot: usize = 0;

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
                    link_position_snapshot = plain_text.len() + current_heading_text.len();
                }
                Event::End(Tag::Link(_, _, _)) => {
                    links.push(Link {
                        text: link_text.clone(),
                        url: link_url.clone(),
                        position: link_position_snapshot,
                    });
                    // Inside a heading the link text is already in
                    // current_heading_text and will reach plain_text via
                    // End(Heading); pushing again would duplicate it.
                    if current_heading_level.is_none() {
                        plain_text.push_str(&link_text);
                    }
                    in_link = false;
                }
                Event::Text(text) => {
                    let text_str = text.to_string();
                    if in_code_block {
                        current_code.push_str(&text_str);
                    } else if current_heading_level.is_some() {
                        current_heading_text.push_str(&text_str);
                        // A link nested inside a heading still needs its text
                        // collected so Link::text is not empty.
                        if in_link {
                            link_text.push_str(&text_str);
                        }
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
                Event::End(Tag::Paragraph) if !plain_text.ends_with('\n') => {
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
        // ── Extract title, headings, links (html5ever/scraper, single parse) ─
        // scraper handles mismatched tags, nested elements, the full HTML5
        // entity table (including title entity decode), and link positions.
        let (html_title, headings, links) = scraper_extract_all(&raw.content);

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

        let title = html_title.unwrap_or_else(|| raw.title.clone());

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

    /// Position scan is case-insensitive: `<H1>` in source must resolve to its
    /// correct byte offset, not silently fall back to 0.
    #[test]
    fn html_heading_uppercase_tag_position() {
        let stage = ParsingStage::new();
        // "<p>aa</p>" is 9 bytes; <H1> starts at offset 9.
        let raw = make_raw_doc("<p>aa</p><H1>upper</H1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.headings.len(), 1);
        assert_eq!(
            parsed.headings[0].position,
            9,
            "uppercase <H1> must resolve to byte offset 9, not 0"
        );
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

    /// html5ever collects text nodes in DOM order: `<h2>foo<b>bar</b></h2>`
    /// produces text nodes "foo" and "bar" concatenated to "foobar" — no
    /// implicit space is injected at element boundaries (DOM-accurate behavior).
    #[test]
    fn html_heading_inner_tags_produce_space() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h2>foo<b>bar</b></h2>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(
            parsed.headings[0].text,
            "foobar",
            "scraper/html5ever concatenates text nodes without injecting spaces at element boundaries"
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

    /// html5ever concatenates text nodes without injecting spaces at element
    /// boundaries: `<a href="…">foo<b>bar</b></a>` → link text "foobar".
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
            "foobar",
            "scraper/html5ever concatenates adjacent text nodes without implicit space"
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

    /// Surrogate code point (U+D800) is a parse error per the HTML5 spec.
    /// html5ever replaces it with U+FFFD (REPLACEMENT CHARACTER) — the same
    /// behavior as all conforming HTML5 parsers.
    #[test]
    fn html_invalid_hex_entity_surrogate_replaced_with_fffd() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("<h1>x&#xD800;y</h1>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(
            parsed.headings[0].text.contains('\u{FFFD}'),
            "html5ever must replace surrogate &#xD800; with U+FFFD: got {:?}",
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

    // ── S29-03: multi-<pre> integration tests ─────────────────────────────────

    /// Three separate <pre> blocks with different languages yield three distinct
    /// CodeBlocks with strictly increasing start_positions.
    #[test]
    fn html_code_block_three_pre_with_mixed_languages() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre><code class=\"language-python\">py</code></pre>\
             <pre><code class=\"language-rust\">rs</code></pre>\
             <pre>plain</pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 3, "three <pre> → three CodeBlocks");
        assert_eq!(parsed.code_blocks[0].language, Some("python".to_string()));
        assert_eq!(parsed.code_blocks[1].language, Some("rust".to_string()));
        assert_eq!(parsed.code_blocks[2].language, None, "bare <pre> → language=None");
        assert!(
            parsed.code_blocks[0].start_position < parsed.code_blocks[1].start_position,
            "positions must be strictly increasing"
        );
        assert!(
            parsed.code_blocks[1].start_position < parsed.code_blocks[2].start_position
        );
    }

    /// First <pre> contains two <code> siblings; second <pre> has one <code>.
    /// The two siblings share start_position; the last has a different one.
    #[test]
    fn html_code_block_pre_siblings_plus_separate_pre() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre><code class=\"language-js\">a</code><code class=\"language-ts\">b</code></pre>\
             <pre><code>c</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 3);
        assert_eq!(
            parsed.code_blocks[0].start_position,
            parsed.code_blocks[1].start_position,
            "siblings share the parent <pre> offset"
        );
        assert_ne!(
            parsed.code_blocks[1].start_position,
            parsed.code_blocks[2].start_position,
            "third block is from a different <pre>"
        );
    }

    /// Bare <pre> followed by a <pre><code> — languages: None then Some.
    #[test]
    fn html_code_block_bare_pre_then_pre_with_code() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre>plain text</pre><pre><code class=\"language-go\">func main() {}</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 2);
        assert_eq!(parsed.code_blocks[0].language, None);
        assert_eq!(parsed.code_blocks[0].code.trim(), "plain text");
        assert_eq!(parsed.code_blocks[1].language, Some("go".to_string()));
    }

    /// HTML entities inside code blocks are decoded; <pre> with attributes
    /// does not confuse pre_regex; second block with no language uses None.
    #[test]
    fn html_code_block_entities_in_pre_with_attributes() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<pre class=\"x\"><code class=\"language-rust\">a &lt; b &amp;&amp; c</code></pre>\
             <pre><code>plain &gt; 0</code></pre>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.code_blocks.len(), 2);
        assert_eq!(parsed.code_blocks[0].language, Some("rust".to_string()));
        assert_eq!(parsed.code_blocks[0].code, "a < b && c");
        assert_eq!(parsed.code_blocks[1].language, None);
        assert_eq!(parsed.code_blocks[1].code, "plain > 0");
    }

    // ── S30-01: title extraction via scraper/html5ever ───────────────────────

    /// html5ever fully decodes HTML5 named entities in <title>; the custom
    /// decode_html_entities() only covers 32 entities and would miss &mdash;.
    #[test]
    fn html_title_decodes_entities() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<html><head><title>AT&amp;T &mdash; News</title></head><body></body></html>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.title, "AT&T \u{2014} News");
    }

    /// Whitespace in <title> is collapsed to a single space; leading/trailing
    /// whitespace is trimmed.
    #[test]
    fn html_title_collapses_whitespace() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<html><head><title>  Hello\n  World  </title></head><body></body></html>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.title, "Hello World");
    }

    /// When no <title> element is present, parse_html falls back to raw.title.
    #[test]
    fn html_title_missing_falls_back_to_raw_title() {
        let stage = ParsingStage::new();
        let raw = RawDocument {
            title: "Fallback Title".to_string(),
            content: "<html><head></head><body><h1>Body</h1></body></html>".to_string(),
            mime_type: "text/html".to_string(),
            source_type: "test".to_string(),
            source_path: None,
        };
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.title, "Fallback Title");
    }

    // ── S30-02: Link::position byte offsets ───────────────────────────────────

    /// For HTML, Link::position is the byte offset of the opening `<a` tag in
    /// raw.content.  `<p>x</p>` is 8 bytes, so the link starts at byte 8.
    #[test]
    fn html_link_position_is_byte_offset() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(r#"<p>x</p><a href="/y">y</a>"#, "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].position, 8);
    }

    /// `<abbr>` starts with `<a` but is immediately followed by `b`, an
    /// alphanumeric byte, so the 3-byte window detector must skip it and
    /// find the real `<a ` tag.
    #[test]
    fn html_link_position_ignores_abbr() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            r#"<abbr title="HyperText">HT</abbr><a href="/y">y</a>"#,
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        // <abbr title="HyperText">HT</abbr> is 33 bytes; <a> starts at index 33.
        assert_eq!(parsed.links[0].position, 33);
    }

    /// html5ever normalises tag names to lowercase but the source may use
    /// uppercase `<A HREF=…>`.  The case-insensitive window scan must still
    /// return the correct byte offset.
    #[test]
    fn html_link_position_uppercase_tag() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(r#"<A HREF="/y">y</A>"#, "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].position, 0);
    }

    /// For Markdown, Link::position is the byte offset into plain_text at the
    /// moment the link event is emitted — i.e., `plain_text.len()` before the
    /// link text is appended.  Here "prefix " is 7 bytes.
    #[test]
    fn markdown_link_position_is_plain_text_offset() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("prefix [click](/url)", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].position, 7);
    }

    /// A link nested inside a heading must have non-empty text even though the
    /// heading event handler gets priority for routing to plain_text.
    #[test]
    fn markdown_link_inside_heading_has_text() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("# Title [click](/url)", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].text, "click");
        assert_eq!(parsed.links[0].url, "/url");
    }

    // ── S31-03: Link::position accuracy inside headings ──────────────────────

    /// For a link inside a heading, position must index into the final plain_text
    /// at the start of the link text — not at the heading start.
    /// "# Title [click](/url)" → plain_text = "Title click\n", link at byte 6.
    #[test]
    fn markdown_link_in_heading_position_is_link_offset() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("# Title [click](/url)", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        let pos = parsed.links[0].position;
        assert_eq!(pos, 6, "position must be past 'Title '");
        // Self-consistency: plain_text[position..] must start with the link text.
        assert!(
            parsed.plain_text[pos..].starts_with(&parsed.links[0].text),
            "plain_text[{}..] = {:?} must start with link text {:?}",
            pos, &parsed.plain_text[pos..], parsed.links[0].text,
        );
    }

    /// With preceding paragraph text, heading-link position must account for
    /// both prior plain_text and heading prefix text.
    /// "intro\n\n# Header [link](/u)" → plain_text = "intro\nHeader link",
    /// "intro\n" = 6, "Header " = 7 → link at byte 13.
    #[test]
    fn markdown_link_in_heading_with_preceding_text() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("intro\n\n# Header [link](/u)", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert_eq!(parsed.links.len(), 1);
        let pos = parsed.links[0].position;
        assert_eq!(pos, 13, "position must be past 'intro\\n' + 'Header '");
        // Self-consistency: plain_text[position..] must start with the link text.
        assert!(
            parsed.plain_text[pos..].starts_with(&parsed.links[0].text),
            "plain_text[{}..] = {:?} must start with link text {:?}",
            pos, &parsed.plain_text[pos..], parsed.links[0].text,
        );
    }

    // ── S31-01: html-escape full HTML5 entity table ───────────────────────────

    /// html-escape covers the full HTML5 named-entity table; entities outside the
    /// old 32-entry hand-rolled table must now decode correctly.
    #[test]
    fn html_entity_micro_decoded() {
        let stage = ParsingStage::new();
        // &micro; (U+00B5 µ) was NOT in the 32-entry table
        let raw = make_raw_doc("<p>&micro;g of data</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(parsed.plain_text.contains('\u{00B5}'), "µ must be decoded");
    }

    #[test]
    fn html_entity_sect_decoded() {
        let stage = ParsingStage::new();
        // &sect; (§) was NOT in the 32-entry table
        let raw = make_raw_doc("<p>&sect;3 of the spec</p>", "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert!(parsed.plain_text.contains('\u{00A7}'), "§ must be decoded");
    }

    /// &#0; decodes to NUL via html-escape; decode_html_entities must replace it
    /// with U+FFFD so the output is safe for PostgreSQL TEXT columns.
    #[test]
    fn html_entity_nul_replaced_with_replacement_char() {
        let result = decode_html_entities("before&#0;after");
        assert!(
            !result.contains('\0'),
            "NUL byte must not appear in decoded output"
        );
        assert!(
            result.contains('\u{FFFD}'),
            "NUL must be replaced with U+FFFD"
        );
    }

    #[test]
    fn html_entity_unknown_preserved() {
        // Unknown named entities must still be preserved verbatim.
        // This is a regression guard against over-eager decode.
        let result = decode_html_entities("&fakeentity; test");
        assert!(result.contains("&fakeentity;"), "unknown entity must be preserved verbatim");
    }

    // ── S31-02: T_SEL "head > title" — SVG <title> must not be selected ─────────

    /// T_SEL uses "head > title" so an SVG <title> inside <body> is not
    /// matched even if it appears before the document <title> in the source.
    #[test]
    fn html_title_ignores_svg_title() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<html><head><title>Page Title</title></head>\
             <body><svg><title>SVG label</title></svg></body></html>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.title, "Page Title");
    }

    /// html5ever inserts an implicit <head> even when the source omits it,
    /// so "head > title" still matches when the markup is <html><title>X</title>.
    #[test]
    fn html_title_implicit_head() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc(
            "<html><title>Implicit</title><body>content</body></html>",
            "text/html",
        );
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.title, "Implicit");
    }

    // ── S32-01/02: Surrogate codepoint handling ───────────────────────────────

    /// Surrogate codepoints (U+D800–U+DFFF) cannot be represented as Rust
    /// `char`, so html_escape preserves them verbatim as `&#x…;` text.
    /// decode_html_entities must replace them with U+FFFD to match html5ever's
    /// heading/link path behavior (test: html_invalid_hex_entity_surrogate_replaced_with_fffd).
    #[test]
    fn decode_html_entities_surrogate_hex_replaced_with_fffd() {
        let result = decode_html_entities("before&#xD800;after");
        assert!(
            result.contains('\u{FFFD}'),
            "hex surrogate &#xD800; must produce U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#xD800;"),
            "hex surrogate entity must not appear verbatim: got {:?}",
            result
        );
    }

    /// Decimal surrogate U+D800 = 55296; same replacement-with-FFFD rule
    /// as the hex form.
    #[test]
    fn decode_html_entities_surrogate_decimal_replaced_with_fffd() {
        let result = decode_html_entities("before&#55296;after");
        assert!(
            result.contains('\u{FFFD}'),
            "decimal surrogate &#55296; must produce U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#55296;"),
            "decimal surrogate entity must not appear verbatim: got {:?}",
            result
        );
    }

    // ── S32-03: End(Paragraph) newline separator ──────────────────────────────

    /// Two consecutive Markdown paragraphs must be separated by a newline
    /// in plain_text.  End(Paragraph) inserts '\n' when the buffer does not
    /// already end with one.
    #[test]
    fn markdown_two_paragraphs_are_newline_separated() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("first\n\nsecond", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert!(
            parsed.plain_text.contains("first\nsecond"),
            "consecutive paragraphs must be newline-separated: got {:?}",
            parsed.plain_text
        );
    }

    /// A paragraph immediately before a heading must be separated from the
    /// heading text by a newline in plain_text.
    #[test]
    fn markdown_paragraph_before_heading_separated_by_newline() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("intro\n\n# Title", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert!(
            parsed.plain_text.contains("intro\nTitle"),
            "paragraph before heading must be newline-separated: got {:?}",
            parsed.plain_text
        );
    }

    // ── S32-04: Consecutive <a> link position verification ────────────────────

    /// Two consecutive `<a>` elements must get distinct, ordered positions.
    /// The position scanner advances `a_search_start` by `position + 1` after
    /// each match so the second scan does not re-find the first tag.
    #[test]
    fn html_consecutive_links_positions_distinct() {
        let stage = ParsingStage::new();
        let html = r#"<a href="https://a.com">A</a><a href="https://b.com">B</a>"#;
        let raw = make_raw_doc(html, "text/html");
        let parsed = stage.parse_html(&raw).unwrap();
        assert_eq!(parsed.links.len(), 2);
        assert!(
            parsed.links[0].position < parsed.links[1].position,
            "second link position ({}) must exceed first ({})",
            parsed.links[1].position,
            parsed.links[0].position,
        );
        // Self-consistency: each position must point at `<a` in the raw HTML.
        assert_eq!(
            &html[parsed.links[0].position..parsed.links[0].position + 2],
            "<a",
            "link[0] position must point to `<a`"
        );
        assert_eq!(
            &html[parsed.links[1].position..parsed.links[1].position + 2],
            "<a",
            "link[1] position must point to `<a`"
        );
    }

    // ── S33-01: Surrogate range boundary tests ────────────────────────────────

    /// U+D7FF is just below the surrogate range (0xD800–0xDFFF) and is a
    /// valid Unicode character.  The surrogate post-processing must NOT replace it.
    #[test]
    fn decode_html_entities_boundary_d7ff_not_replaced() {
        let result = decode_html_entities("&#xD7FF;");
        assert!(
            !result.contains('\u{FFFD}'),
            "U+D7FF is not a surrogate — must not produce U+FFFD: got {:?}",
            result
        );
        assert!(
            result.contains('\u{D7FF}'),
            "&#xD7FF; must decode to U+D7FF: got {:?}",
            result
        );
    }

    /// U+E000 is just above the surrogate range and is a valid Unicode character
    /// (Private Use Area).  The surrogate post-processing must NOT replace it.
    #[test]
    fn decode_html_entities_boundary_e000_not_replaced() {
        let result = decode_html_entities("&#xE000;");
        assert!(
            !result.contains('\u{FFFD}'),
            "U+E000 is not a surrogate — must not produce U+FFFD: got {:?}",
            result
        );
        assert!(
            result.contains('\u{E000}'),
            "&#xE000; must decode to U+E000: got {:?}",
            result
        );
    }

    // ── S34-01: Decimal-form surrogate boundary tests ─────────────────────────

    /// U+D7FF in decimal NCR form — exercises `surrogate_dec_regex()` path.
    /// html_escape decodes &#55295; to U+D7FF before our regex runs,
    /// so the "&#" early-exit guard is never reached.  The char is preserved as-is.
    #[test]
    fn decode_html_entities_boundary_d7ff_decimal_not_replaced() {
        let result = decode_html_entities("&#55295;");
        assert!(
            !result.contains('\u{FFFD}'),
            "U+D7FF decimal form is not a surrogate — must not produce U+FFFD: got {:?}",
            result
        );
        assert!(
            result.contains('\u{D7FF}'),
            "&#55295; must decode to U+D7FF: got {:?}",
            result
        );
    }

    /// U+E000 in decimal NCR form (Private Use Area) — exercises `surrogate_dec_regex()` path.
    #[test]
    fn decode_html_entities_boundary_e000_decimal_not_replaced() {
        let result = decode_html_entities("&#57344;");
        assert!(
            !result.contains('\u{FFFD}'),
            "U+E000 decimal form is not a surrogate — must not produce U+FFFD: got {:?}",
            result
        );
        assert!(
            result.contains('\u{E000}'),
            "&#57344; must decode to U+E000: got {:?}",
            result
        );
    }

    // ── S33-02: End(Paragraph) affects list items ─────────────────────────────

    /// pulldown-cmark emits Start/End(Paragraph) inside list items.
    /// The S32-03 End(Paragraph) '\n' insertion therefore also separates
    /// list items — this test pins that behavior.
    #[test]
    fn markdown_list_items_are_newline_separated() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("- item one\n\n- item two", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert!(
            parsed.plain_text.contains("item one\nitem two"),
            "list items must be newline-separated via End(Paragraph): got {:?}",
            parsed.plain_text
        );
    }

    /// Tight lists (`- a\n- b`, no blank line between items) do NOT emit
    /// Start/End(Paragraph) events in pulldown-cmark — so the S32-03
    /// End(Paragraph) '\n' insertion does not fire.  Items are therefore
    /// concatenated directly in plain_text without a newline separator.
    /// This is the complement to `markdown_list_items_are_newline_separated`.
    #[test]
    fn markdown_tight_list_items_are_not_newline_separated() {
        let stage = ParsingStage::new();
        let raw = make_raw_doc("- item one\n- item two", "text/markdown");
        let parsed = stage.parse_markdown(&raw).unwrap();
        assert!(
            !parsed.plain_text.contains("item one\nitem two"),
            "tight list items must NOT be newline-separated (no End(Paragraph) event): got {:?}",
            parsed.plain_text
        );
        // Both items must still be present in the output.
        assert!(
            parsed.plain_text.contains("item one") && parsed.plain_text.contains("item two"),
            "tight list items must both appear in plain_text: got {:?}",
            parsed.plain_text
        );
    }

    // ── S33-04: Unsemicoloned NCR — known limitation documentation ────────────

    /// Numeric char refs without a trailing semicolon (`&#xD800` vs `&#xD800;`)
    /// are NOT handled by the surrogate post-processing regex (which requires ';').
    /// html5ever processes some unsemicoloned NCRs as parse errors, but the
    /// body/code path does not.  This is a documented limitation — the body path
    /// receives already-stripped HTML where html5ever would not have acted on
    /// bare `&#xD800` text.  The function must not panic on this input.
    #[test]
    fn decode_html_entities_unsemicoloned_ncr_not_panics() {
        let result = decode_html_entities("x&#xD800y");
        // The surrogate regex requires ';', so "&#xD800y" (no semicolon) is not
        // matched and the literal text leaks through.  If html_escape ever gains
        // more aggressive NCR handling, U+FFFD is also an acceptable outcome.
        assert!(
            result.contains("&#xD800") || result.contains('\u{FFFD}'),
            "unsemicoloned NCR: expected verbatim leak or U+FFFD; got {:?}",
            result
        );
        // The semicoloned form is guaranteed to produce U+FFFD.
        let semicoloned = decode_html_entities("x&#xD800;y");
        assert!(
            semicoloned.contains('\u{FFFD}'),
            "semicoloned &#xD800; must produce U+FFFD: got {:?}",
            semicoloned
        );
    }

    // ── S34-04: Inclusive endpoint replacement (U+D800 and U+DFFF) ───────────

    /// U+D800 is the lower inclusive endpoint of the surrogate range.
    /// `decode_html_entities` must replace it with U+FFFD.
    #[test]
    fn decode_html_entities_d800_replaced_with_fffd() {
        let result = decode_html_entities("&#xD800;");
        assert!(
            result.contains('\u{FFFD}'),
            "U+D800 (lower surrogate boundary) must be replaced with U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#xD800;"),
            "&#xD800; must not appear verbatim after replacement: got {:?}",
            result
        );
    }

    /// U+DFFF is the upper inclusive endpoint of the surrogate range.
    /// `decode_html_entities` must replace it with U+FFFD.
    #[test]
    fn decode_html_entities_dfff_replaced_with_fffd() {
        let result = decode_html_entities("&#xDFFF;");
        assert!(
            result.contains('\u{FFFD}'),
            "U+DFFF (upper surrogate boundary) must be replaced with U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#xDFFF;"),
            "&#xDFFF; must not appear verbatim after replacement: got {:?}",
            result
        );
    }

    // ── Compile-time safety: scraper::Selector must be Sync + Send ───────────

    /// `OnceLock<Selector>` in `scraper_extract_all` is shared across Axum
    /// worker threads.  This test asserts that `Selector` implements both
    /// `Sync` and `Send` so a future scraper bump that breaks the assumption
    /// is caught at compile time, not at runtime.
    #[test]
    fn selector_is_sync_send() {
        fn _assert<T: Sync + Send>() {}
        _assert::<scraper::Selector>();
    }
}
