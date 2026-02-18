use anyhow::Result;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::models::{
    BroadcastMessage, ChannelSubscription, CollaborationSession, CursorPosition, EventType,
    PresenceInfo, UserStatus, WsEvent, WsMessage,
};

/// WebSocket connection manager
pub struct WebSocketManager {
    /// Channel subscriptions: channel_name -> set of user_ids
    subscriptions: Arc<RwLock<HashMap<String, HashSet<i32>>>>,
    /// User presence info
    presence: Arc<RwLock<HashMap<i32, PresenceInfo>>>,
    /// Broadcast sender for events
    broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            presence: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    /// Subscribe user to channels
    pub async fn subscribe(&self, user_id: i32, channels: Vec<String>) -> Result<Vec<String>> {
        let mut subs = self.subscriptions.write().await;

        for channel in &channels {
            subs.entry(channel.clone())
                .or_insert_with(HashSet::new)
                .insert(user_id);
        }

        Ok(channels)
    }

    /// Unsubscribe user from channels
    pub async fn unsubscribe(&self, user_id: i32, channels: Vec<String>) -> Result<Vec<String>> {
        let mut subs = self.subscriptions.write().await;

        for channel in &channels {
            if let Some(users) = subs.get_mut(channel) {
                users.remove(&user_id);
                if users.is_empty() {
                    subs.remove(channel);
                }
            }
        }

        Ok(channels)
    }

    /// Unsubscribe user from all channels (on disconnect)
    pub async fn disconnect(&self, user_id: i32) -> Result<()> {
        let mut subs = self.subscriptions.write().await;

        for users in subs.values_mut() {
            users.remove(&user_id);
        }

        // Remove empty channels
        subs.retain(|_, users| !users.is_empty());

        // Remove presence
        let mut presence = self.presence.write().await;
        presence.remove(&user_id);

        Ok(())
    }

    /// Update user presence
    pub async fn update_presence(&self, user_id: i32, info: PresenceInfo) -> Result<()> {
        let mut presence = self.presence.write().await;
        presence.insert(user_id, info);
        Ok(())
    }

