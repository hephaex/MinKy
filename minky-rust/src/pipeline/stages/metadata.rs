//! Metadata extraction stage
//!
//! Extracts document metadata including:
//! - Title refinement
//! - Date detection
//! - Author extraction
//! - Language detection

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::pipeline::{PipelineContext, PipelineResult, PipelineStage};

use super::parsing::ParsedDocument;

/// Document with extracted metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWithMetadata {
    /// Document title (refined)
    pub title: String,

    /// Plain text content
    pub plain_text: String,

    /// Original content
    pub original_content: String,

    /// MIME type
    pub mime_type: String,

    /// Extracted metadata
    pub metadata: ExtractedMetadata,

    /// Headings from parsing
    pub headings: Vec<super::parsing::Heading>,

    /// Links from parsing
    pub links: Vec<super::parsing::Link>,

    /// Code blocks from parsing
    pub code_blocks: Vec<super::parsing::CodeBlock>,

    /// Source information
    pub source_type: String,
    pub source_path: Option<String>,
}

/// Metadata extracted from the document
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedMetadata {
    /// Detected author
    pub author: Option<String>,

    /// Detected date
    pub date: Option<DateTime<Utc>>,

    /// Detected language (ISO 639-1 code)
    pub language: Option<String>,

    /// Word count
    pub word_count: usize,

    /// Character count
    pub char_count: usize,

    /// Reading time in minutes
    pub reading_time_minutes: u32,

    /// Detected document type
    pub doc_type: Option<DocumentType>,

    /// YAML frontmatter (if present)
    pub frontmatter: Option<serde_json::Value>,
}

/// Type of document detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Article,
    Tutorial,
    Reference,
    Notes,
    Code,
    Discussion,
    Announcement,
    Unknown,
}

/// Metadata extraction stage
#[derive(Debug, Clone, Default)]
pub struct MetadataExtractionStage;

impl MetadataExtractionStage {
    /// Create a new metadata extraction stage
    pub fn new() -> Self {
        Self
    }

    /// Extract YAML frontmatter from content
    fn extract_frontmatter(&self, content: &str) -> Option<serde_json::Value> {
        let frontmatter_re = Regex::new(r"(?s)^---\n(.+?)\n---").unwrap();

        frontmatter_re
            .captures(content)
            .and_then(|caps| caps.get(1))
            .and_then(|m| {
                // Parse YAML and convert to JSON
                // For simplicity, we'll just try to parse as JSON-like key-value pairs
                let yaml_str = m.as_str();
                self.parse_simple_yaml(yaml_str)
            })
    }

