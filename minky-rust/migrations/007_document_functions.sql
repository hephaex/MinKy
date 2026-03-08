-- Migration: Document Functions and Export Themes
-- Description: Add support for Quarkdown-style functions and custom export themes

-- Document Functions Table
-- Stores user-defined and system functions for document expansion
CREATE TABLE IF NOT EXISTS document_functions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    parameters JSONB NOT NULL DEFAULT '[]',
    body_type VARCHAR(20) NOT NULL CHECK (body_type IN ('builtin', 'template', 'ai_powered')),
    body TEXT NOT NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint on function name (case-insensitive)
CREATE UNIQUE INDEX IF NOT EXISTS idx_document_functions_name_unique
    ON document_functions (LOWER(name));

-- Index for listing by creator
CREATE INDEX IF NOT EXISTS idx_document_functions_created_by
    ON document_functions (created_by);

-- Export Themes Table
-- Stores custom themes for HTML/PDF/Slides export
CREATE TABLE IF NOT EXISTS export_themes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    css TEXT NOT NULL,
    fonts JSONB DEFAULT '[]',
    is_builtin BOOLEAN DEFAULT FALSE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Unique constraint on theme name (case-insensitive)
CREATE UNIQUE INDEX IF NOT EXISTS idx_export_themes_name_unique
    ON export_themes (LOWER(name));

-- Index for listing builtin themes
CREATE INDEX IF NOT EXISTS idx_export_themes_builtin
    ON export_themes (is_builtin);

