use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::fmt;

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

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Comment => "comment",
            Self::Mention => "mention",
            Self::DocumentShare => "document_share",
            Self::WorkflowUpdate => "workflow_update",
            Self::SystemAlert => "system_alert",
        };
        write!(f, "{}", s)
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
