-- 021: Soft-archive flag + partial UNIQUE constraint for content_hash dedup
-- Strategy: mark duplicate extras as archived=true (keep oldest), then enforce
-- uniqueness on active documents only. Reversible — no rows deleted.

-- 1. Add archived column
ALTER TABLE documents ADD COLUMN IF NOT EXISTS archived BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS archived_at TIMESTAMPTZ;

-- 2. Soft-archive duplicates: keep oldest (MIN created_at), mark rest archived
--    Uses a CTE to identify non-oldest duplicates per (user_id, content_hash).
WITH to_archive AS (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY user_id, content_hash
                   ORDER BY created_at ASC, id ASC
               ) AS rn
        FROM documents
        WHERE content_hash IS NOT NULL
    ) ranked
    WHERE rn > 1
)
UPDATE documents
SET archived    = true,
    archived_at = NOW()
WHERE id IN (SELECT id FROM to_archive)
  AND archived = false;

-- 3. Partial UNIQUE index: only among active (non-archived) documents.
--    This enforces the no-duplicate rule going forward without constraining
--    the archived history.
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_content_hash_active
    ON documents (user_id, content_hash)
    WHERE archived = false AND content_hash IS NOT NULL;
