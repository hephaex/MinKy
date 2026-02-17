use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

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

impl ToString for AuditAction {
    fn to_string(&self) -> String {
        match self {
            Self::Create => "create".to_string(),
            Self::Read => "read".to_string(),
            Self::Update => "update".to_string(),
            Self::Delete => "delete".to_string(),
            Self::Login => "login".to_string(),
            Self::Logout => "logout".to_string(),
            Self::LoginFailed => "login_failed".to_string(),
            Self::Export => "export".to_string(),
            Self::Import => "import".to_string(),
            Self::Share => "share".to_string(),
            Self::AdminAction => "admin_action".to_string(),
        }
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

impl ToString for ResourceType {
    fn to_string(&self) -> String {
        match self {
            Self::Document => "document".to_string(),
            Self::User => "user".to_string(),
            Self::Tag => "tag".to_string(),
            Self::Category => "category".to_string(),
            Self::Comment => "comment".to_string(),
            Self::Attachment => "attachment".to_string(),
            Self::Workflow => "workflow".to_string(),
            Self::System => "system".to_string(),
        }
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
