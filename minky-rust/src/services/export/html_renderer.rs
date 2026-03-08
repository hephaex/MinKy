use anyhow::Result;
use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::models::ExportTheme;

/// HTML renderer for Markdown documents
pub struct HtmlRenderer {
    theme: ExportTheme,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    include_wrapper: bool,
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new(ExportTheme::light())
    }
}

impl HtmlRenderer {
    /// Create a new HTML renderer with the specified theme
    pub fn new(theme: ExportTheme) -> Self {
        Self {
            theme,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            include_wrapper: true,
        }
    }

    /// Create a renderer without wrapper (for embedding)
    pub fn without_wrapper(theme: ExportTheme) -> Self {
        Self {
            theme,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            include_wrapper: false,
        }
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: ExportTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Render Markdown content to HTML
    pub fn render(&self, markdown: &str) -> Result<String> {
        let content_html = self.render_markdown(markdown)?;

        if self.include_wrapper {
            Ok(self.wrap_html(&content_html))
        } else {
            Ok(content_html)
        }
    }

    /// Render Markdown to HTML content (without wrapper)
    fn render_markdown(&self, markdown: &str) -> Result<String> {
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        // Collect events and handle code blocks with syntax highlighting
        let mut events: Vec<Event> = Vec::new();
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();

        for event in parser {
            match &event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    code_content.clear();
                }
                Event::End(Tag::CodeBlock(_)) => {
                    in_code_block = false;

                    // Generate highlighted HTML
                    let highlighted = self.highlight_code(&code_content, &code_lang);
                    events.push(Event::Html(highlighted.into()));
                }
                Event::Text(text) if in_code_block => {
                    code_content.push_str(text);
                }
                _ => {
                    if !in_code_block {
                        events.push(event);
                    }
                }
            }
        }

        let mut html_output = String::new();
        html::push_html(&mut html_output, events.into_iter());

