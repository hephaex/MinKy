-- Sprint 1: Add sensitivity classification to documents
-- confidential = 기밀 (재무제표·고객PoC·견적서 등), internal = 내부용 (default), public = 공개
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS sensitivity VARCHAR(20)
        NOT NULL DEFAULT 'internal'
        CHECK (sensitivity IN ('confidential', 'internal', 'public'));

CREATE INDEX IF NOT EXISTS idx_documents_sensitivity ON documents(sensitivity);
