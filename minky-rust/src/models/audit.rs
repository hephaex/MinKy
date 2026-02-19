use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::fmt;

/// Audit log model representing the audit_logs table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AuditLog {
    pub id: i32,
    pub user_id: Option<i32>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Audit actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Read,
    Update,
    Delete,
    Login,
    Logout,
    LoginFailed,
    Export,
    Import,
    Share,
    AdminAction,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Create => "create",
            Self::Read => "read",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Login => "login",
            Self::Logout => "logout",
            Self::LoginFailed => "login_failed",
            Self::Export => "export",
            Self::Import => "import",
            Self::Share => "share",
            Self::AdminAction => "admin_action",
        };
        write!(f, "{}", s)
    }
}

/// Resource types for audit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Document,
    User,
    Tag,
    Category,
    Comment,
    Attachment,
    Workflow,
    System,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Document => "document",
            Self::User => "user",
            Self::Tag => "tag",
            Self::Category => "category",
            Self::Comment => "comment",
            Self::Attachment => "attachment",
            Self::Workflow => "workflow",
            Self::System => "system",
        };
        write!(f, "{}", s)
    }
}

/// DTO for creating an audit log entry
#[derive(Debug)]
pub struct CreateAuditLog {
    pub user_id: Option<i32>,
    pub action: AuditAction,
    pub resource_type: ResourceType,
    pub resource_id: Option<String>,
    pub details: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::Create.to_string(), "create");
        assert_eq!(AuditAction::Read.to_string(), "read");
        assert_eq!(AuditAction::Update.to_string(), "update");
        assert_eq!(AuditAction::Delete.to_string(), "delete");
        assert_eq!(AuditAction::Login.to_string(), "login");
        assert_eq!(AuditAction::Logout.to_string(), "logout");
        assert_eq!(AuditAction::LoginFailed.to_string(), "login_failed");
        assert_eq!(AuditAction::Export.to_string(), "export");
        assert_eq!(AuditAction::Import.to_string(), "import");
        assert_eq!(AuditAction::Share.to_string(), "share");
        assert_eq!(AuditAction::AdminAction.to_string(), "admin_action");
    }

    #[test]
    fn test_resource_type_display() {
        assert_eq!(ResourceType::Document.to_string(), "document");
        assert_eq!(ResourceType::User.to_string(), "user");
        assert_eq!(ResourceType::Tag.to_string(), "tag");
        assert_eq!(ResourceType::Category.to_string(), "category");
        assert_eq!(ResourceType::Comment.to_string(), "comment");
        assert_eq!(ResourceType::Attachment.to_string(), "attachment");
        assert_eq!(ResourceType::Workflow.to_string(), "workflow");
        assert_eq!(ResourceType::System.to_string(), "system");
    }

    #[test]
    fn test_audit_action_format_string() {
        // Verify Display works correctly in format strings
        let action = AuditAction::Create;
        let formatted = format!("Action: {}", action);
        assert_eq!(formatted, "Action: create");
    }
}
