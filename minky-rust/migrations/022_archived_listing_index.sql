-- Partial composite index to support GET /api/documents/archived listing.
-- Covers WHERE user_id=$1 AND archived=true ORDER BY archived_at DESC without a full-scan sort.
CREATE INDEX IF NOT EXISTS idx_documents_archived_listing
    ON documents (user_id, archived_at DESC NULLS LAST)
    WHERE archived = true;
