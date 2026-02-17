use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

/// Notification model representing the notifications table
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Notification {
    pub id: i32,
    pub user_id: i32,
    #[sqlx(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub message: Option<String>,
    pub is_read: bool,
    pub data: Option<Value>,
    pub created_at: DateTime<Utc>,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Comment,
    Mention,
    DocumentShare,
    WorkflowUpdate,
    SystemAlert,
}

impl ToString for NotificationType {
    fn to_string(&self) -> String {
        match self {
            Self::Comment => "comment".to_string(),
            Self::Mention => "mention".to_string(),
            Self::DocumentShare => "document_share".to_string(),
            Self::WorkflowUpdate => "workflow_update".to_string(),
            Self::SystemAlert => "system_alert".to_string(),
        }
    }
}

/// DTO for creating a new notification
#[derive(Debug, Deserialize)]
pub struct CreateNotification {
    pub user_id: i32,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: Option<String>,
    pub data: Option<Value>,
}

/// Unread notification count
#[derive(Debug, Serialize, FromRow)]
pub struct NotificationCount {
    pub unread_count: i64,
}
