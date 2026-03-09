-- Migration: QMD-Inspired Hybrid Search Features
-- Add hierarchical context, collections, and search optimizations

-- Collections table for organizing documents
CREATE TABLE IF NOT EXISTS collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    patterns JSONB DEFAULT '[]'::jsonb,
    parent_id UUID REFERENCES collections(id) ON DELETE SET NULL,
    context JSONB DEFAULT '{}'::jsonb,
    document_count BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_collections_parent ON collections(parent_id);
CREATE INDEX idx_collections_name ON collections(name);

-- Context annotations for collections and documents
CREATE TABLE IF NOT EXISTS context_annotations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_type VARCHAR(20) NOT NULL CHECK (target_type IN ('collection', 'document')),
    target_id UUID NOT NULL,
    context_path VARCHAR(512) NOT NULL,
    context_text TEXT NOT NULL,
    context_type VARCHAR(20) NOT NULL DEFAULT 'description',
    priority INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_context_annotations_target ON context_annotations(target_type, target_id);
CREATE INDEX idx_context_annotations_path ON context_annotations(context_path);

-- Add collection_id to documents
ALTER TABLE documents ADD COLUMN IF NOT EXISTS collection_id UUID REFERENCES collections(id);
CREATE INDEX IF NOT EXISTS idx_documents_collection ON documents(collection_id);

-- Add full-text search vector to documents (if not exists)
ALTER TABLE documents ADD COLUMN IF NOT EXISTS search_vector tsvector;
CREATE INDEX IF NOT EXISTS idx_documents_fts ON documents USING GIN(search_vector);

-- Trigger to update search_vector on document changes
CREATE OR REPLACE FUNCTION update_document_search_vector()
RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.content, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trig_documents_search_vector ON documents;
CREATE TRIGGER trig_documents_search_vector
    BEFORE INSERT OR UPDATE OF title, content ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_document_search_vector();

-- Update existing documents to populate search_vector
UPDATE documents
SET search_vector =
    setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(content, '')), 'B')
WHERE search_vector IS NULL;

-- Collection contexts view (for hierarchical context retrieval)
CREATE OR REPLACE VIEW collection_contexts AS
SELECT
    d.id as document_id,
    c.id as collection_id,
    c.name as collection_name,
    COALESCE(
        (SELECT context_path FROM context_annotations
         WHERE target_type = 'document' AND target_id = d.id
         ORDER BY priority DESC LIMIT 1),
        'minky://' || c.name
    ) as context_path
FROM documents d
LEFT JOIN collections c ON d.collection_id = c.id;

-- Query expansion cache (optional, for performance)
CREATE TABLE IF NOT EXISTS query_expansion_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_hash VARCHAR(64) NOT NULL UNIQUE,
    original_query TEXT NOT NULL,
    expansions JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ DEFAULT NOW() + INTERVAL '1 day'
);

CREATE INDEX idx_query_expansion_cache_hash ON query_expansion_cache(query_hash);
CREATE INDEX idx_query_expansion_cache_expires ON query_expansion_cache(expires_at);

-- Search history enhancements
ALTER TABLE search_history ADD COLUMN IF NOT EXISTS search_mode VARCHAR(20);
ALTER TABLE search_history ADD COLUMN IF NOT EXISTS expanded_queries JSONB;
ALTER TABLE search_history ADD COLUMN IF NOT EXISTS result_count INTEGER;
ALTER TABLE search_history ADD COLUMN IF NOT EXISTS took_ms INTEGER;

-- Function to get collection hierarchy
CREATE OR REPLACE FUNCTION get_collection_hierarchy(collection_uuid UUID)
RETURNS TABLE (
    id UUID,
    name VARCHAR(255),
    depth INTEGER
) AS $$
WITH RECURSIVE hierarchy AS (
    SELECT id, name, parent_id, 0 as depth
    FROM collections
    WHERE id = collection_uuid

    UNION ALL

    SELECT c.id, c.name, c.parent_id, h.depth + 1
    FROM collections c
    JOIN hierarchy h ON c.id = h.parent_id
)
SELECT id, name, depth FROM hierarchy ORDER BY depth DESC;
$$ LANGUAGE SQL;

-- Function to update collection document counts
CREATE OR REPLACE FUNCTION update_collection_document_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' OR TG_OP = 'UPDATE' THEN
        UPDATE collections
        SET document_count = (
            SELECT COUNT(*) FROM documents WHERE collection_id = NEW.collection_id
        )
        WHERE id = NEW.collection_id;
    END IF;

    IF TG_OP = 'DELETE' OR TG_OP = 'UPDATE' THEN
        UPDATE collections
        SET document_count = (
            SELECT COUNT(*) FROM documents WHERE collection_id = OLD.collection_id
        )
        WHERE id = OLD.collection_id;
    END IF;

    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trig_update_collection_count ON documents;
CREATE TRIGGER trig_update_collection_count
    AFTER INSERT OR UPDATE OF collection_id OR DELETE ON documents
    FOR EACH ROW
    EXECUTE FUNCTION update_collection_document_count();

-- Seed default collection
INSERT INTO collections (id, name, description, context)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'default',
    'Default collection for uncategorized documents',
    '{"summary": "General documents", "topics": ["general"], "audience": ["all"]}'::jsonb
) ON CONFLICT DO NOTHING;
