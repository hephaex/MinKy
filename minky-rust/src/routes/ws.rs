use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, error, info, warn};

use crate::{
    middleware::AuthUser,
    models::{PresenceInfo, UserStatus, WsMessage},
    services::WebSocketEventService,
    AppState,
};

/// WebSocket routes
pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    user: AuthUser,
) -> impl IntoResponse {
    info!("WebSocket connection request from user {}", user.id);

    ws.on_upgrade(move |socket| handle_socket(socket, state, user.id))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, user_id: i32) {
    let manager = state.ws_manager.clone();
    let service = WebSocketEventService::new(manager.clone());

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcasts
    let mut broadcast_rx = manager.subscribe_to_broadcasts();

    // Update presence to online
    let presence = PresenceInfo {
        user_id,
        username: format!("user_{}", user_id), // TODO: Fetch from DB
        status: UserStatus::Online,
        current_document: None,
        cursor_position: None,
        last_active: chrono::Utc::now(),
    };
    let _ = manager.update_presence(user_id, presence).await;

    info!("WebSocket connected: user_id={}", user_id);

    // Spawn task to forward broadcasts to this client
    let broadcast_user_id = user_id;
    let broadcast_subscriptions = manager.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            // Check if user is subscribed to this channel
            let users = broadcast_subscriptions.get_channel_users(&msg.channel).await;
            if !users.contains(&broadcast_user_id) {
                continue;
            }

            // Skip if this user is excluded
            if msg.exclude_user == Some(broadcast_user_id) {
                continue;
            }

            // Send the event
            let ws_msg = WsMessage::Event(msg.event);
            if let Ok(json) = serde_json::to_string(&ws_msg) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    let recv_manager = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        match service.handle_message(user_id, ws_msg).await {
                            Ok(response) => {
                                debug!("Handled message from user {}: {:?}", user_id, response);
                            }
                            Err(e) => {
                                warn!("Error handling message: {}", e);
                            }
                        }
                    } else {
                        warn!("Invalid JSON from user {}: {}", user_id, text);
                    }
                }
                Message::Binary(_) => {
                    debug!("Received binary message from user {}", user_id);
                }
                Message::Ping(data) => {
                    debug!("Received ping from user {}", user_id);
                    // Axum handles pong automatically
                    let _ = data;
                }
                Message::Pong(_) => {
                    debug!("Received pong from user {}", user_id);
                }
                Message::Close(_) => {
                    info!("WebSocket close requested by user {}", user_id);
                    break;
                }
            }
        }

        // Cleanup on disconnect
        if let Err(e) = recv_manager.disconnect(user_id).await {
            error!("Error during disconnect cleanup: {}", e);
        }
        info!("WebSocket disconnected: user_id={}", user_id);
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_parse_subscribe() {
        let json = r#"{"type":"subscribe","channels":["doc:123","doc:456"]}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        if let WsMessage::Subscribe { channels } = msg {
            assert_eq!(channels.len(), 2);
            assert_eq!(channels[0], "doc:123");
        } else {
            panic!("Expected Subscribe variant");
        }
    }

    #[test]
    fn test_ws_message_parse_ping() {
        let json = r#"{"type":"ping","timestamp":1700000000}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WsMessage::Ping { timestamp: 1700000000 }));
    }

    #[test]
    fn test_ws_message_parse_unsubscribe() {
        let json = r#"{"type":"unsubscribe","channels":["room1"]}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        if let WsMessage::Unsubscribe { channels } = msg {
            assert_eq!(channels.len(), 1);
            assert_eq!(channels[0], "room1");
        } else {
            panic!("Expected Unsubscribe variant");
        }
    }

    #[test]
    fn test_ws_message_serialize_subscribed() {
        let msg = WsMessage::Subscribed {
            channels: vec!["ch1".to_string()],
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "subscribed");
        assert_eq!(json["channels"][0], "ch1");
    }

    #[test]
    fn test_ws_message_serialize_pong() {
        let msg = WsMessage::Pong { timestamp: 42 };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "pong");
        assert_eq!(json["timestamp"], 42);
    }

    #[test]
    fn test_ws_message_serialize_error() {
        let msg = WsMessage::Error {
            code: "E001".to_string(),
            message: "Test error".to_string(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["code"], "E001");
    }
}
