from app.models.notification import Notification, NotificationType, NotificationPreference
from app.models.user import User
from app.models.comment import Comment
from app import db
from datetime import datetime, timedelta, timezone
import re

class NotificationService:
    """Service for managing notifications and their delivery"""
    
    @staticmethod
    def create_document_comment_notification(comment, document, actor):
        """Create notification when someone comments on a document"""
        # Notify document author (if not the commenter)
        if document.author_id != actor.id:
            preferences = NotificationPreference.get_or_create_preferences(document.author_id)
            if preferences.should_notify(NotificationType.DOCUMENT_COMMENTED):
                title = f"New comment on '{document.title}'"
                message = f"{actor.username} commented on your document"
                
                Notification.create_notification(
                    user_id=document.author_id,
                    notification_type=NotificationType.DOCUMENT_COMMENTED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    comment_id=comment.id,
                    actor_id=actor.id,
                    data={
                        'comment_excerpt': comment.content[:100] + '...' if len(comment.content) > 100 else comment.content
                    }
                )
        
        # Check for mentions in comment content
        NotificationService._create_mention_notifications(comment.content, comment, document, actor)
        
        # Notify other commenters on the same document (excluding actor and document author)
        other_commenters = db.session.query(Comment.user_id).filter(
            Comment.document_id == document.id,
            Comment.user_id != actor.id,
            Comment.user_id != document.author_id,
            Comment.is_deleted == False
        ).distinct().all()
        
        for (commenter_id,) in other_commenters:
            preferences = NotificationPreference.get_or_create_preferences(commenter_id)
            if preferences.should_notify(NotificationType.DOCUMENT_COMMENTED):
                title = f"New comment on '{document.title}'"
                message = f"{actor.username} also commented on this document"
                
                Notification.create_notification(
                    user_id=commenter_id,
                    notification_type=NotificationType.DOCUMENT_COMMENTED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    comment_id=comment.id,
                    actor_id=actor.id
                )
    
    @staticmethod
    def create_document_rating_notification(rating, document, actor):
        """Create notification when someone rates a document"""
        if document.author_id != actor.id:
            preferences = NotificationPreference.get_or_create_preferences(document.author_id)
            if preferences.should_notify(NotificationType.DOCUMENT_RATED):
                stars = "⭐" * rating.rating
                title = f"New rating on '{document.title}'"
                message = f"{actor.username} rated your document {stars} ({rating.rating}/5)"
                
                Notification.create_notification(
                    user_id=document.author_id,
                    notification_type=NotificationType.DOCUMENT_RATED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    actor_id=actor.id,
                    data={'rating': rating.rating}
                )
    
    @staticmethod
    def create_document_update_notification(document, actor):
        """Create notification when a document is updated"""
        # Notify followers/watchers of the document
        # For now, notify commenters and raters
        interested_users = set()
        
        # Get users who have commented
        commenters = db.session.query(Comment.user_id).filter(
            Comment.document_id == document.id,
            Comment.user_id != actor.id,
            Comment.is_deleted == False
        ).distinct().all()
        interested_users.update([user_id for (user_id,) in commenters])
        
        # Get users who have rated
        from app.models.comment import Rating
        raters = db.session.query(Rating.user_id).filter(
            Rating.document_id == document.id,
            Rating.user_id != actor.id
        ).distinct().all()
        interested_users.update([user_id for (user_id,) in raters])
        
        for user_id in interested_users:
            preferences = NotificationPreference.get_or_create_preferences(user_id)
            if preferences.should_notify(NotificationType.DOCUMENT_UPDATED):
                title = f"Document updated: '{document.title}'"
                message = f"{actor.username} updated a document you're following"
                
                Notification.create_notification(
                    user_id=user_id,
                    notification_type=NotificationType.DOCUMENT_UPDATED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    actor_id=actor.id
                )
    
    @staticmethod
    def create_document_version_notification(version, document, actor):
        """Create notification when a new document version is created"""
        # Notify document author if version was created by someone else
        if document.author_id != actor.id:
            preferences = NotificationPreference.get_or_create_preferences(document.author_id)
            if preferences.should_notify(NotificationType.DOCUMENT_VERSION_CREATED):
                title = f"New version of '{document.title}'"
                message = f"{actor.username} created version {version.version_number} of your document"
                
                Notification.create_notification(
                    user_id=document.author_id,
                    notification_type=NotificationType.DOCUMENT_VERSION_CREATED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    actor_id=actor.id,
                    data={'version_number': version.version_number}
                )
    
    @staticmethod
    def create_template_usage_notification(template, document, actor):
        """Create notification when someone uses a template"""
        if template.author_id != actor.id:
            preferences = NotificationPreference.get_or_create_preferences(template.author_id)
            if preferences.should_notify(NotificationType.TEMPLATE_USED):
                title = f"Template used: '{template.name}'"
                message = f"{actor.username} created a document using your template"
                
                Notification.create_notification(
                    user_id=template.author_id,
                    notification_type=NotificationType.TEMPLATE_USED,
                    title=title,
                    message=message,
                    document_id=document.id,
                    actor_id=actor.id,
                    data={
                        'template_id': template.id,
                        'template_name': template.name,
                        'document_title': document.title
                    }
                )
    
    @staticmethod
    def create_document_export_notification(document, actor, export_format):
        """Create notification when a document is exported"""
        # This is mainly for analytics/tracking purposes
        # Could be used for compliance or usage tracking
        if document.author_id != actor.id:
            title = f"Document exported: '{document.title}'"
            message = f"{actor.username} exported your document as {export_format.upper()}"
            
            Notification.create_notification(
                user_id=document.author_id,
                notification_type=NotificationType.DOCUMENT_EXPORTED,
                title=title,
                message=message,
                document_id=document.id,
                actor_id=actor.id,
                data={'export_format': export_format}
            )
    
    @staticmethod
    def _create_mention_notifications(content, comment, document, actor):
        """Create notifications for mentioned users in content"""
        # Find mentions in format @username
        mentions = re.findall(r'@(\w+)', content)
        
        for username in mentions:
            user = User.query.filter_by(username=username).first()
            if user and user.id != actor.id:
                preferences = NotificationPreference.get_or_create_preferences(user.id)
                if preferences.should_notify(NotificationType.MENTION):
                    title = f"You were mentioned in '{document.title}'"
                    message = f"{actor.username} mentioned you in a comment"
                    
                    Notification.create_notification(
                        user_id=user.id,
                        notification_type=NotificationType.MENTION,
                        title=title,
                        message=message,
                        document_id=document.id,
                        comment_id=comment.id,
                        actor_id=actor.id,
                        data={
                            'mention_context': content[:200] + '...' if len(content) > 200 else content
                        }
                    )
    
    @staticmethod
    def create_bulk_notifications(notification_data_list):
        """Create multiple notifications efficiently"""
        notifications = []
        for data in notification_data_list:
            notification = Notification(
                user_id=data['user_id'],
                type=data['type'],
                title=data['title'],
                message=data['message'],
                document_id=data.get('document_id'),
                comment_id=data.get('comment_id'),
                actor_id=data.get('actor_id'),
                data=data.get('data')
            )
            notifications.append(notification)
        
        db.session.add_all(notifications)
        db.session.commit()
        return notifications
    
    @staticmethod
    def get_notification_summary(user_id):
        """Get notification summary for user"""
        total_count = Notification.query.filter(
            Notification.user_id == user_id,
            Notification.is_deleted == False
        ).count()
        
        unread_count = Notification.get_unread_count(user_id)
        
        # Get counts by type
        type_counts = {}
        for notification_type in NotificationType:
            count = Notification.query.filter(
                Notification.user_id == user_id,
                Notification.type == notification_type,
                Notification.is_read == False,
                Notification.is_deleted == False
            ).count()
            if count > 0:
                type_counts[notification_type.value] = count
        
        return {
            'total_count': total_count,
            'unread_count': unread_count,
            'unread_by_type': type_counts
        }
    
    @staticmethod
    def cleanup_notifications():
        """Clean up old notifications (run periodically)"""
        # Delete old read notifications (older than 30 days)
        Notification.cleanup_old_notifications(days=30)
        
        # Delete very old unread notifications (older than 90 days)
        cutoff_date = datetime.now(timezone.utc) - timedelta(days=90)
        Notification.query.filter(
            Notification.created_at < cutoff_date
        ).delete()
        
        db.session.commit()