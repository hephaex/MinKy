use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;

use crate::models::{AuditAction, AuditLog, CreateAuditLog, ResourceType};

/// Audit service for logging all security-relevant actions
pub struct AuditService {
    db: PgPool,
}

impl AuditService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Log an audit event
    pub async fn log(&self, entry: CreateAuditLog) -> Result<AuditLog> {
        let audit_log = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (user_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(entry.user_id)
        .bind(entry.action.to_string())
        .bind(entry.resource_type.to_string())
        .bind(entry.resource_id)
        .bind(entry.details)
        .bind(entry.ip_address)
        .bind(entry.user_agent)
        .fetch_one(&self.db)
        .await?;

        Ok(audit_log)
    }

    /// Get audit logs for a user
    pub async fn get_user_logs(
        &self,
        user_id: i32,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<AuditLog>> {
        let logs = sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    /// Get audit logs for a resource
    pub async fn get_resource_logs(
        &self,
        resource_type: ResourceType,
        resource_id: &str,
        limit: i32,
    ) -> Result<Vec<AuditLog>> {
        let logs = sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            WHERE resource_type = $1 AND resource_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(resource_type.to_string())
        .bind(resource_id)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    /// Get all audit logs (admin only)
    pub async fn get_all_logs(
        &self,
        action_filter: Option<AuditAction>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i32,
        offset: i32,
    ) -> Result<(Vec<AuditLog>, i64)> {
        let mut query = String::from("SELECT * FROM audit_logs WHERE 1=1");
        let mut count_query = String::from("SELECT COUNT(*) FROM audit_logs WHERE 1=1");

        if action_filter.is_some() {
            query.push_str(" AND action = $1");
            count_query.push_str(" AND action = $1");
        }

        if start_date.is_some() {
            query.push_str(" AND created_at >= $2");
            count_query.push_str(" AND created_at >= $2");
        }

        if end_date.is_some() {
            query.push_str(" AND created_at <= $3");
            count_query.push_str(" AND created_at <= $3");
        }

        query.push_str(" ORDER BY created_at DESC LIMIT $4 OFFSET $5");

        // For now, use a simpler implementation
        let logs = sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(&self.db)
            .await?;

        Ok((logs, count.0))
    }

    /// Quick helpers for common audit actions
    pub async fn log_login(&self, user_id: i32, ip: Option<String>, user_agent: Option<String>) -> Result<()> {
        self.log(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::Login,
            resource_type: ResourceType::User,
            resource_id: Some(user_id.to_string()),
            details: None,
            ip_address: ip,
            user_agent,
        })
        .await?;
        Ok(())
    }

    pub async fn log_login_failed(&self, email: &str, ip: Option<String>, user_agent: Option<String>) -> Result<()> {
        self.log(CreateAuditLog {
            user_id: None,
            action: AuditAction::LoginFailed,
            resource_type: ResourceType::User,
            resource_id: None,
            details: Some(serde_json::json!({ "email": email })),
            ip_address: ip,
            user_agent,
        })
        .await?;
        Ok(())
    }

    pub async fn log_document_access(
        &self,
        user_id: i32,
        document_id: &str,
        action: AuditAction,
        ip: Option<String>,
    ) -> Result<()> {
        self.log(CreateAuditLog {
            user_id: Some(user_id),
            action,
            resource_type: ResourceType::Document,
            resource_id: Some(document_id.to_string()),
            details: None,
            ip_address: ip,
            user_agent: None,
        })
        .await?;
        Ok(())
    }

    pub async fn log_export(
        &self,
        user_id: i32,
        document_ids: &[String],
        format: &str,
        ip: Option<String>,
    ) -> Result<()> {
        self.log(CreateAuditLog {
            user_id: Some(user_id),
            action: AuditAction::Export,
            resource_type: ResourceType::Document,
            resource_id: None,
            details: Some(serde_json::json!({
                "document_ids": document_ids,
                "format": format,
                "count": document_ids.len()
            })),
            ip_address: ip,
            user_agent: None,
        })
        .await?;
        Ok(())
    }
}
