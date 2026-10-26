use anyhow::Result;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::models::{
    BroadcastMessage, EventType,
    PresenceInfo, WsEvent, WsMessage,
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





}

#[cfg(test)]
mod tests {
    use super::*;

    // --- WebSocketManager ---

    #[tokio::test]
    async fn test_subscribe_adds_user_to_channel() {
        let mgr = WebSocketManager::new();
        mgr.subscribe(1, vec!["doc:123".to_string()]).await.unwrap();
        let users = mgr.get_channel_users("doc:123").await;
        assert!(users.contains(&1), "User 1 should be in the channel after subscribe");
    }

    #[tokio::test]
    async fn test_subscribe_multiple_users_to_same_channel() {
        let mgr = WebSocketManager::new();
        mgr.subscribe(1, vec!["chat".to_string()]).await.unwrap();
        mgr.subscribe(2, vec!["chat".to_string()]).await.unwrap();
        let users = mgr.get_channel_users("chat").await;
        assert!(users.contains(&1));
        assert!(users.contains(&2));
    }

    #[tokio::test]
    async fn test_unsubscribe_removes_user_from_channel() {
        let mgr = WebSocketManager::new();
        mgr.subscribe(1, vec!["room".to_string()]).await.unwrap();
        mgr.unsubscribe(1, vec!["room".to_string()]).await.unwrap();
        let users = mgr.get_channel_users("room").await;
        assert!(!users.contains(&1), "User should be removed after unsubscribe");
    }

    #[tokio::test]
    async fn test_get_channel_users_empty_for_unknown_channel() {
        let mgr = WebSocketManager::new();
        let users = mgr.get_channel_users("nonexistent").await;
        assert!(users.is_empty(), "Unknown channel should return empty user list");
    }

    #[tokio::test]
    async fn test_subscribe_to_broadcasts_receives_messages() {
        let mgr = WebSocketManager::new();
        let mut rx = mgr.subscribe_to_broadcasts();

        // Subscribe user so broadcast has something to send to
        mgr.subscribe(1, vec!["updates".to_string()]).await.unwrap();

        // Broadcast returns immediately (fire-and-forget)
        mgr.broadcast(
            "updates",
            WsEvent {
                event_type: EventType::DocumentUpdated,
                channel: "updates".to_string(),
                payload: serde_json::json!({}),
                timestamp: chrono::Utc::now(),
                user_id: None,
            },
            None,
        )
        .await;

        let msg = rx.try_recv();
        assert!(msg.is_ok(), "Should have received a broadcast message");
        assert_eq!(msg.unwrap().channel, "updates");
    }

    #[tokio::test]
    async fn test_subscribe_returns_channel_list() {
        let mgr = WebSocketManager::new();
        let channels = vec!["a".to_string(), "b".to_string()];
        let result = mgr.subscribe(1, channels.clone()).await.unwrap();
        assert_eq!(result, channels, "Subscribe should return the subscribed channels");
    }
}
