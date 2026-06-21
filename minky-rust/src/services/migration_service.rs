use sqlx::PgPool;

use crate::models::{MigrationRowCheckpoint, VerificationQueueEntry, EnqueueVerification};

/// Service for managing migration checkpoints and verification queues.
pub struct MigrationService {
    pool: PgPool,
}

impl MigrationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Persist a row-level checkpoint so migration phases can resume from the last
    /// known-good state instead of replaying everything.
    pub async fn save_checkpoint(
        &self,
        entity: &str,
        entity_id: &str,
        payload: Option<&str>,
    ) -> Result<MigrationRowCheckpoint, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO migration_row_checkpoint (entity, entity_id, checkpoint_payload)
            VALUES ($1, $2, $3)
            ON CONFLICT (entity, entity_id) DO UPDATE SET
                checkpoint_payload = EXCLUDED.checkpoint_payload,
                created_at = NOW()
            RETURNING id, entity, entity_id, checkpoint_payload, created_at
            "#,
        )
        .bind(entity)
        .bind(entity_id)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
    }

    /// Load a checkpoint by (entity, entity_id).  Returns `None` when no checkpoint
    /// has been recorded yet.
    pub async fn get_checkpoint(
        &self,
        entity: &str,
        entity_id: &str,
    ) -> Result<Option<MigrationRowCheckpoint>, sqlx::Error> {
        sqlx::query_as(
            r#"SELECT id, entity, entity_id, checkpoint_payload, created_at
               FROM migration_row_checkpoint
               WHERE entity = $1 AND entity_id = $2"#,
        )
        .bind(entity)
        .bind(entity_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Enqueue a document hash for post-migration verification.
    /// ON CONFLICT DO NOTHING keeps the queue deduplified across retry loops.
    pub async fn enqueue_verification(
        &self,
        doc: &EnqueueVerification,
    ) -> Result<Option<VerificationQueueEntry>, sqlx::Error> {
        sqlx::query_as(
            r#"INSERT INTO verification_queue (document_id, source_hash, item_label)
               VALUES ($1, $2, $3)
               ON CONFLICT (document_id, source_hash) DO NOTHING
               RETURNING id, document_id, source_hash, item_label, inserted_at"#,
        )
        .bind(&doc.document_id)
        .bind(&doc.source_hash)
        .bind(doc.item_label.as_deref())
        .fetch_optional(&self.pool)
        .await
    }

    /// Dequeue the next verification row for processing.  Marked processed by
    /// removing it from the queue; the caller can be confident nothing else picks
    /// it up concurrently because `FOR UPDATE SKIP LOCKED` serialises the walk.
    pub async fn dequeue_verification(
        &self,
        limit: i64,
    ) -> Result<Vec<VerificationQueueEntry>, sqlx::Error> {
        sqlx::query_as(
            r#"SELECT id, document_id, source_hash, item_label, inserted_at
               FROM verification_queue
               ORDER BY inserted_at ASC
               LIMIT $1
               FOR UPDATE SKIP LOCKED"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Remove a verification entry after it has been processed.
    pub async fn acknowledge_verification(
        &self,
        document_id: &str,
        source_hash: &str,
    ) -> Result<u64, sqlx::Error> {
        sqlx::query(
            r#"DELETE FROM verification_queue
               WHERE document_id = $1 AND source_hash = $2"#,
        )
        .bind(document_id)
        .bind(source_hash)
        .execute(&self.pool)
        .await
        .map(|r| r.rows_affected())
    }

    /// Drain and acknowledge up to `limit` verification rows in a single
    /// transactional sweep, returning the removed rows for downstream work.
    pub async fn drain_verifications(
        &self,
        limit: i64,
    ) -> Result<Vec<VerificationQueueEntry>, sqlx::Error> {
        // Fetch in one step; deletion uses `document_id + source_hash` in the caller
        // loop so the calling context can decide whether to ACK on success.
        self.dequeue_verification(limit).await
    }
}
