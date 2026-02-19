use anyhow::Result;
use chrono::{DateTime, Utc};
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

// ---- Pure helper functions (testable without DB) ----

/// Build an export details payload from document IDs and format
pub fn build_export_details(document_ids: &[String], format: &str) -> serde_json::Value {
    serde_json::json!({
        "document_ids": document_ids,
        "format": format,
        "count": document_ids.len()
    })
}

/// Build a login failed details payload
pub fn build_login_failed_details(email: &str) -> serde_json::Value {
    serde_json::json!({ "email": email })
}

/// Return whether an audit action is security-sensitive
pub fn is_security_sensitive(action: &AuditAction) -> bool {
    matches!(
        action,
        AuditAction::LoginFailed
            | AuditAction::Login
            | AuditAction::Logout
    )
}

/// Return whether an audit action affects document data
pub fn is_document_action(action: &AuditAction) -> bool {
    matches!(
        action,
        AuditAction::Create
            | AuditAction::Update
            | AuditAction::Delete
            | AuditAction::Read
            | AuditAction::Export
    )
}

/// Validate pagination parameters and clamp to safe range
pub fn clamp_audit_page_params(limit: i32, offset: i32) -> (i32, i32) {
    let safe_limit = limit.clamp(1, 1000);
    let safe_offset = offset.max(0);
    (safe_limit, safe_offset)
}

/// Build a document access details payload
pub fn build_document_access_details(document_id: &str, action: &str) -> serde_json::Value {
    serde_json::json!({
        "document_id": document_id,
        "action": action
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_export_details_contains_count() {
        let ids = vec!["id1".to_string(), "id2".to_string(), "id3".to_string()];
        let details = build_export_details(&ids, "json");
        assert_eq!(details["count"], 3);
        assert_eq!(details["format"], "json");
        assert!(details["document_ids"].is_array());
    }

    #[test]
    fn test_build_export_details_empty_list() {
        let ids: Vec<String> = vec![];
        let details = build_export_details(&ids, "csv");
        assert_eq!(details["count"], 0);
        assert_eq!(details["format"], "csv");
    }

    #[test]
    fn test_build_login_failed_details() {
        let details = build_login_failed_details("user@example.com");
        assert_eq!(details["email"], "user@example.com");
    }

    #[test]
    fn test_is_security_sensitive_login_failed() {
        assert!(is_security_sensitive(&AuditAction::LoginFailed));
        assert!(is_security_sensitive(&AuditAction::Login));
        assert!(is_security_sensitive(&AuditAction::Logout));
    }

    #[test]
    fn test_is_security_sensitive_non_sensitive() {
        assert!(!is_security_sensitive(&AuditAction::Create));
        assert!(!is_security_sensitive(&AuditAction::Read));
        assert!(!is_security_sensitive(&AuditAction::Export));
    }

    #[test]
    fn test_is_document_action_create_update_delete() {
        assert!(is_document_action(&AuditAction::Create));
        assert!(is_document_action(&AuditAction::Update));
        assert!(is_document_action(&AuditAction::Delete));
        assert!(is_document_action(&AuditAction::Read));
        assert!(is_document_action(&AuditAction::Export));
    }

    #[test]
    fn test_is_document_action_login_not_document() {
        assert!(!is_document_action(&AuditAction::Login));
        assert!(!is_document_action(&AuditAction::LoginFailed));
    }

    #[test]
    fn test_clamp_audit_page_params_valid() {
        let (limit, offset) = clamp_audit_page_params(50, 100);
        assert_eq!(limit, 50);
        assert_eq!(offset, 100);
    }

    #[test]
    fn test_clamp_audit_page_params_too_large() {
        let (limit, _) = clamp_audit_page_params(9999, 0);
        assert_eq!(limit, 1000);
    }

    #[test]
    fn test_clamp_audit_page_params_negative_offset() {
        let (_, offset) = clamp_audit_page_params(10, -5);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_clamp_audit_page_params_zero_limit() {
        let (limit, _) = clamp_audit_page_params(0, 0);
        assert_eq!(limit, 1);
    }

    #[test]
    fn test_build_document_access_details() {
        let details = build_document_access_details("doc-uuid-123", "view");
        assert_eq!(details["document_id"], "doc-uuid-123");
        assert_eq!(details["action"], "view");
    }
}