    /// Simple YAML-like parser for frontmatter
    fn parse_simple_yaml(&self, content: &str) -> Option<serde_json::Value> {
        let mut map = serde_json::Map::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                if !key.is_empty() {
                    map.insert(
                        key.to_string(),
                        serde_json::Value::String(value.to_string()),
                    );
                }
            }
        }

        if map.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(map))
        }
    }

    /// Extract author from content
    fn extract_author(&self, content: &str, frontmatter: &Option<serde_json::Value>) -> Option<String> {
        // Check frontmatter first
        if let Some(fm) = frontmatter {
            if let Some(author) = fm.get("author").and_then(|v| v.as_str()) {
                return Some(author.to_string());
            }
        }

        // Look for "Author: " pattern
        let author_re = Regex::new(r"(?i)(?:author|written by|by)[:\s]+([^\n]+)").unwrap();
        author_re
            .captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// Extract date from content
    fn extract_date(&self, content: &str, frontmatter: &Option<serde_json::Value>) -> Option<DateTime<Utc>> {
        // Check frontmatter first
        if let Some(fm) = frontmatter {
            if let Some(date_str) = fm.get("date").and_then(|v| v.as_str()) {
                if let Some(date) = self.parse_date(date_str) {
                    return Some(date);
                }
            }
        }

        // Look for date patterns in content
        let date_patterns = [
            r"(\d{4}-\d{2}-\d{2})",                    // 2024-01-15
            r"(\d{1,2}/\d{1,2}/\d{4})",               // 1/15/2024
            r"(\w+\s+\d{1,2},?\s+\d{4})",             // January 15, 2024
        ];

        for pattern in &date_patterns {
            let re = Regex::new(pattern).ok()?;
            if let Some(caps) = re.captures(content) {
                if let Some(m) = caps.get(1) {
                    if let Some(date) = self.parse_date(m.as_str()) {
                        return Some(date);
                    }
                }
            }
        }

        None
    }

    /// Parse a date string into DateTime<Utc>
    fn parse_date(&self, s: &str) -> Option<DateTime<Utc>> {
        // Try ISO format first
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc());
        }

        // Try US format
        if let Ok(date) = NaiveDate::parse_from_str(s, "%m/%d/%Y") {
            return date.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc());
        }

        None
    }

    /// Detect primary language
    fn detect_language(&self, content: &str) -> Option<String> {
        // Simple heuristic based on character frequency
        let korean_count = content.chars().filter(|c| (*c >= '\u{AC00}' && *c <= '\u{D7AF}') || (*c >= '\u{1100}' && *c <= '\u{11FF}')).count();
        let japanese_count = content.chars().filter(|c| (*c >= '\u{3040}' && *c <= '\u{30FF}') || (*c >= '\u{4E00}' && *c <= '\u{9FAF}')).count();
        let chinese_count = content.chars().filter(|c| *c >= '\u{4E00}' && *c <= '\u{9FAF}').count();
        let total_chars = content.chars().count();

        if total_chars == 0 {
            return None;
        }

        let korean_ratio = korean_count as f64 / total_chars as f64;
        let japanese_ratio = japanese_count as f64 / total_chars as f64;
        let chinese_ratio = chinese_count as f64 / total_chars as f64;

        if korean_ratio > 0.1 {
            return Some("ko".to_string());
        }
        if japanese_ratio > 0.1 && japanese_ratio > chinese_ratio {
            return Some("ja".to_string());
        }
        if chinese_ratio > 0.1 {
            return Some("zh".to_string());
        }

        // Default to English
        Some("en".to_string())
    }

    /// Detect document type based on content
    fn detect_document_type(&self, content: &str, headings: &[super::parsing::Heading]) -> DocumentType {
        let lower_content = content.to_lowercase();

        // Check for tutorial indicators
        if lower_content.contains("step 1") || lower_content.contains("how to") {
            return DocumentType::Tutorial;
        }

        // Check for reference/API doc indicators
        if lower_content.contains("api reference") || lower_content.contains("parameters:") {
            return DocumentType::Reference;
        }

        // Check for code-heavy content
        let code_keywords = ["fn ", "function ", "class ", "def ", "import ", "const ", "let "];
        let code_count: usize = code_keywords.iter().map(|k| lower_content.matches(k).count()).sum();
        if code_count > 5 {
            return DocumentType::Code;
        }

        // Check for article structure
        if !headings.is_empty() && content.len() > 1000 {
            return DocumentType::Article;
        }

        // Check for notes (short, informal)
        if content.len() < 500 {
            return DocumentType::Notes;
        }

        DocumentType::Unknown
    }

    /// Calculate word count
    fn word_count(&self, content: &str) -> usize {
        content.split_whitespace().count()
    }

    /// Calculate reading time in minutes
    fn reading_time(&self, word_count: usize) -> u32 {
        // Average reading speed: 200 words per minute
        ((word_count as f64) / 200.0).ceil() as u32
    }
}