-- Insert built-in themes
INSERT INTO export_themes (name, css, fonts, is_builtin) VALUES
(
    'light',
    ':root {
        --bg-color: #ffffff;
        --text-color: #1a1a1a;
        --heading-color: #0f0f0f;
        --link-color: #2563eb;
        --code-bg: #f3f4f6;
        --border-color: #e5e7eb;
        --blockquote-bg: #f9fafb;
    }
    body {
        font-family: var(--font-family, Inter, system-ui, sans-serif);
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
    code { background-color: var(--code-bg); padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.9em; }
    pre { background-color: var(--code-bg); padding: 1rem; border-radius: 8px; overflow-x: auto; }
    pre code { background: none; padding: 0; }
    blockquote { background-color: var(--blockquote-bg); border-left: 4px solid var(--link-color); margin: 1rem 0; padding: 1rem; }
    table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
    th, td { border: 1px solid var(--border-color); padding: 0.75rem; text-align: left; }
    th { background-color: var(--code-bg); font-weight: 600; }
    img { max-width: 100%; height: auto; }',
    '["Inter", "system-ui", "sans-serif"]',
    TRUE
),
(
    'dark',
    ':root {
        --bg-color: #1a1a2e;
        --text-color: #e4e4e7;
        --heading-color: #ffffff;
        --link-color: #60a5fa;
        --code-bg: #27273a;
        --border-color: #3f3f5a;
        --blockquote-bg: #232338;
    }
    body {
        font-family: var(--font-family, Inter, system-ui, sans-serif);
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
    code { background-color: var(--code-bg); padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.9em; }
    pre { background-color: var(--code-bg); padding: 1rem; border-radius: 8px; overflow-x: auto; }
    pre code { background: none; padding: 0; }
    blockquote { background-color: var(--blockquote-bg); border-left: 4px solid var(--link-color); margin: 1rem 0; padding: 1rem; }
    table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
    th, td { border: 1px solid var(--border-color); padding: 0.75rem; text-align: left; }
    th { background-color: var(--code-bg); font-weight: 600; }
    img { max-width: 100%; height: auto; }',
    '["Inter", "system-ui", "sans-serif"]',
    TRUE
),
(
    'academic',
    ':root {
        --bg-color: #fffef8;
        --text-color: #2d2d2d;
        --heading-color: #1a1a1a;
        --link-color: #8b0000;
        --code-bg: #f5f5f0;
        --border-color: #d4d4c8;
        --blockquote-bg: #fafaf5;
    }
    body {
        font-family: var(--font-family, Georgia, Times New Roman, serif);
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
    h1 { font-size: 2rem; text-align: center; margin-bottom: 2rem; }
    h2 { font-size: 1.5rem; border-bottom: 1px solid var(--border-color); padding-bottom: 0.25rem; }
    h3 { font-size: 1.25rem; font-style: italic; }
    a { color: var(--link-color); text-decoration: none; }
    a:hover { text-decoration: underline; }
    code { background-color: var(--code-bg); padding: 0.15rem 0.3rem; border-radius: 2px; font-size: 0.85em; font-family: Courier New, monospace; }
    pre { background-color: var(--code-bg); padding: 1rem; border: 1px solid var(--border-color); overflow-x: auto; }
    blockquote { font-style: italic; border-left: 3px solid var(--border-color); margin: 1.5rem 0; padding: 0.5rem 1.5rem; }
    table { width: 100%; border-collapse: collapse; margin: 1.5rem 0; }
    th, td { border: 1px solid var(--border-color); padding: 0.5rem; }
    th { background-color: var(--code-bg); font-weight: bold; }
    img { max-width: 100%; height: auto; display: block; margin: 1rem auto; }
    figure { margin: 1.5rem 0; text-align: center; }
    figcaption { font-size: 0.9em; font-style: italic; margin-top: 0.5rem; }',
    '["Georgia", "Times New Roman", "serif"]',
    TRUE
),
(
    'minimal',
    ':root {
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
    code { background-color: var(--code-bg); padding: 0.1rem 0.3rem; font-size: 0.9em; }
    pre { background-color: var(--code-bg); padding: 0.75rem; overflow-x: auto; }
    pre code { background: none; padding: 0; }
    blockquote { border-left: 2px solid var(--border-color); margin: 1rem 0; padding-left: 1rem; color: #666; }
    table { width: 100%; border-collapse: collapse; }
    th, td { border-bottom: 1px solid var(--border-color); padding: 0.5rem; text-align: left; }
    img { max-width: 100%; }',
    '["system-ui", "sans-serif"]',
    TRUE
)
ON CONFLICT (LOWER(name)) DO NOTHING;

-- Insert some example built-in functions
INSERT INTO document_functions (name, description, parameters, body_type, body) VALUES
(
    'greeting',
    'Generate a greeting message',
    '[{"name": "name", "type": "string", "required": true}]',
    'template',
    'Hello, {{0}}! Welcome to MinKy.'
),
(
    'signature',
    'Insert a signature block',
    '[{"name": "name", "type": "string", "required": true}, {"name": "title", "type": "string", "required": false}]',
    'template',
    '---\n**{{0}}**\n{{1}}'
),
(
    'callout',
    'Create a callout box',
    '[{"name": "type", "type": "string", "required": true}, {"name": "content", "type": "string", "required": true}]',
    'template',
    '> **{{0}}**: {{1}}'
)
ON CONFLICT (LOWER(name)) DO NOTHING;

-- Add trigger for updated_at on document_functions
CREATE OR REPLACE FUNCTION update_document_functions_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_document_functions_updated_at ON document_functions;
CREATE TRIGGER trigger_document_functions_updated_at
    BEFORE UPDATE ON document_functions
    FOR EACH ROW
    EXECUTE FUNCTION update_document_functions_timestamp();

-- Add trigger for updated_at on export_themes
CREATE OR REPLACE FUNCTION update_export_themes_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_export_themes_updated_at ON export_themes;
CREATE TRIGGER trigger_export_themes_updated_at
    BEFORE UPDATE ON export_themes
    FOR EACH ROW
    EXECUTE FUNCTION update_export_themes_timestamp();

-- Comments
COMMENT ON TABLE document_functions IS 'Quarkdown-style document functions for content expansion';
COMMENT ON TABLE export_themes IS 'Custom themes for HTML/PDF/Slides export';
COMMENT ON COLUMN document_functions.body_type IS 'Function type: builtin (system), template (user-defined), ai_powered (Claude)';
COMMENT ON COLUMN export_themes.is_builtin IS 'Built-in themes cannot be modified by users';
