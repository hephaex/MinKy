use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    // Client -> Server
    Subscribe { channels: Vec<String> },
    Unsubscribe { channels: Vec<String> },
    Ping { timestamp: i64 },

    // Server -> Client
    Pong { timestamp: i64 },
    Subscribed { channels: Vec<String> },
    Unsubscribed { channels: Vec<String> },

    // Events
    Event(WsEvent),
    Error { code: String, message: String },
}

/// WebSocket event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event_type: EventType,
    pub channel: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<i32>,
}

/// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Document events
    DocumentCreated,
    DocumentUpdated,
    DocumentDeleted,
    DocumentViewed,

    // Comment events
    CommentAdded,
    CommentUpdated,
    CommentDeleted,

    // Collaboration events
    UserJoined,
    UserLeft,
    CursorMoved,
    SelectionChanged,

    // Workflow events
    WorkflowStateChanged,
    WorkflowAssigned,

    // Notification events
    NotificationCreated,
    NotificationRead,

    // System events
    SystemAlert,
    MaintenanceScheduled,
}

/// Channel subscription
#[derive(Debug, Clone)]
pub struct ChannelSubscription {
    pub channel: String,
    pub user_id: i32,
    pub subscribed_at: DateTime<Utc>,
}

/// Presence info
#[derive(Debug, Clone, Serialize)]
pub struct PresenceInfo {
    pub user_id: i32,
    pub username: String,
    pub status: UserStatus,
    pub current_document: Option<uuid::Uuid>,
    pub cursor_position: Option<CursorPosition>,
    pub last_active: DateTime<Utc>,
}

/// User status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// Cursor position for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: i32,
    pub column: i32,
    pub selection_start: Option<i32>,
    pub selection_end: Option<i32>,
}

/// Collaboration session
#[derive(Debug, Serialize)]
pub struct CollaborationSession {
    pub document_id: uuid::Uuid,
    pub participants: Vec<PresenceInfo>,
    pub started_at: DateTime<Utc>,
}

/// Broadcast message
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    pub channel: String,
    pub event: WsEvent,
    pub exclude_user: Option<i32>,
}
