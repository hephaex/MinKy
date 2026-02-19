use anyhow::Result;
use sqlx::PgPool;

use crate::{
    error::{AppError, AppResult},
    models::{CreateNotification, Notification, NotificationCount},
};

/// Notification service for user notifications
pub struct NotificationService {
    db: PgPool,
}

impl NotificationService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Get notifications for a user
    pub async fn list(
        &self,
        user_id: i32,
        include_read: bool,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<Notification>> {
        let notifications = if include_read {
            sqlx::query_as::<_, Notification>(
                r#"
                SELECT * FROM notifications
                WHERE user_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        } else {
            sqlx::query_as::<_, Notification>(
                r#"
                SELECT * FROM notifications
                WHERE user_id = $1 AND is_read = false
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db)
            .await?
        };

        Ok(notifications)
    }

    /// Get unread notification count
    pub async fn get_unread_count(&self, user_id: i32) -> Result<i64> {
        let count: NotificationCount = sqlx::query_as(
            "SELECT COUNT(*) as unread_count FROM notifications WHERE user_id = $1 AND is_read = false",
        )
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        Ok(count.unread_count)
    }

    /// Create a new notification
    pub async fn create(&self, data: CreateNotification) -> Result<Notification> {
        let notification = sqlx::query_as::<_, Notification>(
            r#"
            INSERT INTO notifications (user_id, type, title, message, data)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(data.notification_type.to_string())
        .bind(&data.title)
        .bind(&data.message)
        .bind(&data.data)
        .fetch_one(&self.db)
        .await?;

        Ok(notification)
    }

    /// Mark a notification as read
    pub async fn mark_as_read(&self, id: i32, user_id: i32) -> AppResult<Notification> {
        let notification = sqlx::query_as::<_, Notification>(
            r#"
            UPDATE notifications
            SET is_read = true
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?
        .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

        Ok(notification)
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_as_read(&self, user_id: i32) -> Result<i64> {
        let result = sqlx::query(
            "UPDATE notifications SET is_read = true WHERE user_id = $1 AND is_read = false",
        )
        .bind(user_id)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Delete a notification
    pub async fn delete(&self, id: i32, user_id: i32) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Notification not found".to_string()));
        }

        Ok(())
    }

    /// Delete old notifications (cleanup)
    pub async fn cleanup_old(&self, days: i32) -> Result<i64> {
        let result = sqlx::query(
            r#"
            DELETE FROM notifications
            WHERE is_read = true AND created_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days)
        .execute(&self.db)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// Notify user of a new comment on their document
    pub async fn notify_comment(
        &self,
        document_owner_id: i32,
        commenter_name: &str,
        document_title: &str,
        document_id: &str,
    ) -> Result<()> {
        use crate::models::NotificationType;

        self.create(CreateNotification {
            user_id: document_owner_id,
            notification_type: NotificationType::Comment,
            title: format!("{} commented on your document", commenter_name),
            message: Some(format!("New comment on \"{}\"", document_title)),
            data: Some(serde_json::json!({
                "document_id": document_id,
                "commenter": commenter_name
            })),
        })
        .await?;

        Ok(())
    }

    /// Notify user of a mention
    pub async fn notify_mention(
        &self,
        mentioned_user_id: i32,
        mentioner_name: &str,
        context: &str,
        document_id: &str,
    ) -> Result<()> {
        use crate::models::NotificationType;

        self.create(CreateNotification {
            user_id: mentioned_user_id,
            notification_type: NotificationType::Mention,
            title: format!("{} mentioned you", mentioner_name),
            message: Some(context.to_string()),
            data: Some(serde_json::json!({
                "document_id": document_id,
                "mentioner": mentioner_name
            })),
        })
        .await?;

        Ok(())
    }
}
