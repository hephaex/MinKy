use sqlx::PgPool;

use crate::models::VerificationQueueEntry;

/// Service for managing migration checkpoints and verification queues.
pub struct MigrationService {
    pool: PgPool,
}

impl MigrationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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


}
