use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::time::Duration;

use headless_chrome::{Browser, LaunchOptions};

use crate::models::ExportTheme;
use crate::services::export::HtmlRenderer;

/// PDF generation engine
#[derive(Debug, Clone)]
pub enum PdfEngine {
    /// Headless Chrome/Chromium
    Chromium { path: Option<PathBuf> },
}

impl Default for PdfEngine {
    fn default() -> Self {
        Self::Chromium { path: None }
    }
}

/// PDF writer for exporting documents
pub struct PdfWriter {
    html_renderer: HtmlRenderer,
    engine: PdfEngine,
}

impl Default for PdfWriter {
    fn default() -> Self {
        Self::new(ExportTheme::light(), PdfEngine::default())
    }
}

impl PdfWriter {
    /// Create a new PDF writer
    pub fn new(theme: ExportTheme, engine: PdfEngine) -> Self {
        Self {
            html_renderer: HtmlRenderer::new(theme),
            engine,
        }
    }

    /// Create with custom HTML renderer
    pub fn with_renderer(html_renderer: HtmlRenderer, engine: PdfEngine) -> Self {
        Self {
            html_renderer,
            engine,
        }
    }

    /// Generate PDF from Markdown content
    pub fn render(&self, markdown: &str) -> Result<Vec<u8>> {
        let html = self.html_renderer.render(markdown)?;
        self.html_to_pdf(&html)
    }

    /// Generate PDF from Markdown with title
    pub fn render_with_title(&self, markdown: &str, title: &str) -> Result<Vec<u8>> {
        let html = self.html_renderer.render_with_title(markdown, title)?;
        self.html_to_pdf(&html)
    }

    /// Generate PDF from raw HTML
    pub fn html_to_pdf(&self, html: &str) -> Result<Vec<u8>> {
        match &self.engine {
            PdfEngine::Chromium { path } => self.chromium_to_pdf(html, path.as_ref()),
        }
    }

    /// Use headless Chrome to generate PDF
    fn chromium_to_pdf(&self, html: &str, chrome_path: Option<&PathBuf>) -> Result<Vec<u8>> {
        let mut builder = LaunchOptions::default_builder();

        if let Some(path) = chrome_path {
            builder.path(Some(path.clone()));
        }

        // Set headless mode and sandbox options
        builder
            .headless(true)
            .sandbox(false)
            .idle_browser_timeout(Duration::from_secs(60));

        let options = builder
            .build()
            .map_err(|e| anyhow!("Failed to build launch options: {}", e))?;

        let browser =
            Browser::new(options).map_err(|e| anyhow!("Failed to launch browser: {}", e))?;

        let tab = browser
            .new_tab()
            .map_err(|e| anyhow!("Failed to create tab: {}", e))?;

        // Navigate to data URL with HTML content
        let data_url = format!("data:text/html;charset=utf-8,{}", urlencoding(html));

        tab.navigate_to(&data_url)
            .map_err(|e| anyhow!("Failed to navigate: {}", e))?;

        tab.wait_until_navigated()
            .map_err(|e| anyhow!("Navigation timeout: {}", e))?;

        // Wait for content to render
        std::thread::sleep(Duration::from_millis(500));

        // Print to PDF
        let pdf_options = headless_chrome::types::PrintToPdfOptions {
            landscape: Some(false),
            display_header_footer: Some(false),
            print_background: Some(true),
            scale: Some(1.0),
            paper_width: Some(8.5),  // Letter size
            paper_height: Some(11.0),
            margin_top: Some(0.5),
            margin_bottom: Some(0.5),
            margin_left: Some(0.5),
            margin_right: Some(0.5),
            ..Default::default()
        };

        let pdf_data = tab
            .print_to_pdf(Some(pdf_options))
            .map_err(|e| anyhow!("Failed to generate PDF: {}", e))?;

        Ok(pdf_data)
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: ExportTheme) -> Self {
        self.html_renderer = HtmlRenderer::new(theme);
        self
    }
}

