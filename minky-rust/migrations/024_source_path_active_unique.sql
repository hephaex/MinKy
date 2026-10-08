-- 024: Scope source_path uniqueness to active (non-archived) documents only
--
-- Required by the upload S1 upsert (routes/documents.rs::upload_document):
-- when a re-upload targets a source_path that already has an active document
-- with different content, the old document is archived (archived=true) and a
-- new document is inserted with the SAME source_path. Migration 010's
-- idx_documents_user_source has no archived filter, so it would reject that
-- INSERT with a unique-violation the moment more than one row (active or
-- archived) shares a (user_id, source_path) pair.
--
-- Mirrors 021's uq_user_content_hash_active pattern: allow any number of
-- archived rows to share a source_path (version history), enforce
-- uniqueness only among active documents.

DROP INDEX IF EXISTS idx_documents_user_source;

CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_user_source_active
    ON documents (user_id, source_path)
    WHERE source_path IS NOT NULL AND archived = false;