        Ok(html_output)
    }

    /// Highlight code with syntax highlighting
    fn highlight_code(&self, code: &str, lang: &str) -> String {
        // Find syntax for language
        let syntax = if lang.is_empty() {
            self.syntax_set.find_syntax_plain_text()
        } else {
            self.syntax_set
                .find_syntax_by_token(lang)
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        // Determine highlight theme based on export theme
        let highlight_theme_name = if self.theme.name.contains("dark") {
            "base16-ocean.dark"
        } else {
            "base16-ocean.light"
        };

        let highlight_theme = self
            .theme_set
            .themes
            .get(highlight_theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("No themes available")
            });

        // Generate highlighted HTML
        match highlighted_html_for_string(code, &self.syntax_set, syntax, highlight_theme) {
            Ok(highlighted) => {
                format!(
                    "<div class=\"code-block\" data-language=\"{}\"><pre>{}</pre></div>",
                    lang, highlighted
                )
            }
            Err(_) => {
                // Fallback to plain code block
                format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>",
                    lang,
                    html_escape(code)
                )
            }
        }
    }

    /// Wrap content in full HTML document
    fn wrap_html(&self, content: &str) -> String {
        let fonts_link = if !self.theme.fonts.is_empty() {
            let font_families = self
                .theme
                .fonts
                .iter()
                .filter(|f| !f.contains("system") && !f.contains("serif") && !f.contains("sans"))
                .map(|f| f.replace(' ', "+"))
                .collect::<Vec<_>>()
                .join("|");

            if !font_families.is_empty() {
                format!(
                    r#"<link href="https://fonts.googleapis.com/css2?family={}&display=swap" rel="stylesheet">"#,
                    font_families
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let font_family = self.theme.fonts.join(", ");

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Document</title>
    {}
    <style>
        :root {{
            --font-family: {};
        }}
        {}
    </style>
</head>
<body>
    <article class="document">
        {}
    </article>
</body>
</html>"#,
            fonts_link, font_family, self.theme.css, content
        )
    }

    /// Render with custom title
    pub fn render_with_title(&self, markdown: &str, title: &str) -> Result<String> {
        let content_html = self.render_markdown(markdown)?;

        if self.include_wrapper {
            Ok(self.wrap_html_with_title(&content_html, title))
        } else {
            Ok(content_html)
        }
    }

    /// Wrap content with custom title
    fn wrap_html_with_title(&self, content: &str, title: &str) -> String {
        let fonts_link = if !self.theme.fonts.is_empty() {
            let font_families = self
                .theme
                .fonts
                .iter()
                .filter(|f| !f.contains("system") && !f.contains("serif") && !f.contains("sans"))
                .map(|f| f.replace(' ', "+"))
                .collect::<Vec<_>>()
                .join("|");

            if !font_families.is_empty() {
                format!(
                    r#"<link href="https://fonts.googleapis.com/css2?family={}&display=swap" rel="stylesheet">"#,
                    font_families
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let font_family = self.theme.fonts.join(", ");

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    {}
    <style>
        :root {{
            --font-family: {};
        }}
        {}
    </style>
</head>
<body>
    <article class="document">
        {}
    </article>
</body>
</html>"#,
            html_escape(title),
            fonts_link,
            font_family,
            self.theme.css,
            content
        )
    }

    /// Get current theme name
    pub fn theme_name(&self) -> &str {
        &self.theme.name
    }
}

/// Escape HTML special characters
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// HTML export options
#[derive(Debug, Clone, Default)]
pub struct HtmlExportOptions {
    pub theme: Option<String>,
    pub include_toc: bool,
    pub standalone: bool,
    pub custom_css: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_markdown() {
        let renderer = HtmlRenderer::default();
        let html = renderer.render("# Hello World").unwrap();

        assert!(html.contains("<h1>Hello World</h1>"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_render_without_wrapper() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let html = renderer.render("# Hello World").unwrap();

        assert!(html.contains("<h1>Hello World</h1>"));
        assert!(!html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_render_code_block() {
        let renderer = HtmlRenderer::default();
        let markdown = r#"
```rust
fn main() {
    println!("Hello");
}
```
"#;
        let html = renderer.render(markdown).unwrap();

        // Should have syntax highlighting
        assert!(html.contains("code-block") || html.contains("pre"));
    }

    #[test]
    fn test_render_with_dark_theme() {
        let renderer = HtmlRenderer::new(ExportTheme::dark());
        let html = renderer.render("# Dark Theme").unwrap();

        assert!(html.contains("#1a1a2e")); // Dark background color
    }

    #[test]
    fn test_render_with_title() {
        let renderer = HtmlRenderer::default();
        let html = renderer
            .render_with_title("# Content", "My Document")
            .unwrap();

        assert!(html.contains("<title>My Document</title>"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quote\""), "&quot;quote&quot;");
    }

    #[test]
    fn test_render_paragraph() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let html = renderer.render("Hello world").unwrap();

        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn test_render_list() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let html = renderer.render(markdown).unwrap();

        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_render_link() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let html = renderer.render("[Link](https://example.com)").unwrap();

        assert!(html.contains("<a href=\"https://example.com\">Link</a>"));
    }

    #[test]
    fn test_render_image() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let html = renderer.render("![Alt](image.png)").unwrap();

        assert!(html.contains("<img"));
        assert!(html.contains("src=\"image.png\""));
        assert!(html.contains("alt=\"Alt\""));
    }

    #[test]
    fn test_render_table() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let markdown = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = renderer.render(markdown).unwrap();

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn test_render_blockquote() {
        let renderer = HtmlRenderer::without_wrapper(ExportTheme::light());
        let html = renderer.render("> Quote").unwrap();

        assert!(html.contains("<blockquote>"));
    }

    #[test]
    fn test_theme_name() {
        let renderer = HtmlRenderer::new(ExportTheme::academic());
        assert_eq!(renderer.theme_name(), "academic");
    }

    #[test]
    fn test_with_theme() {
        let renderer = HtmlRenderer::default().with_theme(ExportTheme::minimal());
        assert_eq!(renderer.theme_name(), "minimal");
    }
}
