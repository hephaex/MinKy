use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Checkpoint record anchoring a migrated row for reproducible replay.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MigrationRowCheckpoint {
    pub id: i64,
    pub entity: String,
    pub entity_id: String,
    #[serde(default)]
    pub checkpoint_payload: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Verification queue entry for post-migration hash comparison.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationQueueEntry {
    pub id: i64,
    pub document_id: String,
    pub source_hash: String,
    #[serde(default)]
    pub item_label: Option<String>,
    pub inserted_at: DateTime<Utc>,
}

/// Parameters for enqueuing a verification row.
#[derive(Debug, Deserialize)]
pub struct EnqueueVerification {
    pub document_id: String,
    pub source_hash: String,
    #[serde(default)]
    pub item_label: Option<String>,
}
