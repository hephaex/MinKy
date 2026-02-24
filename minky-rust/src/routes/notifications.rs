use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    middleware::AuthUser,
    models::Notification,
    services::NotificationService,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/count", get(get_unread_count))
        .route("/{id}/read", put(mark_as_read))
        .route("/read-all", post(mark_all_as_read))
        .route("/{id}", delete(delete_notification))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub include_read: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct NotificationListResponse {
    pub success: bool,
    pub data: Vec<Notification>,
    pub unread_count: i64,
}

async fn list_notifications(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<NotificationListResponse>> {
    let service = NotificationService::new(state.db.clone());
    let notifications = service
        .list(
            auth_user.id,
            query.include_read.unwrap_or(true),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await?;
    let unread_count = service.get_unread_count(auth_user.id).await?;

    Ok(Json(NotificationListResponse {
        success: true,
        data: notifications,
        unread_count,
    }))
}

#[derive(Debug, Serialize)]
pub struct UnreadCountResponse {
    pub success: bool,
    pub count: i64,
}

async fn get_unread_count(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<UnreadCountResponse>> {
    let service = NotificationService::new(state.db.clone());
    let count = service.get_unread_count(auth_user.id).await?;

    Ok(Json(UnreadCountResponse {
        success: true,
        count,
    }))
}

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub success: bool,
    pub data: Notification,
}

async fn mark_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<NotificationResponse>> {
    let service = NotificationService::new(state.db.clone());
    let notification = service.mark_as_read(id, auth_user.id).await?;

    Ok(Json(NotificationResponse {
        success: true,
        data: notification,
    }))
}

#[derive(Debug, Serialize)]
pub struct MarkAllReadResponse {
    pub success: bool,
    pub count: i64,
}

async fn mark_all_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<MarkAllReadResponse>> {
    let service = NotificationService::new(state.db.clone());
    let count = service.mark_all_as_read(auth_user.id).await?;

    Ok(Json(MarkAllReadResponse {
        success: true,
        count,
    }))
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

async fn delete_notification(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<i32>,
) -> AppResult<Json<DeleteResponse>> {
    let service = NotificationService::new(state.db.clone());
    service.delete(id, auth_user.id).await?;

    Ok(Json(DeleteResponse {
        success: true,
        message: "Notification deleted successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ListQuery tests
    #[test]
    fn test_list_query_default_values() {
        let query = ListQuery {
            include_read: None,
            limit: None,
            offset: None,
        };
        assert!(query.include_read.is_none());
        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_list_query_with_values() {
        let query = ListQuery {
            include_read: Some(false),
            limit: Some(20),
            offset: Some(10),
        };
        assert_eq!(query.include_read, Some(false));
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_list_query_include_read_default() {
        let query = ListQuery {
            include_read: None,
            limit: None,
            offset: None,
        };
        let include_read = query.include_read.unwrap_or(true);
        assert!(include_read);
    }

    #[test]
    fn test_list_query_limit_default() {
        let query = ListQuery {
            include_read: None,
            limit: None,
            offset: None,
        };
        let limit = query.limit.unwrap_or(50);
        assert_eq!(limit, 50);
    }

    #[test]
    fn test_list_query_offset_default() {
        let query = ListQuery {
            include_read: None,
            limit: None,
            offset: None,
        };
        let offset = query.offset.unwrap_or(0);
        assert_eq!(offset, 0);
    }

    // NotificationListResponse tests
    #[test]
    fn test_notification_list_response_creation() {
        let notifications = vec![Notification {
            id: 1,
            user_id: 42,
            notification_type: "comment".to_string(),
            title: "New comment".to_string(),
            message: Some("Someone commented".to_string()),
            is_read: false,
            data: None,
            created_at: Utc::now(),
        }];
        let response = NotificationListResponse {
            success: true,
            data: notifications,
            unread_count: 1,
        };
        assert!(response.success);
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.unread_count, 1);
    }

    #[test]
    fn test_notification_list_response_empty() {
        let response = NotificationListResponse {
            success: true,
            data: vec![],
            unread_count: 0,
        };
        assert!(response.data.is_empty());
        assert_eq!(response.unread_count, 0);
    }

    #[test]
    fn test_notification_list_response_multiple() {
        let notifications = vec![
            Notification {
                id: 1,
                user_id: 1,
                notification_type: "comment".to_string(),
                title: "Comment 1".to_string(),
                message: None,
                is_read: false,
                data: None,
                created_at: Utc::now(),
            },
            Notification {
                id: 2,
                user_id: 1,
                notification_type: "mention".to_string(),
                title: "Mention".to_string(),
                message: None,
                is_read: true,
                data: None,
                created_at: Utc::now(),
            },
        ];
        let response = NotificationListResponse {
            success: true,
            data: notifications,
            unread_count: 1,
        };
        assert_eq!(response.data.len(), 2);
    }

    // UnreadCountResponse tests
    #[test]
    fn test_unread_count_response_zero() {
        let response = UnreadCountResponse {
            success: true,
            count: 0,
        };
        assert!(response.success);
        assert_eq!(response.count, 0);
    }

    #[test]
    fn test_unread_count_response_many() {
        let response = UnreadCountResponse {
            success: true,
            count: 99,
        };
        assert_eq!(response.count, 99);
    }

    // NotificationResponse tests
    #[test]
    fn test_notification_response_creation() {
        let notification = Notification {
            id: 5,
            user_id: 10,
            notification_type: "workflow".to_string(),
            title: "Status changed".to_string(),
            message: Some("Document approved".to_string()),
            is_read: true,
            data: None,
            created_at: Utc::now(),
        };
        let response = NotificationResponse {
            success: true,
            data: notification,
        };
        assert!(response.success);
        assert_eq!(response.data.id, 5);
        assert!(response.data.is_read);
    }

    // MarkAllReadResponse tests
    #[test]
    fn test_mark_all_read_response_zero() {
        let response = MarkAllReadResponse {
            success: true,
            count: 0,
        };
        assert!(response.success);
        assert_eq!(response.count, 0);
    }

    #[test]
    fn test_mark_all_read_response_many() {
        let response = MarkAllReadResponse {
            success: true,
            count: 15,
        };
        assert_eq!(response.count, 15);
    }

    // DeleteResponse tests
    #[test]
    fn test_delete_response_creation() {
        let response = DeleteResponse {
            success: true,
            message: "Notification deleted successfully".to_string(),
        };
        assert!(response.success);
        assert!(response.message.contains("deleted"));
    }

    // Notification tests
    #[test]
    fn test_notification_unread() {
        let notification = Notification {
            id: 1,
            user_id: 1,
            notification_type: "system".to_string(),
            title: "System alert".to_string(),
            message: None,
            is_read: false,
            data: None,
            created_at: Utc::now(),
        };
        assert!(!notification.is_read);
    }

    #[test]
    fn test_notification_with_message() {
        let notification = Notification {
            id: 2,
            user_id: 2,
            notification_type: "comment".to_string(),
            title: "New reply".to_string(),
            message: Some("Check it out!".to_string()),
            is_read: false,
            data: None,
            created_at: Utc::now(),
        };
        assert!(notification.message.is_some());
        assert_eq!(notification.message.unwrap(), "Check it out!");
    }

    #[test]
    fn test_notification_without_message() {
        let notification = Notification {
            id: 3,
            user_id: 3,
            notification_type: "mention".to_string(),
            title: "You were mentioned".to_string(),
            message: None,
            is_read: false,
            data: None,
            created_at: Utc::now(),
        };
        assert!(notification.message.is_none());
    }
}
