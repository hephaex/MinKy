-- Phase 2 Migration: Flask compatibility columns (additive only)
-- Decision C = A1: minky (Flask) → minky_rust (Rust). Source is SELECT-only.
-- All columns are nullable so existing Rust data is unaffected.

-- ── documents ──────────────────────────────────────────────────────────────
ALTER TABLE documents
    ADD COLUMN IF NOT EXISTS author          VARCHAR(255),
    ADD COLUMN IF NOT EXISTS html_content    TEXT,
    ADD COLUMN IF NOT EXISTS document_metadata JSONB,
    ADD COLUMN IF NOT EXISTS is_published    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS published_at    TIMESTAMP WITH TIME ZONE;

-- ── categories ─────────────────────────────────────────────────────────────
-- Rust categories already has: id, name, parent_id, user_id, created_at, updated_at
ALTER TABLE categories
    ADD COLUMN IF NOT EXISTS slug        VARCHAR(100),
    ADD COLUMN IF NOT EXISTS description TEXT,
    ADD COLUMN IF NOT EXISTS color       VARCHAR(7) NOT NULL DEFAULT '#007bff',
    ADD COLUMN IF NOT EXISTS sort_order  INTEGER    NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS is_active   BOOLEAN    NOT NULL DEFAULT true;

CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug) WHERE slug IS NOT NULL;

-- ── tags ───────────────────────────────────────────────────────────────────
-- Rust tags already has: id, name, user_id, created_at, UNIQUE(name, user_id)
-- Flask tags have a global UNIQUE(name); Rust's per-user unique is a superset constraint.
ALTER TABLE tags
    ADD COLUMN IF NOT EXISTS slug        VARCHAR(50),
    ADD COLUMN IF NOT EXISTS description TEXT,
    ADD COLUMN IF NOT EXISTS color       VARCHAR(7) NOT NULL DEFAULT '#007bff';

CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_slug ON tags(slug) WHERE slug IS NOT NULL;

-- ── document_versions ──────────────────────────────────────────────────────
-- Rust already has: id, document_id UUID, content, version_number, created_by, created_at
ALTER TABLE document_versions
    ADD COLUMN IF NOT EXISTS title        VARCHAR(500),
    ADD COLUMN IF NOT EXISTS author       VARCHAR(255),
    ADD COLUMN IF NOT EXISTS html_content TEXT,
    ADD COLUMN IF NOT EXISTS content_hash VARCHAR(64),
    ADD COLUMN IF NOT EXISTS change_summary TEXT;

-- ── flask_document_id_mapping ──────────────────────────────────────────────
-- Maps Flask INTEGER document.id → Rust UUID document.id for FK rewriting.
-- Populated by migrate_flask_to_rust.py; used to verify completeness.
CREATE TABLE IF NOT EXISTS flask_document_id_mapping (
    flask_id  INTEGER NOT NULL,
    rust_id   UUID    NOT NULL,
    migrated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (flask_id),
    UNIQUE (rust_id)
);

COMMENT ON TABLE flask_document_id_mapping IS
    'Flask int → Rust UUID document ID mapping. Populated by migration script.';
