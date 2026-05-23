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
///
/// | Input form                        | Result                            |
/// |-----------------------------------|-----------------------------------|
/// | `&#xD800;` (direct surrogate)     | `"\u{FFFD}"` (replacement char)   |
/// | `&amp;#xD800;` (amp-escaped)      | `"&#xD800;"` (literal string)     |
/// | `&#x200000;` (above Unicode)      | `"&#x200000;"` (verbatim)         |
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

    /// U+D7FF (`&#xD7FF;`) is just below the surrogate range.  `html_escape`
    /// decodes it to the valid character U+D7FF directly, so `result.contains("&#")`
    /// is false and the surrogate post-processing is short-circuited.  End-to-end
    /// contract: a valid codepoint adjacent to the surrogate range must not be
    /// replaced with U+FFFD regardless of which internal branch runs.
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

    /// U+E000 (`&#xE000;`) is just above the surrogate range (Private Use Area).
    /// `html_escape` decodes it directly, so the surrogate post-processing is
    /// short-circuited.  End-to-end contract: a valid PUA codepoint must not be
    /// replaced with U+FFFD.
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

    /// U+D7FF in decimal NCR form (`&#55295;`).  `html_escape` decodes this to
    /// U+D7FF directly, so `result.contains("&#")` is false and the surrogate
    /// post-processing is short-circuited.  End-to-end contract: symmetric with
    /// the hex form — valid codepoint just below the surrogate range is preserved.
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

    /// U+E000 in decimal NCR form (`&#57344;`, Private Use Area).  `html_escape`
    /// decodes this directly; surrogate post-processing is short-circuited.
    /// End-to-end contract: symmetric with the hex form — valid PUA codepoint
    /// just above the surrogate range is preserved.
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
        // Tight list: no Paragraph events → items concatenated with no separator.
        assert!(
            parsed.plain_text.contains("item oneitem two"),
            "tight list items must be concatenated with no separator: got {:?}",
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

    // ── S35-01: Decimal upper endpoint (U+DFFF = 57343) ──────────────────────

    /// U+DFFF in decimal NCR form (`&#57343;`) — upper inclusive endpoint of the
    /// surrogate range via the decimal path.  html_escape leaves it verbatim
    /// (Rust `String` cannot hold surrogates), so `surrogate_dec_regex()` runs
    /// and replaces it with U+FFFD.  Symmetric with the hex form S34-04.
    #[test]
    fn decode_html_entities_surrogate_decimal_dfff_replaced_with_fffd() {
        let result = decode_html_entities("&#57343;");
        assert!(
            result.contains('\u{FFFD}'),
            "U+DFFF decimal (57343) must be replaced with U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#57343;"),
            "&#57343; must not appear verbatim after replacement: got {:?}",
            result
        );
    }

    // ── S35-02: Unsemicoloned NCR surrounding-character survival ──────────────

    /// Verifies that characters adjacent to an unsemicoloned NCR (`x&#xD800y`)
    /// survive the function unchanged.  A regression in the regex (e.g. a
    /// greedy pattern) could accidentally consume neighbouring bytes.
    /// The union assertion pins both the surrounding bytes and the middle:
    /// either the NCR leaks verbatim or only the middle is replaced with U+FFFD.
    #[test]
    fn decode_html_entities_unsemicoloned_ncr_surrounding_chars_survive() {
        let result = decode_html_entities("x&#xD800y");
        assert!(
            result == "x&#xD800y" || result == "x\u{FFFD}y",
            "unsemicoloned NCR: expected verbatim leak or middle-only U+FFFD; got {:?}",
            result
        );
    }

    // ── S35-03: Mixed-NCR sequence (surrogates + valid codepoints) ────────────

    /// A single input mixing two surrogates and two valid adjacent codepoints:
    ///   &#xD800;  — lower surrogate → U+FFFD (hex regex)
    ///   &#xDFFF;  — upper surrogate → U+FFFD (hex regex)
    ///   &#55295;  — U+D7FF (valid, html_escape decodes before regex runs)
    ///   &#xE000;  — U+E000 PUA (valid, html_escape decodes before regex runs)
    ///
    /// Verifies that the two-pass regex chain (hex then dec) handles each NCR
    /// independently without cross-contamination.
    #[test]
    fn decode_html_entities_mixed_ncr_sequence() {
        let input = "&#xD800;&#xDFFF;&#55295;&#xE000;";
        let result = decode_html_entities(input);
        // Two surrogates → two U+FFFD replacements.
        let fffd_count = result.chars().filter(|&c| c == '\u{FFFD}').count();
        assert_eq!(
            fffd_count, 2,
            "mixed NCR sequence must produce exactly 2 U+FFFD replacements: got {:?}",
            result
        );
        // Valid adjacent codepoints must be preserved.
        assert!(
            result.contains('\u{D7FF}'),
            "U+D7FF must be preserved in mixed sequence: got {:?}",
            result
        );
        assert!(
            result.contains('\u{E000}'),
            "U+E000 must be preserved in mixed sequence: got {:?}",
            result
        );
    }

    // ── S35-04: Astral-plane NCR (4-byte UTF-8) ───────────────────────────────

    /// `&#128512;` is U+1F600 (😀), a 4-byte UTF-8 codepoint in the Supplementary
    /// Multilingual Plane.  `html_escape` decodes it directly; the surrogate
    /// post-processing is short-circuited (result does not contain "&#" after
    /// decoding).  Pins the astral-plane path that was previously untested.
    #[test]
    fn decode_html_entities_astral_plane_ncr_preserved() {
        let result = decode_html_entities("&#128512;");
        assert!(
            result.contains('\u{1F600}'),
            "&#128512; must decode to U+1F600 (😀): got {:?}",
            result
        );
        assert!(
            !result.contains('\u{FFFD}'),
            "astral-plane NCR must not produce U+FFFD: got {:?}",
            result
        );
    }

    // ── S36-01: Above-Unicode codepoints (> U+10FFFF) ────────────────────────

    /// `&#x110000;` (hex) and `&#1114112;` (decimal) are one past U+10FFFF, the
    /// highest valid Unicode scalar value.  `char::try_from(1114112)` returns `Err`.
    /// Under `html_escape 0.2.13` the state machine requires a successful
    /// `char::try_from` conversion; on failure it preserves the entity verbatim
    /// (does NOT produce U+FFFD).  The surrogate post-processing also leaves them
    /// untouched (1114112 is not in 0xD800–0xDFFF).  Both passes converge to
    /// verbatim, pinned here.  If `html_escape` ever adopts the HTML5-spec
    /// replacement-character behaviour (U+FFFD for > U+10FFFF), update these tests.
    #[test]
    fn decode_html_entities_above_unicode_hex_preserved_verbatim() {
        let result = decode_html_entities("&#x110000;");
        assert_eq!(
            result, "&#x110000;",
            "&#x110000; must be preserved verbatim (html_escape 0.2.x): got {:?}",
            result
        );
    }

    #[test]
    fn decode_html_entities_above_unicode_decimal_preserved_verbatim() {
        let result = decode_html_entities("&#1114112;");
        assert_eq!(
            result, "&#1114112;",
            "&#1114112; must be preserved verbatim (html_escape 0.2.x): got {:?}",
            result
        );
    }

    // ── S36-02: Case sensitivity of hex NCR prefix ────────────────────────────

    /// The hex regex uses `(?i)`, making the `x` in `&#x...;` case-insensitive.
    /// `&#xd800;` (lowercase digits and x) and `&#XD800;` (uppercase X) must
    /// both produce U+FFFD, proving the `(?i)` flag is load-bearing.
    #[test]
    fn decode_html_entities_lowercase_hex_surrogate_replaced_with_fffd() {
        let result = decode_html_entities("&#xd800;");
        assert!(
            result.contains('\u{FFFD}'),
            "&#xd800; (lowercase) must be replaced with U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#xd800;"),
            "&#xd800; must not appear verbatim after replacement: got {:?}",
            result
        );
    }

    #[test]
    fn decode_html_entities_uppercase_x_surrogate_replaced_with_fffd() {
        let result = decode_html_entities("&#XD800;");
        assert!(
            result.contains('\u{FFFD}'),
            "&#XD800; (uppercase X) must be replaced with U+FFFD: got {:?}",
            result
        );
        assert!(
            !result.contains("&#XD800;"),
            "&#XD800; must not appear verbatim after replacement: got {:?}",
            result
        );
    }

    // ── S36-04: Idempotency of decode_html_entities ───────────────────────────

    /// For single-decoded outputs (surrogate → U+FFFD, NCR → char, U+FFFD passthrough,
    /// plain text), running `decode_html_entities` a second time returns the same string.
    ///
    /// Note: this property does NOT hold for chained `&amp;` forms — e.g.
    /// `"&amp;amp;"` decodes to `"&amp;"` on the first pass and `"&"` on the second.
    /// See `decode_html_entities_amp_chain_peels_one_level` for that contract.
    /// Production call sites invoke this function exactly once per source string, so
    /// chained forms do not arise.
    #[test]
    fn decode_html_entities_is_idempotent_for_single_decoded_outputs() {
        let cases = [
            "&#xD800;",          // surrogate → U+FFFD on first pass
            "&#55296;",          // surrogate decimal → U+FFFD on first pass
            "hello &amp; world", // named entity &amp; → '&' followed by space, not re-parseable
            "&#128512;",         // astral plane → U+1F600 on first pass
            "\u{FFFD}",          // already U+FFFD — must survive second pass unchanged
            "plain text",        // no entities
        ];
        for input in &cases {
            let once = decode_html_entities(input);
            let twice = decode_html_entities(&once);
            assert_eq!(
                once, twice,
                "decode_html_entities must be idempotent for {:?}: \
                 first pass={:?}, second pass={:?}",
                input, once, twice
            );
        }
    }

    /// Pins the peel-one-level contract for chained `&amp;` forms.
    /// This is intentionally NOT idempotent: each call peels one layer of encoding.
    /// Production callers invoke this function once per source string, so the
    /// non-idempotent chain case does not arise in practice.
    #[test]
    fn decode_html_entities_amp_chain_peels_one_level() {
        assert_eq!(decode_html_entities("&amp;amp;"), "&amp;");
        assert_eq!(decode_html_entities("&amp;"), "&");
    }

    // ── S37-01: &amp;#xD800; end-to-end contract ──────────────────────────────

    /// Pins the docstring contract (line ~220-223): an escaped-ampersand surrogate
    /// form `&amp;#xD800;` is "more aggressive than html5ever" — the `&amp;` is
    /// peeled to `&` by html_escape, yielding `&#xD800;`, which the hex-surrogate
    /// regex then replaces with U+FFFD.  The entire sequence resolves in one call.
    ///
    /// html5ever (heading/link path) would treat `&amp;#xD800;` as a text node
    /// containing the literal characters `&#xD800;` and preserve them as-is;
    /// `decode_html_entities` produces U+FFFD instead, which is safe for indexing.
    ///
    /// Verified against html-escape 0.2.x (Cargo.toml). Bumps must re-verify the
    /// peel-once &amp;→& behavior and the surrogate-verbatim contract.
    #[test]
    fn decode_html_entities_amp_escaped_surrogate_produces_fffd() {
        // bare escaped-ampersand form
        assert_eq!(
            decode_html_entities("&amp;#xD800;"),
            "\u{FFFD}",
            "&amp;#xD800; must decode to U+FFFD via peel-then-regex path"
        );
        // with surrounding text — surrounding chars must survive
        assert_eq!(
            decode_html_entities("before&amp;#xD800;after"),
            "before\u{FFFD}after",
            "surrounding text must be preserved around &amp;#xD800;"
        );
    }

    /// Decimal variant: `&amp;#55296;` (= &#55296; = U+D800 in decimal).
    /// Same peel-then-regex path as the hex variant.
    ///
    /// Verified against html-escape 0.2.x (Cargo.toml).
    #[test]
    fn decode_html_entities_amp_escaped_surrogate_decimal_produces_fffd() {
        assert_eq!(
            decode_html_entities("&amp;#55296;"),
            "\u{FFFD}",
            "&amp;#55296; must decode to U+FFFD via peel-then-regex path"
        );
    }

    // ── S37-02: Mixed-case hex digit in surrogate NCR ────────────────────────

    /// Exercises the `(?i)` flag's digit-case axis — not just the `x`/`X` prefix
    /// axis tested in S36-02.  `&#xDc00;` has an uppercase `D` and lowercase `c`
    /// in the hex payload, which `[0-9a-f]` with `(?i)` must match.
    ///
    /// 0xDC00 = 56320 ∈ [0xD800, 0xDFFF] → U+FFFD.
    #[test]
    fn decode_html_entities_mixed_case_hex_digit_surrogate_produces_fffd() {
        assert_eq!(
            decode_html_entities("&#xDc00;"),
            "\u{FFFD}",
            "&#xDc00; (mixed-case digits) must match (?i) pattern and produce U+FFFD"
        );
        // Companion: all-uppercase digits
        assert_eq!(
            decode_html_entities("&#xDFFF;"),
            "\u{FFFD}",
            "&#xDFFF; (all-uppercase) must produce U+FFFD"
        );
    }

    // ── S37-03: Hex u32 overflow — parse-Err path verbatim ──────────────────

    /// `&#xFFFFFFFF;` (u32::MAX = 4294967295) — parse succeeds as u32 but the
    /// value is far outside the surrogate range (0xD800..=0xDFFF), so the
    /// fallback arm `_ => caps[0].to_owned()` returns the entity verbatim.
    /// html_escape also leaves it verbatim (char::try_from fails for >U+10FFFF).
    #[test]
    fn decode_html_entities_hex_u32_max_preserved_verbatim() {
        let result = decode_html_entities("&#xFFFFFFFF;");
        assert_eq!(
            result, "&#xFFFFFFFF;",
            "&#xFFFFFFFF; (u32::MAX) must be preserved verbatim; got {:?}", result
        );
    }

    /// `&#xFFFFFFFFF;` (9 hex digits, > u32::MAX) — `u32::from_str_radix` returns
    /// `Err` (overflow).  The `_ =>` fallback must return the entity verbatim, not
    /// panic or silently truncate.
    #[test]
    fn decode_html_entities_hex_u32_overflow_preserved_verbatim() {
        let result = decode_html_entities("&#xFFFFFFFFF;");
        assert_eq!(
            result, "&#xFFFFFFFFF;",
            "&#xFFFFFFFFF; (> u32::MAX) must be preserved verbatim; got {:?}", result
        );
    }

    // ── S37-04: Cross-path consistency (scraper heading vs decode_html_entities) ─

    /// Both the html5ever/scraper heading-extraction path and the
    /// `decode_html_entities` body path must produce U+FFFD for the same
    /// surrogate NCR input.  This pins the docstring contract (line ~218-219)
    /// that the two paths "replace direct surrogate NCRs with U+FFFD in the
    /// same way."
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x (Cargo.toml).
    /// If this test fails after a scraper bump, audit html5ever's tokenizer
    /// surrogate-NCR replacement behavior before adjusting the assertion.
    #[test]
    fn scraper_heading_and_decode_html_entities_agree_on_surrogate_fffd() {
        // Heading path (scraper / html5ever)
        let html = "<html><body><h1>&#xD800;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(
            headings[0].text, "\u{FFFD}",
            "scraper heading path must produce U+FFFD for &#xD800;"
        );

        // Body path (decode_html_entities)
        let body_result = decode_html_entities("&#xD800;");
        assert_eq!(
            body_result, "\u{FFFD}",
            "decode_html_entities body path must produce U+FFFD for &#xD800;"
        );

        // Cross-path agreement
        assert_eq!(
            headings[0].text, body_result,
            "heading path and body path must agree on surrogate → U+FFFD"
        );
    }

    /// Decimal surrogate NCR cross-path consistency — same contract as hex.
    /// U+DFFF (decimal 57343) is the upper endpoint of the surrogate range.
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x (Cargo.toml).
    #[test]
    fn scraper_heading_and_decode_html_entities_agree_on_surrogate_decimal() {
        let html = "<html><body><h1>&#57343;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(
            headings[0].text, "\u{FFFD}",
            "scraper heading path must produce U+FFFD for &#57343;"
        );
        let body_result = decode_html_entities("&#57343;");
        assert_eq!(body_result, "\u{FFFD}");
        assert_eq!(headings[0].text, body_result);
    }

    /// Pins the intentional asymmetry between paths documented at line ~220-223:
    /// html5ever treats `&amp;#xD800;` as a text node and preserves it verbatim
    /// as the characters `&#xD800;`; `decode_html_entities` peels the `&amp;`
    /// first (html_escape), making the entity active, then replaces it with U+FFFD.
    ///
    /// This divergence is deliberate — the body/code path is "slightly more
    /// aggressive" to ensure surrogate NCRs never reach the PostgreSQL index.
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x.
    #[test]
    fn scraper_heading_and_decode_html_entities_diverge_on_amp_escaped_surrogate() {
        // html5ever heading path: &amp; is a text-level entity; the resulting
        // text node contains the literal characters '&', '#', 'x', 'D', '8', '0', '0', ';'
        let html = "<html><body><h1>&amp;#xD800;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(
            headings[0].text, "&#xD800;",
            "html5ever heading path must preserve &amp;#xD800; as literal text &#xD800;"
        );

        // decode_html_entities body path: peels &amp; → & → &#xD800; → U+FFFD
        let body_result = decode_html_entities("&amp;#xD800;");
        assert_eq!(
            body_result, "\u{FFFD}",
            "decode_html_entities body path must produce U+FFFD for &amp;#xD800;"
        );

        // Intentional asymmetry — they must NOT agree on this input
        assert_ne!(
            headings[0].text, body_result,
            "divergence at parsing.rs:~220-223 must be preserved"
        );
    }

    // ── S38-01: &amp;#xD800 (no trailing semicolon) verbatim ──────────────────

    /// Pins the interaction between S33-04 (unsemicoloned NCR) and S37-01
    /// (&amp;-escaped surrogate chain).
    ///
    /// `&amp;#xD800` (no `;`) → html_escape peels `&amp;` → `&`, leaving
    /// `&#xD800` (no `;`).  The `result.contains("&#")` guard succeeds, so the
    /// regex block is entered, but `(?i)&#x([0-9a-f]+);` requires a trailing
    /// semicolon and `replace_all` finds no match.  The input is returned
    /// unchanged as `"&#xD800"` — verbatim, no panic.
    ///
    /// Verified against html-escape 0.2.x (Cargo.toml).
    #[test]
    fn decode_html_entities_amp_escaped_surrogate_no_semicolon_verbatim() {
        // Bare form: no trailing semicolon after hex digits
        let result = decode_html_entities("&amp;#xD800");
        assert_eq!(
            result, "&#xD800",
            "&amp;#xD800 (no ';') must be verbatim after &amp; peel; got {:?}", result
        );

        // With surrounding text — surrounding chars must survive
        let result2 = decode_html_entities("before&amp;#xD800 after");
        assert_eq!(
            result2, "before&#xD800 after",
            "surrounding text must be preserved; got {:?}", result2
        );

        // Decimal form: &amp;#55296 (no ';')
        let result3 = decode_html_entities("&amp;#55296");
        assert_eq!(
            result3, "&#55296",
            "&amp;#55296 (no ';') must be verbatim; got {:?}", result3
        );
    }

    // ── S38-03: Hex regex match-arm structural coverage ─────────────────────

    /// Table-driven test that explicitly marks which arm of the hex surrogate
    /// `match` (parsing.rs lines ~245-248) each case exercises.  To reach any
    /// arm, the input must survive html_escape decoding with `"&#"` still present.
    ///
    /// ```text
    /// match u32::from_str_radix(&caps[1], 16) {
    ///     Ok(cp) if (0xD800..=0xDFFF).contains(&cp) => U+FFFD,  // ARM-A
    ///     _                                          => verbatim, // ARM-B (Ok, out-of-range)
    ///                                                             // ARM-C (Err, overflow)
    /// }
    /// ```
    ///
    /// ARM-A: parse succeeds + value in surrogate range → U+FFFD
    ///        html_escape leaves surrogates verbatim; regex match runs.
    /// ARM-B: parse succeeds + value NOT in surrogate range → verbatim
    ///        Must be above-Unicode so html_escape also leaves it verbatim.
    ///        Valid Unicode codepoints (e.g. U+D7FF, U+E000) are decoded by
    ///        html_escape upstream and never reach this arm — see S33/S34 tests.
    /// ARM-C: parse fails (u32 overflow) → verbatim (same `_ =>` arm as ARM-B)
    #[test]
    fn decode_html_entities_hex_regex_match_arm_coverage() {
        // ARM-A: Ok(cp) in 0xD800..=0xDFFF → U+FFFD
        // html_escape preserves surrogates verbatim; regex runs; in-range guard fires.
        // u32::from_str_radix("D800", 16) = Ok(55296) — in range
        assert_eq!(decode_html_entities("&#xD800;"), "\u{FFFD}", "ARM-A lower bound");
        // u32::from_str_radix("DFFF", 16) = Ok(57343) — in range
        assert_eq!(decode_html_entities("&#xDFFF;"), "\u{FFFD}", "ARM-A upper bound");

        // ARM-B: Ok(cp) NOT in 0xD800..=0xDFFF → verbatim
        // &#x200000; = 2097152 (> U+10FFFF): html_escape leaves verbatim (char::try_from
        // fails for above-Unicode); regex matches; cp = 0x200000 ∉ surrogate range → fallback.
        assert_eq!(decode_html_entities("&#x200000;"), "&#x200000;", "ARM-B: above-Unicode verbatim");
        // &#xFFFFFFFF; = u32::MAX: html_escape verbatim; regex matches; cp ∉ surrogate range.
        assert_eq!(decode_html_entities("&#xFFFFFFFF;"), "&#xFFFFFFFF;", "ARM-B: u32::MAX verbatim");

        // ARM-C: Err (u32 overflow) → verbatim (same `_` arm as ARM-B)
        // u32::from_str_radix("FFFFFFFFF", 16) → Err(PosOverflow)
        assert_eq!(decode_html_entities("&#xFFFFFFFFF;"), "&#xFFFFFFFFF;", "ARM-C: overflow verbatim");
    }

    // ── S38-04: Decimal NCR u32 overflow — verbatim (S37-03 decimal symmetric) ─

    /// Decimal counterpart of S37-03: `&#4294967296;` = 2^32, which exceeds
    /// `u32::MAX` (4294967295).  `caps[1].parse::<u32>()` returns
    /// `Err(PosOverflow)`, and the `_ =>` fallback arm must return the entity
    /// verbatim without panicking.
    ///
    /// Note: html_escape 0.2.x also leaves these verbatim — its decimal parse
    /// step rejects values that exceed u32::MAX before `char::try_from` is reached.
    ///
    /// See also `decode_html_entities_above_unicode_decimal_preserved_verbatim` (S36)
    /// which covers above-Unicode but parse-Ok values (e.g. 1114112).  This test
    /// covers parse-Err (overflow).
    #[test]
    fn decode_html_entities_decimal_u32_overflow_preserved_verbatim() {
        // 2^32 = 4294967296 — first value that overflows u32
        let result = decode_html_entities("&#4294967296;");
        assert_eq!(
            result, "&#4294967296;",
            "&#4294967296; (> u32::MAX decimal) must be verbatim; got {:?}", result
        );

        // Even larger: 10 digits
        let result2 = decode_html_entities("&#99999999999;");
        assert_eq!(
            result2, "&#99999999999;",
            "&#99999999999; (10-digit decimal) must be verbatim; got {:?}", result2
        );
    }

    // ── S39-01: Decimal regex match-arm structural coverage ──────────────────

    /// Decimal counterpart of S38-03 (`decode_html_entities_hex_regex_match_arm_coverage`):
    /// explicitly annotates which arm of the decimal surrogate `match` each case
    /// exercises.  ARM-B uses `&#2097152;` (decimal 0x200000) to mirror S38-03's
    /// `&#x200000;`.
    ///
    /// ```text
    /// match caps[1].parse::<u32>() {
    ///     Ok(cp) if (0xD800..=0xDFFF).contains(&cp) => U+FFFD,  // ARM-A
    ///     _                                          => verbatim, // ARM-B/C
    /// }
    /// ```
    ///
    /// ARM-A: parse-Ok + in surrogate range → U+FFFD
    ///        html_escape leaves surrogates verbatim; decimal regex match runs.
    /// ARM-B: parse-Ok + NOT in surrogate range → verbatim
    ///        Must be above-Unicode (>U+10FFFF) so html_escape leaves it verbatim.
    ///        Valid codepoints (e.g. 55295, 57344) are decoded by html_escape
    ///        upstream; see S33/S34 tests for those boundary assertions.
    /// ARM-C: parse-Err (u32 overflow) → verbatim (same `_ =>` arm as ARM-B)
    ///        Covered by S38-04; repeated here for structural completeness.
    ///
    /// Verified against html-escape 0.2.x (Cargo.toml).
    #[test]
    fn decode_html_entities_decimal_regex_match_arm_coverage() {
        // ARM-A: Ok(cp) in 0xD800..=0xDFFF → U+FFFD
        // html_escape leaves surrogates verbatim (char::try_from fails for surrogates);
        // decimal regex runs; in-range guard fires.
        // parse::<u32>() = Ok(55296) — lower bound (U+D800)
        assert_eq!(decode_html_entities("&#55296;"), "\u{FFFD}", "ARM-A lower bound");
        // parse::<u32>() = Ok(57343) — upper bound (U+DFFF)
        assert_eq!(decode_html_entities("&#57343;"), "\u{FFFD}", "ARM-A upper bound");

        // ARM-B: Ok(cp) NOT in 0xD800..=0xDFFF → verbatim
        // &#2097152; = 0x200000 (> U+10FFFF): html_escape leaves verbatim (char::try_from
        // fails for above-Unicode); decimal regex matches; cp = 2097152 ∉ surrogate range.
        assert_eq!(decode_html_entities("&#2097152;"), "&#2097152;", "ARM-B: above-Unicode verbatim");
        // &#4294967295; = u32::MAX: html_escape verbatim; decimal regex matches; cp ∉ range.
        assert_eq!(decode_html_entities("&#4294967295;"), "&#4294967295;", "ARM-B: u32::MAX verbatim");

        // ARM-C: Err (u32 overflow) → verbatim (same `_` arm as ARM-B)
        // parse::<u32>() on "4294967296" → Err(PosOverflow)
        // (mirrors S38-04 which pins this as a standalone test)
        assert_eq!(decode_html_entities("&#4294967296;"), "&#4294967296;", "ARM-C: overflow verbatim");
    }

    // ── S39-02: Cross-path divergence table (decimal + boundary variants) ────

    /// Extends the S37-04 divergence test to cover decimal surrogate forms
    /// and the boundary non-surrogate case.
    ///
    /// Summary of intentional asymmetry (docstring line ~220-223):
    /// - `&amp;#<SURROGATE>;` → heading preserves literal `"&#...;"`, body → U+FFFD
    /// - `&amp;#<NON-SURROGATE>;` → both paths produce literal `"&#<value>;"` (AGREE)
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x and html-escape 0.2.x.
    #[test]
    fn scraper_heading_decode_html_entities_diverge_on_decimal_surrogate() {
        // Decimal D800 form: html5ever peels &amp;→& → text "&#55296;", verbatim.
        // decode_html_entities: peels &amp;→& → "&#55296;" → decimal regex → U+FFFD.
        let html = "<html><body><h1>&amp;#55296;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "&#55296;",
            "html5ever must preserve &amp;#55296; as literal text");
        assert_eq!(decode_html_entities("&amp;#55296;"), "\u{FFFD}",
            "body path must produce U+FFFD for &amp;#55296;");
        assert_ne!(headings[0].text, decode_html_entities("&amp;#55296;"),
            "decimal surrogate divergence must be preserved");
    }

    /// Hex U+DFFF (upper bound): same asymmetry as U+D800 (S37-04 M2 test),
    /// extended to the upper endpoint of the surrogate range.
    #[test]
    fn scraper_heading_decode_html_entities_diverge_on_hex_dfff() {
        let html = "<html><body><h1>&amp;#xDFFF;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "&#xDFFF;",
            "html5ever must preserve &amp;#xDFFF; as literal text");
        assert_eq!(decode_html_entities("&amp;#xDFFF;"), "\u{FFFD}",
            "body path must produce U+FFFD for &amp;#xDFFF;");
        assert_ne!(
            headings[0].text, decode_html_entities("&amp;#xDFFF;"),
            "divergence at parsing.rs:~220-223 must be preserved for upper-bound surrogate"
        );
    }

    /// Non-surrogate case: `&amp;#xD7FF;` (U+D7FF, just below surrogate range).
    /// Both paths agree — neither produces U+FFFD; both preserve `"&#xD7FF;"`.
    ///
    /// Heading path: html5ever peels `&amp;` → `&`, leaving `#xD7FF;` as text
    ///   → element text = `"&#xD7FF;"` (literal entity string, not decoded).
    /// Body path: html_escape peels `&amp;` → `&` → `"&#xD7FF;"` → hex regex
    ///   matches, cp=55295 ∉ surrogate range → fallback, verbatim `"&#xD7FF;"`.
    #[test]
    fn scraper_heading_decode_html_entities_agree_on_non_surrogate_amp_escaped() {
        let html = "<html><body><h1>&amp;#xD7FF;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        let body_result = decode_html_entities("&amp;#xD7FF;");
        assert_eq!(headings[0].text, "&#xD7FF;",
            "html5ever must produce literal '&#xD7FF;' for &amp;#xD7FF;");
        assert_eq!(body_result, "&#xD7FF;",
            "body path must produce literal '&#xD7FF;' for &amp;#xD7FF;");
        assert_eq!(headings[0].text, body_result,
            "both paths must agree for non-surrogate amp-escaped form");
    }

    // ── S39-03: html_escape behavior contract ────────────────────────────────

    /// Pins the five html_escape 0.2.x behaviors that `decode_html_entities`
    /// relies on.  If html-escape bumps and any of these change, tests in
    /// S37/S38/S39 that depend on them will need re-verification.
    ///
    /// Note: this test calls `html_escape::decode_html_entities` directly, not
    /// the local `decode_html_entities` wrapper, to isolate library behavior.
    ///
    /// Verified against html-escape 0.2.x (Cargo.toml).
    #[test]
    fn html_escape_library_behavior_contract() {
        // Contract 1 — Named entity peel-once: &amp; → & (one level only)
        assert_eq!(
            html_escape::decode_html_entities("&amp;").as_ref(), "&",
            "html_escape: &amp; must decode to &"
        );
        // Peel-once: &amp;amp; → &amp; (NOT &)
        assert_eq!(
            html_escape::decode_html_entities("&amp;amp;").as_ref(), "&amp;",
            "html_escape: &amp;amp; peels one level to &amp;"
        );

        // Contract 2 — Other named entities decoded
        assert_eq!(
            html_escape::decode_html_entities("&lt;&gt;").as_ref(), "<>",
            "html_escape: &lt;&gt; → <>"
        );

        // Contract 3 — Valid scalar NCR decoded to char
        // &#65; = U+0041 'A'; &#xD7FF; = U+D7FF (just below surrogate range)
        assert_eq!(
            html_escape::decode_html_entities("&#65;").as_ref(), "A",
            "html_escape: &#65; → 'A'"
        );
        assert_eq!(
            html_escape::decode_html_entities("&#xD7FF;").as_ref(), "\u{D7FF}",
            "html_escape: &#xD7FF; → U+D7FF (valid scalar decoded)"
        );

        // Contract 4 — Surrogate NCR left verbatim (char::try_from fails for surrogates)
        assert_eq!(
            html_escape::decode_html_entities("&#xD800;").as_ref(), "&#xD800;",
            "html_escape: &#xD800; must remain verbatim (surrogate)"
        );
        assert_eq!(
            html_escape::decode_html_entities("&#xDFFF;").as_ref(), "&#xDFFF;",
            "html_escape: &#xDFFF; must remain verbatim (surrogate)"
        );

        // Contract 5 — Above-Unicode NCR left verbatim (char::try_from fails)
        assert_eq!(
            html_escape::decode_html_entities("&#x110000;").as_ref(), "&#x110000;",
            "html_escape: &#x110000; must remain verbatim (above-Unicode)"
        );
    }

    // ── S40-02: ARM-A mid-range surrogate coverage ───────────────────────────

    /// Pins ARM-A for mid-range surrogate values (not just boundary endpoints
    /// D800/DFFF).  Confirms that `(0xD800..=0xDFFF).contains(&cp)` fires for
    /// interior codepoints, not only at boundaries already covered by S34/S38-03.
    #[test]
    fn decode_html_entities_surrogate_mid_range_produces_fffd() {
        // &#xDA00; = 0xDA00 = 55808 — mid-range high surrogate, hex form
        assert_eq!(
            decode_html_entities("&#xDA00;"),
            "\u{FFFD}",
            "&#xDA00; (0xDA00, mid-range high surrogate) must produce U+FFFD"
        );
        // &#56320; = 0xDC00 = 56320 — low surrogate lead (U+DC00), decimal form
        // Symmetric with hex path; exercises decimal regex ARM-A interior.
        assert_eq!(
            decode_html_entities("&#56320;"),
            "\u{FFFD}",
            "&#56320; (0xDC00, low surrogate, decimal) must produce U+FFFD"
        );
    }

    // ── S40-03: &amp;#xE000; above-surrogate AGREE corner ───────────────────

    /// Symmetric counterpart of S39-02c (`&amp;#xD7FF;` below-surrogate AGREE).
    /// U+E000 is just above the surrogate range; both paths produce literal
    /// `"&#xE000;"`, not U+E000 char.
    ///
    /// html5ever peels `&amp;` → `&`, leaving `#xE000;` as text → `"&#xE000;"`.
    /// Body path: html_escape peels → `"&#xE000;"` → hex regex matches,
    /// cp=0xE000=57344 ∉ 0xD800..=0xDFFF → fallback verbatim `"&#xE000;"`.
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x and html-escape 0.2.x.
    #[test]
    fn scraper_heading_decode_html_entities_agree_on_above_surrogate_amp_escaped() {
        let html = "<html><body><h1>&amp;#xE000;</h1></body></html>";
        let (_, headings, _) = scraper_extract_all(html);
        assert_eq!(headings.len(), 1);
        let body_result = decode_html_entities("&amp;#xE000;");
        assert_eq!(headings[0].text, "&#xE000;",
            "html5ever must produce literal '&#xE000;' for &amp;#xE000;");
        assert_eq!(body_result, "&#xE000;",
            "body path must produce literal '&#xE000;' for &amp;#xE000;");
        assert_eq!(headings[0].text, body_result,
            "both paths must agree for above-surrogate amp-escaped form");
    }

    // ── S40-04: Fast-path identity (no `&#` → regex block skipped) ──────────

    /// Pins the `result.contains("&#")` guard's fast path: when the input
    /// contains no `&#` substring, the entire regex block is skipped and the
    /// string is returned unchanged.  This includes the empty-string edge case
    /// and strings with `&` but no `#`.
    #[test]
    fn decode_html_entities_fast_path_no_entity_marker() {
        // Empty string — must not panic and must return ""
        assert_eq!(decode_html_entities(""), "", "empty string must return empty");

        // ASCII-only — no `&` at all, fastest path
        assert_eq!(
            decode_html_entities("hello world"),
            "hello world",
            "plain ASCII must be returned unchanged"
        );

        // `&` present but no `#` following — `result.contains("&#")` is false;
        // regex block is skipped; html_escape may decode named entities here,
        // but no surrogate post-processing runs.
        assert_eq!(
            decode_html_entities("AT&T and D&D"),
            "AT&T and D&D",
            "bare & without # must not trigger regex post-processing"
        );

        // `&amp;` present (→ `&` after peel) but still no `&#` → guard false
        assert_eq!(
            decode_html_entities("&amp;T"),
            "&T",
            "&amp;T → &T via html_escape; no &#  → regex block skipped"
        );
    }

    // ── S41-01: Surrogate NCR in `href` attribute — html5ever attribute path ──

    /// html5ever processes character references in attribute values by the same
    /// rules as in text content.  A direct surrogate NCR in an `href` attribute
    /// (`&#xD800;`) is a parse error; html5ever replaces it with U+FFFD, so
    /// `Link::url` equals `"\u{FFFD}"`.  This is symmetric with the heading
    /// text path (S37-04).
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x (Cargo.toml).
    #[test]
    fn scraper_link_href_direct_surrogate_ncr_produces_fffd() {
        let html = r#"<html><body><a href="&#xD800;">text</a></body></html>"#;
        let (_, _, links) = scraper_extract_all(html);
        assert_eq!(links.len(), 1, "expected one link");
        assert_eq!(
            links[0].url,
            "\u{FFFD}",
            "direct &#xD800; in href: html5ever replaces surrogate NCR in attribute with U+FFFD"
        );
        assert_eq!(links[0].text, "text", "link text must be unaffected");
    }

    /// Decimal surrogate NCR in href attribute (`&#55296;` = U+D800).
    /// html5ever attribute path is symmetric with the hex form (S41-01a).
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x (Cargo.toml).
    #[test]
    fn scraper_link_href_decimal_surrogate_ncr_produces_fffd() {
        let html = r#"<html><body><a href="&#55296;">text</a></body></html>"#;
        let (_, _, links) = scraper_extract_all(html);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].url,
            "\u{FFFD}",
            "direct &#55296; in href: decimal surrogate NCR → U+FFFD via html5ever attribute path"
        );
    }

    /// `&amp;#xD800;` in href follows peel-once: html5ever decodes `&amp;` → `&`,
    /// leaving `#xD800;` without a leading `&` — not an entity — so `Link::url`
    /// is the literal string `"&#xD800;"`.  This is symmetric with heading text
    /// extraction (S37-04 diverge): both the href attribute path and the heading
    /// text path produce the same literal string for `&amp;`-escaped forms (AGREE).
    ///
    /// Verified against scraper 0.20.x / html5ever 0.27.x (Cargo.toml).
    #[test]
    fn scraper_link_href_amp_escaped_surrogate_produces_literal() {
        let html = r#"<html><body><a href="&amp;#xD800;">text</a></body></html>"#;
        let (_, _, links) = scraper_extract_all(html);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].url,
            "&#xD800;",
            "&amp;#xD800; in href: peel-once leaves literal &#xD800; (NOT U+D800 char)"
        );

        // Confirm AGREE with heading text for the same input form
        let heading_html = r#"<html><body><h1>&amp;#xD800;</h1></body></html>"#;
        let (_, headings, _) = scraper_extract_all(heading_html);
        assert_eq!(
            headings[0].text,
            "&#xD800;",
            "heading path: &amp;#xD800; in text → literal &#xD800;"
        );
        assert_eq!(
            links[0].url,
            headings[0].text,
            "href attribute and heading text: symmetric peel-once behavior (AGREE)"
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
