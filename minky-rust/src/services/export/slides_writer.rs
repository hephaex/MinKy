use anyhow::Result;
use regex::Regex;

use crate::models::ExportTheme;
use crate::services::export::HtmlRenderer;

/// Slides writer for generating reveal.js presentations
pub struct SlidesWriter {
    html_renderer: HtmlRenderer,
    config: RevealConfig,
}

impl Default for SlidesWriter {
    fn default() -> Self {
        Self::new(ExportTheme::light(), RevealConfig::default())
    }
}

impl SlidesWriter {
    /// Create a new slides writer
    pub fn new(theme: ExportTheme, config: RevealConfig) -> Self {
        Self {
            html_renderer: HtmlRenderer::without_wrapper(theme),
            config,
        }
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: ExportTheme) -> Self {
        self.html_renderer = HtmlRenderer::without_wrapper(theme);
        self
    }

    /// Set reveal.js configuration
    pub fn with_config(mut self, config: RevealConfig) -> Self {
        self.config = config;
        self
    }

    /// Render Markdown to reveal.js presentation
    pub fn render(&self, markdown: &str) -> Result<String> {
        let slides = self.split_slides(markdown);
        let slides_html = self.render_slides(&slides)?;
        Ok(self.wrap_reveal(&slides_html))
    }

    /// Render with custom title
    pub fn render_with_title(&self, markdown: &str, title: &str) -> Result<String> {
        let slides = self.split_slides(markdown);
        let slides_html = self.render_slides(&slides)?;
        Ok(self.wrap_reveal_with_title(&slides_html, title))
    }

    /// Split markdown into individual slides
    fn split_slides(&self, markdown: &str) -> Vec<String> {
        // Split on --- (horizontal rule) or <!-- slide --> comment
        let separator = Regex::new(r"(?m)^---$|<!--\s*slide\s*-->").unwrap();

        separator
            .split(markdown)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Render individual slides to HTML
    fn render_slides(&self, slides: &[String]) -> Result<String> {
        let mut html = String::new();

        for slide in slides {
            // Check for vertical slides (sections within a slide)
            let vertical_sep = Regex::new(r"(?m)^--$|<!--\s*vertical\s*-->").unwrap();

            if vertical_sep.is_match(slide) {
                // Vertical slide group
                html.push_str("<section>\n");

                for vertical in vertical_sep.split(slide) {
                    let vertical = vertical.trim();
                    if !vertical.is_empty() {
                        let slide_html = self.render_single_slide(vertical)?;
                        html.push_str("<section>\n");
                        html.push_str(&slide_html);
                        html.push_str("\n</section>\n");
                    }
                }

                html.push_str("</section>\n");
            } else {
                // Regular horizontal slide
                let slide_html = self.render_single_slide(slide)?;
                html.push_str("<section>\n");
                html.push_str(&slide_html);
                html.push_str("\n</section>\n");
            }
        }

        Ok(html)
    }

    /// Render a single slide
    fn render_single_slide(&self, markdown: &str) -> Result<String> {
        // Check for slide attributes (e.g., <!-- .slide: data-background="#fff" -->)
        let attr_regex = Regex::new(r"<!--\s*\.slide:\s*([^>]+)\s*-->").unwrap();
        let mut content = markdown.to_string();

        if let Some(_cap) = attr_regex.captures(markdown) {
            // Note: slide attributes (data-background, etc.) can be extracted here if needed
            content = attr_regex.replace(markdown, "").to_string();
        }

        // Check for speaker notes
        let notes_regex = Regex::new(r"(?ms)Note:\s*(.*)$").unwrap();
        let mut notes = String::new();

        if let Some(cap) = notes_regex.captures(&content) {
            notes = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            content = notes_regex.replace(&content, "").to_string();
        }

        let html = self.html_renderer.render(&content.trim())?;

        let mut result = html;

        // Add speaker notes if present
        if !notes.is_empty() {
            result.push_str(&format!("<aside class=\"notes\">{}</aside>", notes.trim()));
        }

        Ok(result)
    }

    /// Wrap slides in reveal.js HTML
    fn wrap_reveal(&self, slides_html: &str) -> String {
        self.wrap_reveal_with_title(slides_html, "Presentation")
    }

    /// Wrap slides with custom title
    fn wrap_reveal_with_title(&self, slides_html: &str, title: &str) -> String {
        let reveal_theme = match self.config.theme.as_str() {
            "black" | "white" | "league" | "beige" | "sky" | "night" | "serif" | "simple"
            | "solarized" | "moon" | "dracula" => self.config.theme.clone(),
            _ => "white".to_string(),
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@4/dist/reset.css">
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@4/dist/reveal.css">
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@4/dist/theme/{theme}.css">
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/reveal.js@4/plugin/highlight/monokai.css">
    <style>
        .reveal section {{
            text-align: left;
        }}
        .reveal h1, .reveal h2, .reveal h3 {{
            text-transform: none;
        }}
        .reveal pre {{
            width: 100%;
            font-size: 0.55em;
        }}
        .reveal code {{
            background: rgba(0,0,0,0.1);
            padding: 2px 6px;
            border-radius: 4px;
        }}
        .reveal pre code {{
            background: none;
            padding: 0;
        }}
        .reveal img {{
            max-height: 60vh;
        }}
        .reveal blockquote {{
            width: 90%;
        }}
    </style>
</head>
<body>
    <div class="reveal">
        <div class="slides">
{slides}
        </div>
    </div>

    <script src="https://cdn.jsdelivr.net/npm/reveal.js@4/dist/reveal.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/reveal.js@4/plugin/notes/notes.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/reveal.js@4/plugin/markdown/markdown.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/reveal.js@4/plugin/highlight/highlight.js"></script>
    <script>
        Reveal.initialize({{
            hash: {hash},
            controls: {controls},
            progress: {progress},
            center: {center},
            slideNumber: {slide_number},
            transition: '{transition}',
            plugins: [ RevealNotes, RevealHighlight ]
        }});
    </script>
</body>
</html>"#,
            title = html_escape(title),
            theme = reveal_theme,
            slides = slides_html,
            hash = self.config.hash,
            controls = self.config.controls,
            progress = self.config.progress,
            center = self.config.center,
            slide_number = self.config.slide_number,
            transition = self.config.transition,
        )
    }
}

/// reveal.js configuration options
#[derive(Debug, Clone)]
pub struct RevealConfig {
    /// Reveal.js theme (black, white, league, beige, etc.)
    pub theme: String,
    /// Show navigation controls
    pub controls: bool,
    /// Show progress bar
    pub progress: bool,
    /// Center slides vertically
    pub center: bool,
    /// Enable URL hash navigation
    pub hash: bool,
    /// Show slide numbers
    pub slide_number: bool,
    /// Transition style (none, fade, slide, convex, concave, zoom)
    pub transition: String,
}

impl Default for RevealConfig {
    fn default() -> Self {
        Self {
            theme: "white".to_string(),
            controls: true,
            progress: true,
            center: true,
            hash: true,
            slide_number: false,
            transition: "slide".to_string(),
        }
    }
}

impl RevealConfig {
    /// Create config for a minimal presentation
    pub fn minimal() -> Self {
        Self {
            theme: "simple".to_string(),
            controls: false,
            progress: false,
            center: true,
            hash: true,
            slide_number: false,
            transition: "fade".to_string(),
        }
    }