#[async_trait]
impl PipelineStage<ParsedDocument, DocumentWithMetadata> for MetadataExtractionStage {
    fn name(&self) -> &'static str {
        "metadata_extraction"
    }

    async fn process(
        &self,
        input: ParsedDocument,
        context: &mut PipelineContext,
    ) -> PipelineResult<DocumentWithMetadata> {
        // Extract frontmatter from original content
        let frontmatter = self.extract_frontmatter(&input.original_content);

        // Extract metadata
        let author = self.extract_author(&input.plain_text, &frontmatter);
        let date = self.extract_date(&input.plain_text, &frontmatter);
        let language = self.detect_language(&input.plain_text);
        let word_count = self.word_count(&input.plain_text);
        let char_count = input.plain_text.len();
        let reading_time_minutes = self.reading_time(word_count);
        let doc_type = self.detect_document_type(&input.plain_text, &input.headings);

        // Refine title from frontmatter if available
        let title = frontmatter
            .as_ref()
            .and_then(|fm| fm.get("title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| input.title.clone());

        let metadata = ExtractedMetadata {
            author,
            date,
            language: language.clone(),
            word_count,
            char_count,
            reading_time_minutes,
            doc_type: Some(doc_type),
            frontmatter,
        };

        // Update context
        context.set_metadata("word_count", word_count);
        context.set_metadata("char_count", char_count);
        if let Some(lang) = &metadata.language {
            context.set_metadata("language", lang);
        }

        Ok(DocumentWithMetadata {
            title,
            plain_text: input.plain_text,
            original_content: input.original_content,
            mime_type: input.mime_type,
            metadata,
            headings: input.headings,
            links: input.links,
            code_blocks: input.code_blocks,
            source_type: input.source_type,
            source_path: input.source_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parsed_doc(content: &str) -> ParsedDocument {
        ParsedDocument {
            title: "Test".to_string(),
            plain_text: content.to_string(),
            original_content: content.to_string(),
            mime_type: "text/plain".to_string(),
            headings: Vec::new(),
            links: Vec::new(),
            code_blocks: Vec::new(),
            source_type: "test".to_string(),
            source_path: None,
        }
    }

    #[tokio::test]
    async fn test_extract_word_count() {
        let stage = MetadataExtractionStage::new();
        let mut context = PipelineContext::new();

        let doc = make_parsed_doc("This is a test document with some words.");
        let result = stage.process(doc, &mut context).await.unwrap();

        assert_eq!(result.metadata.word_count, 8);
    }

    #[tokio::test]
    async fn test_extract_reading_time() {
        let stage = MetadataExtractionStage::new();
        let mut context = PipelineContext::new();

        // 400 words should be ~2 minutes
        let words = vec!["word"; 400].join(" ");
        let doc = make_parsed_doc(&words);
        let result = stage.process(doc, &mut context).await.unwrap();

        assert_eq!(result.metadata.reading_time_minutes, 2);
    }

    #[test]
    fn test_extract_frontmatter() {
        let stage = MetadataExtractionStage::new();
        let content = r#"---
title: My Document
author: John Doe
date: 2024-01-15
---

Content here."#;

        let fm = stage.extract_frontmatter(content);
        assert!(fm.is_some());

        let fm = fm.unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("My Document"));
        assert_eq!(fm.get("author").and_then(|v| v.as_str()), Some("John Doe"));
    }

    #[test]
    fn test_detect_korean() {
        let stage = MetadataExtractionStage::new();
        let lang = stage.detect_language("안녕하세요, 이것은 한국어 문서입니다.");
        assert_eq!(lang, Some("ko".to_string()));
    }

    #[test]
    fn test_detect_english() {
        let stage = MetadataExtractionStage::new();
        let lang = stage.detect_language("This is an English document.");
        assert_eq!(lang, Some("en".to_string()));
    }

    #[test]
    fn test_detect_tutorial_type() {
        let stage = MetadataExtractionStage::new();
        let doc_type = stage.detect_document_type("Step 1: Install the package\nStep 2: Configure", &[]);
        assert_eq!(doc_type, DocumentType::Tutorial);
    }
}