    /// Get users in channel
    pub async fn get_channel_users(&self, channel: &str) -> Vec<i32> {
        let subs = self.subscriptions.read().await;
        subs.get(channel)
            .map(|users| users.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get presence for users
    pub async fn get_presence(&self, user_ids: &[i32]) -> Vec<PresenceInfo> {
        let presence = self.presence.read().await;
        user_ids
            .iter()
            .filter_map(|id| presence.get(id).cloned())
            .collect()
    }

    /// Broadcast event to channel
    pub async fn broadcast(&self, channel: &str, event: WsEvent, exclude_user: Option<i32>) {
        let message = BroadcastMessage {
            channel: channel.to_string(),
            event,
            exclude_user,
        };

        let _ = self.broadcast_tx.send(message);
    }

    /// Get broadcast receiver
    pub fn subscribe_to_broadcasts(&self) -> broadcast::Receiver<BroadcastMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Emit document event
    pub async fn emit_document_event(
        &self,
        event_type: EventType,
        document_id: uuid::Uuid,
        user_id: i32,
        payload: serde_json::Value,
    ) {
        let channel = format!("document:{}", document_id);
        let event = WsEvent {
            event_type,
            channel: channel.clone(),
            payload,
            timestamp: Utc::now(),
            user_id: Some(user_id),
        };

        self.broadcast(&channel, event, Some(user_id)).await;
    }

    /// Emit notification event
    pub async fn emit_notification(&self, user_id: i32, payload: serde_json::Value) {
        let channel = format!("user:{}", user_id);
        let event = WsEvent {
            event_type: EventType::NotificationCreated,
            channel: channel.clone(),
            payload,
            timestamp: Utc::now(),
            user_id: Some(user_id),
        };

        self.broadcast(&channel, event, None).await;
    }

    /// Get collaboration session for document
    pub async fn get_collaboration_session(
        &self,
        document_id: uuid::Uuid,
    ) -> Result<CollaborationSession> {
        let channel = format!("document:{}", document_id);
        let user_ids = self.get_channel_users(&channel).await;
        let participants = self.get_presence(&user_ids).await;

        Ok(CollaborationSession {
            document_id,
            participants,
            started_at: Utc::now(), // TODO: Track actual start time
        })
    }

    /// Handle cursor position update
    pub async fn update_cursor(
        &self,
        user_id: i32,
        document_id: uuid::Uuid,
        position: CursorPosition,
    ) -> Result<()> {
        let channel = format!("document:{}", document_id);

        let payload = serde_json::json!({
            "user_id": user_id,
            "position": position
        });

        let event = WsEvent {
            event_type: EventType::CursorMoved,
            channel: channel.clone(),
            payload,
            timestamp: Utc::now(),
            user_id: Some(user_id),
        };

        self.broadcast(&channel, event, Some(user_id)).await;
        Ok(())
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket event service for business logic
pub struct WebSocketEventService {
    manager: Arc<WebSocketManager>,
}

impl WebSocketEventService {
    pub fn new(manager: Arc<WebSocketManager>) -> Self {
        Self { manager }
    }

    /// Handle incoming message
    pub async fn handle_message(&self, user_id: i32, message: WsMessage) -> Result<WsMessage> {
        match message {
            WsMessage::Subscribe { channels } => {
                let subscribed = self.manager.subscribe(user_id, channels).await?;
                Ok(WsMessage::Subscribed {
                    channels: subscribed,
                })
            }
            WsMessage::Unsubscribe { channels } => {
                let unsubscribed = self.manager.unsubscribe(user_id, channels).await?;
                Ok(WsMessage::Unsubscribed {
                    channels: unsubscribed,
                })
            }
            WsMessage::Ping { timestamp } => Ok(WsMessage::Pong { timestamp }),
            _ => Ok(WsMessage::Error {
                code: "invalid_message".to_string(),
                message: "Invalid message type from client".to_string(),
            }),
        }
    }

    /// Notify document created
    pub async fn notify_document_created(
        &self,
        document_id: uuid::Uuid,
        user_id: i32,
        title: &str,
        category_id: Option<i32>,
    ) {
        let payload = serde_json::json!({
            "document_id": document_id,
            "title": title,
            "category_id": category_id
        });

        self.manager
            .emit_document_event(EventType::DocumentCreated, document_id, user_id, payload)
            .await;
    }

    /// Notify document updated
    pub async fn notify_document_updated(
        &self,
        document_id: uuid::Uuid,
        user_id: i32,
        changes: serde_json::Value,
    ) {
        self.manager
            .emit_document_event(EventType::DocumentUpdated, document_id, user_id, changes)
            .await;
    }

    /// Notify document deleted
    pub async fn notify_document_deleted(&self, document_id: uuid::Uuid, user_id: i32) {
        let payload = serde_json::json!({
            "document_id": document_id
        });

        self.manager
            .emit_document_event(EventType::DocumentDeleted, document_id, user_id, payload)
            .await;
    }

    /// Notify workflow state changed
    pub async fn notify_workflow_changed(
        &self,
        document_id: uuid::Uuid,
        user_id: i32,
        old_state: &str,
        new_state: &str,
    ) {
        let payload = serde_json::json!({
            "old_state": old_state,
            "new_state": new_state
        });

        self.manager
            .emit_document_event(
                EventType::WorkflowStateChanged,
                document_id,
                user_id,
                payload,
            )
            .await;
    }

    /// Notify comment added
    pub async fn notify_comment_added(
        &self,
        document_id: uuid::Uuid,
        comment_id: i32,
        user_id: i32,
        content: &str,
    ) {
        let payload = serde_json::json!({
            "comment_id": comment_id,
            "content": content
        });

        self.manager
            .emit_document_event(EventType::CommentAdded, document_id, user_id, payload)
            .await;
    }
}
