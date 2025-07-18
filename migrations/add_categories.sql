-- Migration to add categories table and category_id to documents table

-- Create categories table
CREATE TABLE IF NOT EXISTS categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    parent_id INTEGER REFERENCES categories(id) ON DELETE CASCADE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    color VARCHAR(7) DEFAULT '#007bff',
    sort_order INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE
);

-- Add category_id column to documents table
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL;

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_categories_parent_id ON categories(parent_id);
CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug);
CREATE INDEX IF NOT EXISTS idx_categories_created_by ON categories(created_by);
CREATE INDEX IF NOT EXISTS idx_documents_category_id ON documents(category_id);

-- Insert some default categories
INSERT INTO categories (name, slug, description, color, sort_order) VALUES
('General', 'general', 'General documents and notes', '#007bff', 0),
('Projects', 'projects', 'Project-related documents', '#28a745', 1),
('Research', 'research', 'Research papers and studies', '#ffc107', 2),
('Personal', 'personal', 'Personal notes and thoughts', '#dc3545', 3),
('Archive', 'archive', 'Archived documents', '#6c757d', 4)
ON CONFLICT (slug) DO NOTHING;

-- Create a trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_categories_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER IF NOT EXISTS categories_updated_at_trigger
    BEFORE UPDATE ON categories
    FOR EACH ROW
    EXECUTE FUNCTION update_categories_updated_at();