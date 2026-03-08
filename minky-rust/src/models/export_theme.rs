use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Export theme for HTML/PDF/Slides rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTheme {
    pub id: Uuid,
    pub name: String,
    pub css: String,
    pub fonts: Vec<String>,
    pub is_builtin: bool,
    pub created_at: DateTime<Utc>,
}

impl ExportTheme {
    /// Light theme - clean and readable
    pub fn light() -> Self {
        Self {
            id: Uuid::nil(),
            name: "light".to_string(),
            css: LIGHT_THEME_CSS.to_string(),
            fonts: vec![
                "Inter".to_string(),
                "system-ui".to_string(),
                "sans-serif".to_string(),
            ],
            is_builtin: true,
            created_at: Utc::now(),
        }
    }

    /// Dark theme - easy on the eyes
    pub fn dark() -> Self {
        Self {
            id: Uuid::nil(),
            name: "dark".to_string(),
            css: DARK_THEME_CSS.to_string(),
            fonts: vec![
                "Inter".to_string(),
                "system-ui".to_string(),
                "sans-serif".to_string(),
            ],
            is_builtin: true,
            created_at: Utc::now(),
        }
    }

    /// Academic theme - formal documents
    pub fn academic() -> Self {
        Self {
            id: Uuid::nil(),
            name: "academic".to_string(),
            css: ACADEMIC_THEME_CSS.to_string(),
            fonts: vec![
                "Georgia".to_string(),
                "Times New Roman".to_string(),
                "serif".to_string(),
            ],
            is_builtin: true,
            created_at: Utc::now(),
        }
    }

    /// Minimal theme - distraction-free
    pub fn minimal() -> Self {
        Self {
            id: Uuid::nil(),
            name: "minimal".to_string(),
            css: MINIMAL_THEME_CSS.to_string(),
            fonts: vec![
                "system-ui".to_string(),
                "sans-serif".to_string(),
            ],
            is_builtin: true,
            created_at: Utc::now(),
        }
    }

    /// Get all built-in themes
    pub fn builtin_themes() -> Vec<Self> {
        vec![
            Self::light(),
            Self::dark(),
            Self::academic(),
            Self::minimal(),
        ]
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "light" => Some(Self::light()),
            "dark" => Some(Self::dark()),
            "academic" => Some(Self::academic()),
            "minimal" => Some(Self::minimal()),
            _ => None,
        }
    }
}

/// Request to create a custom theme
#[derive(Debug, Deserialize)]
pub struct CreateThemeRequest {
    pub name: String,
    pub css: String,
    pub fonts: Option<Vec<String>>,
}

/// Request to update a theme
#[derive(Debug, Deserialize)]
pub struct UpdateThemeRequest {
    pub name: Option<String>,
    pub css: Option<String>,
    pub fonts: Option<Vec<String>>,
}

/// Theme summary for API responses
#[derive(Debug, Serialize)]
pub struct ThemeSummary {
    pub id: Uuid,
    pub name: String,
    pub is_builtin: bool,
    pub font_count: usize,
}

impl From<&ExportTheme> for ThemeSummary {
    fn from(theme: &ExportTheme) -> Self {
        Self {
            id: theme.id,
            name: theme.name.clone(),
            is_builtin: theme.is_builtin,
            font_count: theme.fonts.len(),
        }
    }
}

// Built-in CSS themes

const LIGHT_THEME_CSS: &str = r#"
:root {
    --bg-color: #ffffff;
    --text-color: #1a1a1a;
    --heading-color: #0f0f0f;
    --link-color: #2563eb;
    --code-bg: #f3f4f6;
    --border-color: #e5e7eb;
    --blockquote-bg: #f9fafb;
}

body {
    font-family: var(--font-family, 'Inter', system-ui, sans-serif);
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.7;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin-top: 2rem;
    margin-bottom: 1rem;
    font-weight: 600;
}

h1 { font-size: 2.25rem; border-bottom: 2px solid var(--border-color); padding-bottom: 0.5rem; }
h2 { font-size: 1.75rem; }
h3 { font-size: 1.5rem; }

a { color: var(--link-color); text-decoration: none; }
a:hover { text-decoration: underline; }

code {
    background-color: var(--code-bg);
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    font-size: 0.9em;
}

