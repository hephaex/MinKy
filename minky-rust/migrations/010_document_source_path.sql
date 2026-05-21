-- Add source_path column to documents
ALTER TABLE documents ADD COLUMN IF NOT EXISTS source_path TEXT;

-- Index for fast lookup by source_path (non-null only)
CREATE INDEX IF NOT EXISTS idx_documents_source_path
    ON documents(source_path)
    WHERE source_path IS NOT NULL;

-- Partial unique index: one document per (user_id, source_path)
-- Only enforces uniqueness when source_path is set (vault ingests)
-- Plain text/URL ingests have source_path = NULL and are unaffected
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_user_source
    ON documents(user_id, source_path)
    WHERE source_path IS NOT NULL;