    /// Create config for a dark presentation
    pub fn dark() -> Self {
        Self {
            theme: "black".to_string(),
            controls: true,
            progress: true,
            center: true,
            hash: true,
            slide_number: true,
            transition: "slide".to_string(),
        }
    }
}

/// Escape HTML special characters
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Slides export options
#[derive(Debug, Clone, Default)]
pub struct SlidesExportOptions {
    pub theme: Option<String>,
    pub reveal_theme: Option<String>,
    pub transition: Option<String>,
    pub show_controls: Option<bool>,
    pub show_progress: Option<bool>,
    pub center: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_slides_horizontal_rule() {
        let writer = SlidesWriter::default();
        let markdown = "Slide 1\n---\nSlide 2\n---\nSlide 3";
        let slides = writer.split_slides(markdown);

        assert_eq!(slides.len(), 3);
        assert_eq!(slides[0], "Slide 1");
        assert_eq!(slides[1], "Slide 2");
        assert_eq!(slides[2], "Slide 3");
    }

    #[test]
    fn test_split_slides_comment() {
        let writer = SlidesWriter::default();
        let markdown = "Slide 1\n<!-- slide -->\nSlide 2";
        let slides = writer.split_slides(markdown);

        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_render_basic_slides() {
        let writer = SlidesWriter::default();
        let markdown = "# Title\n---\n## Content\n\nSome text";
        let html = writer.render(markdown).unwrap();

        assert!(html.contains("reveal.js"));
        assert!(html.contains("<section>"));
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<h2>Content</h2>"));
    }

    #[test]
    fn test_render_with_title() {
        let writer = SlidesWriter::default();
        let html = writer
            .render_with_title("# Slide", "My Presentation")
            .unwrap();

        assert!(html.contains("<title>My Presentation</title>"));
    }

    #[test]
    fn test_reveal_config_default() {
        let config = RevealConfig::default();
        assert_eq!(config.theme, "white");
        assert!(config.controls);
        assert!(config.progress);
    }

    #[test]
    fn test_reveal_config_minimal() {
        let config = RevealConfig::minimal();
        assert_eq!(config.theme, "simple");
        assert!(!config.controls);
        assert!(!config.progress);
    }

    #[test]
    fn test_reveal_config_dark() {
        let config = RevealConfig::dark();
        assert_eq!(config.theme, "black");
        assert!(config.slide_number);
    }

    #[test]
    fn test_with_config() {
        let config = RevealConfig {
            theme: "night".to_string(),
            ..Default::default()
        };
        let writer = SlidesWriter::default().with_config(config);
        let html = writer.render("# Test").unwrap();

        assert!(html.contains("theme/night.css"));
    }

    #[test]
    fn test_speaker_notes() {
        let writer = SlidesWriter::default();
        let markdown = "# Title\n\nContent\n\nNote: These are speaker notes";
        let html = writer.render(markdown).unwrap();

        assert!(html.contains("aside class=\"notes\""));
        assert!(html.contains("speaker notes"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_empty_slides_filtered() {
        let writer = SlidesWriter::default();
        let markdown = "Slide 1\n---\n\n---\nSlide 2";
        let slides = writer.split_slides(markdown);

        assert_eq!(slides.len(), 2);
    }
}
