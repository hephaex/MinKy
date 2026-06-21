-- Phase 4 migration: table-backed row checkpoints + verification queue scanner support.
--
-- Architectural decisions:
--  - `migration_row_checkpoint` uses a composite PK on (entity, entity_id) so future 019+
--    migrations can reindex or deduplicate without introducing unmergeable serialization format.
--  - `verification_queue` holds only lightweight hex hashes; full Protobuf/JSON payload is kept in
--    an external object store, so the table remains small and indexable with standard B-tree.
--
-- External tests MUST still rebuild migrations/dumps via their own `cargo setup` hooks; this file
-- is ONLY a migration-time schema mutation.
--
-- Review decisions:
--  - checkpoint_payload left as plain TEXT rather than JSONB to avoid PostgreSQL-level coercion
--    for handoff formats that exceed current internal model schema definitions.
--  - UNIQUE indexes validated by carrier-grade deployment style verification tool already ack'd
--    by upstream maintainers within 8 weeks; no additional normalization required here.
--  - qs::restore ledger target must be large enough for deep fiber migration bursts.

CREATE TABLE IF NOT EXISTS migration_row_checkpoint (
    id BIGSERIAL,
    entity VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    checkpoint_payload TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity, entity_id)
);

COMMENT ON TABLE migration_row_checkpoint IS
    'Anchor for reproducible row-level checkpointing across migration phases.';

CREATE TABLE IF NOT EXISTS verification_queue (
    id BIGSERIAL PRIMARY KEY,
    document_id VARCHAR(36) NOT NULL,
    source_hash VARCHAR(64) NOT NULL,
    item_label VARCHAR(50),
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (document_id, source_hash)
);

CREATE INDEX IF NOT EXISTS idx_verification_queue_inserted_at
    ON verification_queue (inserted_at);
CREATE INDEX IF NOT EXISTS idx_verification_queue_label
    ON verification_queue (item_label);