pre {
    background-color: var(--code-bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
}

pre code {
    background: none;
    padding: 0;
}

blockquote {
    background-color: var(--blockquote-bg);
    border-left: 4px solid var(--link-color);
    margin: 1rem 0;
    padding: 1rem;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
}

th, td {
    border: 1px solid var(--border-color);
    padding: 0.75rem;
    text-align: left;
}

th {
    background-color: var(--code-bg);
    font-weight: 600;
}

img {
    max-width: 100%;
    height: auto;
}
"#;

const DARK_THEME_CSS: &str = r#"
:root {
    --bg-color: #1a1a2e;
    --text-color: #e4e4e7;
    --heading-color: #ffffff;
    --link-color: #60a5fa;
    --code-bg: #27273a;
    --border-color: #3f3f5a;
    --blockquote-bg: #232338;
}

body {
    font-family: var(--font-family, 'Inter', system-ui, sans-serif);
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.7;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin-top: 2rem;
    margin-bottom: 1rem;
    font-weight: 600;
}

h1 { font-size: 2.25rem; border-bottom: 2px solid var(--border-color); padding-bottom: 0.5rem; }
h2 { font-size: 1.75rem; }
h3 { font-size: 1.5rem; }

a { color: var(--link-color); text-decoration: none; }
a:hover { text-decoration: underline; }

code {
    background-color: var(--code-bg);
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    font-size: 0.9em;
}

pre {
    background-color: var(--code-bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
}

pre code {
    background: none;
    padding: 0;
}

blockquote {
    background-color: var(--blockquote-bg);
    border-left: 4px solid var(--link-color);
    margin: 1rem 0;
    padding: 1rem;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin: 1rem 0;
}

th, td {
    border: 1px solid var(--border-color);
    padding: 0.75rem;
    text-align: left;
}

th {
    background-color: var(--code-bg);
    font-weight: 600;
}

img {
    max-width: 100%;
    height: auto;
}
"#;

const ACADEMIC_THEME_CSS: &str = r#"
:root {
    --bg-color: #fffef8;
    --text-color: #2d2d2d;
    --heading-color: #1a1a1a;
    --link-color: #8b0000;
    --code-bg: #f5f5f0;
    --border-color: #d4d4c8;
    --blockquote-bg: #fafaf5;
}

body {
    font-family: var(--font-family, 'Georgia', 'Times New Roman', serif);
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.8;
    max-width: 700px;
    margin: 0 auto;
    padding: 3rem 2rem;
    text-align: justify;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin-top: 2.5rem;
    margin-bottom: 1rem;
    font-weight: normal;
    text-align: left;
}

h1 {
    font-size: 2rem;
    text-align: center;
    margin-bottom: 2rem;
}

h2 {
    font-size: 1.5rem;
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 0.25rem;
}

h3 { font-size: 1.25rem; font-style: italic; }

a { color: var(--link-color); text-decoration: none; }
a:hover { text-decoration: underline; }

code {
    background-color: var(--code-bg);
    padding: 0.15rem 0.3rem;
    border-radius: 2px;
    font-size: 0.85em;
    font-family: 'Courier New', monospace;
}

pre {
    background-color: var(--code-bg);
    padding: 1rem;
    border: 1px solid var(--border-color);
    overflow-x: auto;
}

blockquote {
    font-style: italic;
    border-left: 3px solid var(--border-color);
    margin: 1.5rem 0;
    padding: 0.5rem 1.5rem;
}

table {
    width: 100%;
    border-collapse: collapse;
    margin: 1.5rem 0;
}

th, td {
    border: 1px solid var(--border-color);
    padding: 0.5rem;
}

th {
    background-color: var(--code-bg);
    font-weight: bold;
}

img {
    max-width: 100%;
    height: auto;
    display: block;
    margin: 1rem auto;
}

figure {
    margin: 1.5rem 0;
    text-align: center;
}

figcaption {
    font-size: 0.9em;
    font-style: italic;
    margin-top: 0.5rem;
}
"#;

const MINIMAL_THEME_CSS: &str = r#"
:root {
    --bg-color: #ffffff;
    --text-color: #333333;
    --heading-color: #111111;
    --link-color: #0066cc;
    --code-bg: #f8f8f8;
    --border-color: #eeeeee;
}

body {
    font-family: var(--font-family, system-ui, sans-serif);
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.6;
    max-width: 680px;
    margin: 0 auto;
    padding: 1.5rem;
    font-size: 16px;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--heading-color);
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
    font-weight: 500;
}

h1 { font-size: 1.75rem; }
h2 { font-size: 1.4rem; }
h3 { font-size: 1.2rem; }

a { color: var(--link-color); }

code {
    background-color: var(--code-bg);
    padding: 0.1rem 0.3rem;
    font-size: 0.9em;
}

pre {
    background-color: var(--code-bg);
    padding: 0.75rem;
    overflow-x: auto;
}

pre code {
    background: none;
    padding: 0;
}

blockquote {
    border-left: 2px solid var(--border-color);
    margin: 1rem 0;
    padding-left: 1rem;
    color: #666;
}

table {
    width: 100%;
    border-collapse: collapse;
}

th, td {
    border-bottom: 1px solid var(--border-color);
    padding: 0.5rem;
    text-align: left;
}

img {
    max-width: 100%;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_light_theme_has_css() {
        let theme = ExportTheme::light();
        assert_eq!(theme.name, "light");
        assert!(theme.is_builtin);
        assert!(!theme.css.is_empty());
        assert!(theme.css.contains("--bg-color"));
    }

    #[test]
    fn test_dark_theme_has_different_colors() {
        let dark = ExportTheme::dark();
        assert_eq!(dark.name, "dark");
        assert!(dark.css.contains("#1a1a2e")); // dark background
    }

    #[test]
    fn test_academic_theme_uses_serif() {
        let academic = ExportTheme::academic();
        assert_eq!(academic.name, "academic");
        assert!(academic.fonts.iter().any(|f| f.contains("Georgia")));
    }

    #[test]
    fn test_minimal_theme_is_simple() {
        let minimal = ExportTheme::minimal();
        assert_eq!(minimal.name, "minimal");
    }

    #[test]
    fn test_builtin_themes_count() {
        let themes = ExportTheme::builtin_themes();
        assert_eq!(themes.len(), 4);
    }

    #[test]
    fn test_by_name_case_insensitive() {
        assert!(ExportTheme::by_name("LIGHT").is_some());
        assert!(ExportTheme::by_name("Light").is_some());
        assert!(ExportTheme::by_name("light").is_some());
    }

    #[test]
    fn test_by_name_unknown_returns_none() {
        assert!(ExportTheme::by_name("unknown").is_none());
    }

    #[test]
    fn test_theme_summary_from_theme() {
        let theme = ExportTheme::light();
        let summary = ThemeSummary::from(&theme);
        assert_eq!(summary.name, "light");
        assert!(summary.is_builtin);
        assert_eq!(summary.font_count, 3);
    }
}
