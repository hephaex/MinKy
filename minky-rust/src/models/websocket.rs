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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_ping_serde_roundtrip() {
        let msg = WsMessage::Ping { timestamp: 1700000000 };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WsMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, WsMessage::Ping { timestamp: 1700000000 }));
    }

    #[test]
    fn test_ws_message_subscribe_serde_roundtrip() {
        let msg = WsMessage::Subscribe {
            channels: vec!["doc-001".to_string(), "doc-002".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: WsMessage = serde_json::from_str(&json).unwrap();
        if let WsMessage::Subscribe { channels } = back {
            assert_eq!(channels.len(), 2);
            assert_eq!(channels[0], "doc-001");
        } else {
            panic!("Expected Subscribe variant");
        }
    }

    #[test]
    fn test_ws_message_type_tag_snake_case() {
        let msg = WsMessage::Ping { timestamp: 0 };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "ping");
    }

    #[test]
    fn test_ws_message_pong_type_tag() {
        let msg = WsMessage::Pong { timestamp: 42 };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "pong");
    }

    #[test]
    fn test_event_type_document_created_snake_case() {
        let et = EventType::DocumentCreated;
        let json = serde_json::to_value(&et).unwrap();
        assert_eq!(json, "document_created");
    }

    #[test]
    fn test_event_type_user_joined_snake_case() {
        let et = EventType::UserJoined;
        let json = serde_json::to_value(&et).unwrap();
        assert_eq!(json, "user_joined");
    }

    #[test]
    fn test_event_type_system_alert_snake_case() {
        let et = EventType::SystemAlert;
        let json = serde_json::to_value(&et).unwrap();
        assert_eq!(json, "system_alert");
    }

    #[test]
    fn test_user_status_online_lowercase() {
        let s = UserStatus::Online;
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json, "online");
    }

    #[test]
    fn test_user_status_offline_lowercase() {
        let s = UserStatus::Offline;
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json, "offline");
    }

    #[test]
    fn test_user_status_deserialize_busy() {
        let s: UserStatus = serde_json::from_str(r#""busy""#).unwrap();
        assert!(matches!(s, UserStatus::Busy));
    }

    #[test]
    fn test_cursor_position_all_optional_none() {
        let cp = CursorPosition {
            line: 10,
            column: 5,
            selection_start: None,
            selection_end: None,
        };
        assert_eq!(cp.line, 10);
        assert!(cp.selection_start.is_none());
    }

    #[test]
    fn test_cursor_position_with_selection() {
        let cp = CursorPosition {
            line: 1,
            column: 0,
            selection_start: Some(0),
            selection_end: Some(20),
        };
        assert_eq!(cp.selection_start, Some(0));
        assert_eq!(cp.selection_end, Some(20));
    }

    #[test]
    fn test_ws_message_error_serde() {
        let msg = WsMessage::Error {
            code: "E001".to_string(),
            message: "Not authorised".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["code"], "E001");
    }
}
