use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::models::{
    ConflictType, CreateSyncConfig, ResolveConflictRequest,
    SyncConfig, SyncConflict, SyncHistoryEntry, SyncJob,
    SyncProvider, SyncStats, SyncStatus,
};

/// Raw DB row type for sync config queries
type SyncConfigRow = (
    i32,
    String,
    String,
    String,
    Option<String>,
    Option<serde_json::Value>,
    String,
    bool,
    i32,
    bool,
    bool,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// Raw DB row type for sync history queries
type SyncHistoryRow = (
    i64,
    i32,
    String,
    i32,
    i32,
    i64,
    i64,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
);

/// Raw DB row type for sync conflict queries
type SyncConflictRow = (
    String,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    String,
    Option<String>,
);

/// Sync service for document synchronization
pub struct SyncService {
    db: PgPool,
}

impl SyncService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List sync configurations
    pub async fn list_configs(&self, user_id: i32) -> Result<Vec<SyncConfig>> {
        let rows: Vec<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider, remote_path, local_path, credentials, sync_direction,
                   auto_sync, sync_interval_minutes, include_attachments, is_active,
                   created_at, updated_at
            FROM sync_configs
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SyncConfig {
                id: r.0,
                name: r.1,
                provider: serde_json::from_str(&r.2).unwrap_or(SyncProvider::Local),
                remote_path: r.3,
                local_path: r.4,
                credentials: r.5,
                sync_direction: serde_json::from_str(&r.6).unwrap_or_default(),
                auto_sync: r.7,
                sync_interval_minutes: r.8,
                include_attachments: r.9,
                is_active: r.10,
                created_at: r.11,
                updated_at: r.12,
            })
            .collect())
    }

    /// Get sync configuration
    pub async fn get_config(&self, config_id: i32) -> Result<Option<SyncConfig>> {
        let row: Option<SyncConfigRow> = sqlx::query_as(
            r#"
            SELECT id, name, provider, remote_path, local_path, credentials, sync_direction,
                   auto_sync, sync_interval_minutes, include_attachments, is_active,
                   created_at, updated_at
            FROM sync_configs
            WHERE id = $1
            "#,
        )
        .bind(config_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| SyncConfig {
            id: r.0,
            name: r.1,
            provider: serde_json::from_str(&r.2).unwrap_or(SyncProvider::Local),
            remote_path: r.3,
            local_path: r.4,
            credentials: r.5,
            sync_direction: serde_json::from_str(&r.6).unwrap_or_default(),
            auto_sync: r.7,
            sync_interval_minutes: r.8,
            include_attachments: r.9,
            is_active: r.10,
            created_at: r.11,
            updated_at: r.12,
        }))
    }

    /// Create sync configuration
    pub async fn create_config(&self, user_id: i32, create: CreateSyncConfig) -> Result<SyncConfig> {
        let provider_str = serde_json::to_string(&create.provider)?;
        let direction_str = serde_json::to_string(&create.sync_direction.unwrap_or_default())?;

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO sync_configs (
                user_id, name, provider, remote_path, local_path, credentials, sync_direction,
                auto_sync, sync_interval_minutes, include_attachments, is_active, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(&create.name)
        .bind(&provider_str)
        .bind(&create.remote_path)
        .bind(&create.local_path)
        .bind(&create.credentials)
        .bind(&direction_str)
        .bind(create.auto_sync.unwrap_or(false))
        .bind(create.sync_interval_minutes.unwrap_or(60))
        .bind(create.include_attachments.unwrap_or(true))
        .fetch_one(&self.db)
        .await?;

        self.get_config(row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created config"))
    }

    /// Delete sync configuration
    pub async fn delete_config(&self, user_id: i32, config_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM sync_configs WHERE id = $1 AND user_id = $2")
            .bind(config_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Start sync job
    pub async fn start_sync(&self, config_id: i32) -> Result<SyncJob> {
        let config = self
            .get_config(config_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Config not found"))?;

        let job_id = uuid::Uuid::new_v4().to_string();

        Ok(SyncJob {
            id: job_id,
            config_id,
            status: SyncStatus::Pending,
            direction: config.sync_direction,
            files_total: 0,
            files_synced: 0,
            files_failed: 0,
            bytes_transferred: 0,
            conflicts: vec![],
            errors: vec![],
            started_at: Utc::now(),
            completed_at: None,
        })
    }

    /// Get sync job status
    pub async fn get_job_status(&self, _job_id: &str) -> Result<Option<SyncJob>> {
        // TODO: Implement job queue
        Ok(None)
    }

    /// Get sync history
    pub async fn get_history(&self, config_id: i32, limit: i32) -> Result<Vec<SyncHistoryEntry>> {
        let rows: Vec<SyncHistoryRow> = sqlx::query_as(
            r#"
            SELECT id, config_id, status, files_synced, files_failed, bytes_transferred,
                   EXTRACT(EPOCH FROM (completed_at - started_at))::bigint as duration,
                   started_at, completed_at
            FROM sync_history
            WHERE config_id = $1
            ORDER BY started_at DESC
            LIMIT $2
            "#,
        )
        .bind(config_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SyncHistoryEntry {
                id: r.0,
                config_id: r.1,
                status: serde_json::from_str(&r.2).unwrap_or_default(),
                files_synced: r.3,
                files_failed: r.4,
                bytes_transferred: r.5,
                duration_seconds: r.6,
                started_at: r.7,
                completed_at: r.8,
            })
            .collect())
    }

    /// Get pending conflicts
    pub async fn get_conflicts(&self, config_id: i32) -> Result<Vec<SyncConflict>> {
        let rows: Vec<SyncConflictRow> = sqlx::query_as(
            r#"
            SELECT file_path, local_modified, remote_modified, conflict_type, resolution
            FROM sync_conflicts
            WHERE config_id = $1 AND resolved = false
            ORDER BY local_modified DESC
            "#,
        )
        .bind(config_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SyncConflict {
                file_path: r.0,
                local_modified: r.1,
                remote_modified: r.2,
                conflict_type: serde_json::from_str(&r.3).unwrap_or(ConflictType::BothModified),
                resolution: r.4.and_then(|s| serde_json::from_str(&s).ok()),
            })
            .collect())
    }

    /// Resolve conflict
    pub async fn resolve_conflict(
        &self,
        config_id: i32,
        request: ResolveConflictRequest,
    ) -> Result<()> {
        let resolution_str = serde_json::to_string(&request.resolution)?;

        sqlx::query(
            r#"
            UPDATE sync_conflicts
            SET resolution = $1, resolved = true, resolved_at = NOW()
            WHERE config_id = $2 AND file_path = $3
            "#,
        )
        .bind(&resolution_str)
        .bind(config_id)
        .bind(&request.file_path)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get sync statistics
    pub async fn get_stats(&self, config_id: i32) -> Result<SyncStats> {
        let totals: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*)::bigint,
                COUNT(*) FILTER (WHERE status = '"completed"')::bigint,
                COUNT(*) FILTER (WHERE status = '"failed"')::bigint,
                COALESCE(SUM(files_synced), 0)::bigint,
                COALESCE(SUM(bytes_transferred), 0)::bigint
            FROM sync_history
            WHERE config_id = $1
            "#,
        )
        .bind(config_id)
        .fetch_one(&self.db)
        .await?;

        let last_sync: Option<(chrono::DateTime<chrono::Utc>,)> = sqlx::query_as(
            "SELECT completed_at FROM sync_history WHERE config_id = $1 ORDER BY completed_at DESC LIMIT 1",
        )
        .bind(config_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(SyncStats {
            total_syncs: totals.0,
            successful_syncs: totals.1,
            failed_syncs: totals.2,
            total_files_synced: totals.3,
            total_bytes_transferred: totals.4,
            last_sync: last_sync.map(|r| r.0),
            next_scheduled_sync: None, // TODO: Calculate from config
        })
    }
}
