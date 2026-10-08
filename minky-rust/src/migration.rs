pub mod checks {
    use sqlx::PgPool;

    /// Creates the migration_row_checkpoints table if it doesn't exist.
    pub async fn ensure_table(pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS migration_row_checkpoints (
                id BIGSERIAL PRIMARY KEY,
                migrated_schema_id BIGINT,
                source_table TEXT,
                source_row_id TEXT,
                source_hash TEXT,
                target_row_hash TEXT,
                migration_timestamp TIMESTAMPTZ DEFAULT NOW(),
                migration_row_checkpoint_id BYTEA,
                is_complete BOOLEAN DEFAULT false,
                completed_at TIMESTAMPTZ,
                error_message TEXT,
                error_code TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(migration_row_checkpoint_id)
            )"
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
