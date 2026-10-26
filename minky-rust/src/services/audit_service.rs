use anyhow::Result;
use sqlx::PgPool;

use crate::models::{AuditAction, AuditLog, CreateAuditLog};

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