/// URL-encode HTML content for data URL
fn urlencoding(html: &str) -> String {
    html.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '"' => "%22".to_string(),
            '#' => "%23".to_string(),
            '%' => "%25".to_string(),
            '{' => "%7B".to_string(),
            '}' => "%7D".to_string(),
            '|' => "%7C".to_string(),
            '\\' => "%5C".to_string(),
            '^' => "%5E".to_string(),
            '[' => "%5B".to_string(),
            ']' => "%5D".to_string(),
            '`' => "%60".to_string(),
            '\n' => "%0A".to_string(),
            '\r' => "%0D".to_string(),
            '\t' => "%09".to_string(),
            c if c.is_ascii_alphanumeric() || "-_.~!*'();:@&=+$,/?".contains(c) => c.to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}

/// PDF export options
#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub theme: Option<String>,
    pub paper_size: PaperSize,
    pub orientation: Orientation,
    pub margins: Margins,
    pub include_toc: bool,
    pub page_numbers: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            theme: None,
            paper_size: PaperSize::Letter,
            orientation: Orientation::Portrait,
            margins: Margins::default(),
            include_toc: false,
            page_numbers: false,
        }
    }
}

/// Paper sizes
#[derive(Debug, Clone, Copy)]
pub enum PaperSize {
    Letter,  // 8.5 x 11 inches
    A4,      // 210 x 297 mm
    A5,      // 148 x 210 mm
    Legal,   // 8.5 x 14 inches
}

impl PaperSize {
    pub fn dimensions(&self) -> (f64, f64) {
        match self {
            Self::Letter => (8.5, 11.0),
            Self::A4 => (8.27, 11.69),
            Self::A5 => (5.83, 8.27),
            Self::Legal => (8.5, 14.0),
        }
    }
}

/// Page orientation
#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Page margins in inches
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 0.5,
            bottom: 0.5,
            left: 0.5,
            right: 0.5,
        }
    }
}

impl Margins {
    pub fn none() -> Self {
        Self {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
        }
    }

    pub fn small() -> Self {
        Self {
            top: 0.25,
            bottom: 0.25,
            left: 0.25,
            right: 0.25,
        }
    }

    pub fn large() -> Self {
        Self {
            top: 1.0,
            bottom: 1.0,
            left: 1.0,
            right: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_engine_default() {
        let engine = PdfEngine::default();
        assert!(matches!(engine, PdfEngine::Chromium { path: None }));
    }

    #[test]
    fn test_paper_size_letter() {
        let (w, h) = PaperSize::Letter.dimensions();
        assert_eq!(w, 8.5);
        assert_eq!(h, 11.0);
    }

    #[test]
    fn test_paper_size_a4() {
        let (w, h) = PaperSize::A4.dimensions();
        assert!((w - 8.27).abs() < 0.01);
        assert!((h - 11.69).abs() < 0.01);
    }

    #[test]
    fn test_margins_default() {
        let margins = Margins::default();
        assert_eq!(margins.top, 0.5);
        assert_eq!(margins.bottom, 0.5);
    }

    #[test]
    fn test_margins_none() {
        let margins = Margins::none();
        assert_eq!(margins.top, 0.0);
        assert_eq!(margins.left, 0.0);
    }

    #[test]
    fn test_urlencoding_basic() {
        assert_eq!(urlencoding("hello"), "hello");
        assert_eq!(urlencoding("hello world"), "hello%20world");
        assert!(urlencoding("<html>").contains("%3C"));
    }

    #[test]
    fn test_pdf_export_options_default() {
        let options = PdfExportOptions::default();
        assert!(options.theme.is_none());
        assert!(!options.include_toc);
        assert!(!options.page_numbers);
    }

    // Note: Actual PDF generation tests require Chrome/Chromium installed
    // They are marked as ignored for CI environments
    #[test]
    #[ignore]
    fn test_pdf_writer_render() {
        let writer = PdfWriter::default();
        let pdf = writer.render("# Hello PDF").unwrap();
        assert!(!pdf.is_empty());
        // PDF files start with %PDF
        assert!(pdf.starts_with(b"%PDF"));
    }
}
