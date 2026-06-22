-- 020: Add content_hash for duplicate detection
-- Uses pgcrypto sha256 to fingerprint document content.
-- No unique constraint yet — existing duplicates cleaned in Sprint D2.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE documents ADD COLUMN IF NOT EXISTS content_hash TEXT;

-- Backfill existing documents
UPDATE documents
SET content_hash = encode(digest(content, 'sha256'), 'hex')
WHERE content_hash IS NULL;

-- Index for fast duplicate lookup per user
CREATE INDEX IF NOT EXISTS idx_documents_user_content_hash
    ON documents (user_id, content_hash)
    WHERE content_hash IS NOT NULL;

-- View: duplicate groups (same user, same content)
CREATE OR REPLACE VIEW v_duplicate_groups AS
SELECT
    user_id,
    content_hash,
    COUNT(*)         AS doc_count,
    MIN(created_at)  AS oldest_at,
    MAX(created_at)  AS newest_at,
    array_agg(id ORDER BY created_at)     AS document_ids,
    array_agg(title ORDER BY created_at)  AS titles
FROM documents
WHERE content_hash IS NOT NULL
GROUP BY user_id, content_hash
HAVING COUNT(*) > 1;
