from app import db
from datetime import datetime, timedelta, timezone
from enum import Enum
from sqlalchemy import Index


def utc_now():
    """Return current UTC time as timezone-aware datetime."""
    return datetime.now(timezone.utc)


class NotificationType(Enum):
    DOCUMENT_COMMENTED = "document_commented"
    DOCUMENT_RATED = "document_rated"
    DOCUMENT_SHARED = "document_shared"
    DOCUMENT_UPDATED = "document_updated"
    DOCUMENT_VERSION_CREATED = "document_version_created"
    DOCUMENT_LIKED = "document_liked"
    MENTION = "mention"
    FOLLOW = "follow"
    TEMPLATE_USED = "template_used"
    DOCUMENT_EXPORTED = "document_exported"

class Notification(db.Model):
    __tablename__ = 'notifications'
    
    id = db.Column(db.Integer, primary_key=True)
    
    # Recipient
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    
    # Notification details
    type = db.Column(db.Enum(NotificationType), nullable=False)
    title = db.Column(db.String(200), nullable=False)
    message = db.Column(db.Text, nullable=False)
    
    # Related entities
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=True)
    comment_id = db.Column(db.Integer, db.ForeignKey('comments.id'), nullable=True)
    actor_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=True)  # Who triggered the notification
    
    # Metadata
    data = db.Column(db.JSON, nullable=True)  # Additional data specific to notification type
    
    # Status
    is_read = db.Column(db.Boolean, default=False, nullable=False)
    is_deleted = db.Column(db.Boolean, default=False, nullable=False)
    
    # Timestamps
    created_at = db.Column(db.DateTime, default=utc_now, nullable=False)
    read_at = db.Column(db.DateTime, nullable=True)
    
    # Relationships
    user = db.relationship('User', foreign_keys=[user_id], backref='notifications')
    document = db.relationship('Document', backref='notifications')
    comment = db.relationship('Comment', backref='notifications')
    actor = db.relationship('User', foreign_keys=[actor_id])
    
    # Indexes for performance
    __table_args__ = (
        Index('idx_notifications_user_created', 'user_id', 'created_at'),
        Index('idx_notifications_user_unread', 'user_id', 'is_read'),
        Index('idx_notifications_document', 'document_id'),
        Index('idx_notifications_type', 'type'),
    )
    
    def mark_as_read(self):
        """Mark notification as read"""
        if not self.is_read:
            self.is_read = True
            self.read_at = datetime.now(timezone.utc)
            db.session.commit()
    
    def soft_delete(self):
        """Soft delete notification"""
        self.is_deleted = True
        db.session.commit()
    
    def to_dict(self):
        """Convert notification to dictionary"""
        return {
            'id': self.id,
            'type': self.type.value,
            'title': self.title,
            'message': self.message,
            'document_id': self.document_id,
            'comment_id': self.comment_id,
            'actor_id': self.actor_id,
            'actor': self.actor.to_dict() if self.actor else None,
            'document': {
                'id': self.document.id,
                'title': self.document.title
            } if self.document else None,
            'data': self.data,
            'is_read': self.is_read,
            'created_at': self.created_at.isoformat(),
            'read_at': self.read_at.isoformat() if self.read_at else None
        }
    
    @classmethod
    def create_notification(cls, user_id, notification_type, title, message, 
                           document_id=None, comment_id=None, actor_id=None, data=None):
        """Create a new notification"""
        notification = cls(
            user_id=user_id,
            type=notification_type,
            title=title,
            message=message,
            document_id=document_id,
            comment_id=comment_id,
            actor_id=actor_id,
            data=data
        )
        
        db.session.add(notification)
        db.session.commit()
        return notification
    
    @classmethod
    def get_user_notifications(cls, user_id, limit=50, offset=0, unread_only=False):
        """Get notifications for a user"""
        query = cls.query.filter(
            cls.user_id == user_id,
            cls.is_deleted == False
        )
        
        if unread_only:
            query = query.filter(cls.is_read == False)
        
        return query.order_by(cls.created_at.desc()).limit(limit).offset(offset).all()
    
    @classmethod
    def get_unread_count(cls, user_id):
        """Get count of unread notifications for a user"""
        return cls.query.filter(
            cls.user_id == user_id,
            cls.is_read == False,
            cls.is_deleted == False
        ).count()
    
    @classmethod
    def mark_all_read(cls, user_id):
        """Mark all notifications as read for a user"""
        cls.query.filter(
            cls.user_id == user_id,
            cls.is_read == False,
            cls.is_deleted == False
        ).update({
            'is_read': True,
            'read_at': datetime.now(timezone.utc)
        })
        db.session.commit()
    
    @classmethod
    def cleanup_old_notifications(cls, days=30):
        """Clean up old read notifications"""
        cutoff_date = datetime.now(timezone.utc) - timedelta(days=days)
        cls.query.filter(
            cls.is_read == True,
            cls.created_at < cutoff_date
        ).delete()
        db.session.commit()

class NotificationPreference(db.Model):
    __tablename__ = 'notification_preferences'
    
    id = db.Column(db.Integer, primary_key=True)
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    
    # Notification type preferences
    document_comments = db.Column(db.Boolean, default=True, nullable=False)
    document_ratings = db.Column(db.Boolean, default=True, nullable=False)
    document_shares = db.Column(db.Boolean, default=True, nullable=False)
    document_updates = db.Column(db.Boolean, default=False, nullable=False)  # Only for owned docs
    mentions = db.Column(db.Boolean, default=True, nullable=False)
    follows = db.Column(db.Boolean, default=True, nullable=False)
    template_usage = db.Column(db.Boolean, default=True, nullable=False)
    
    # Delivery preferences
    email_notifications = db.Column(db.Boolean, default=False, nullable=False)
    push_notifications = db.Column(db.Boolean, default=True, nullable=False)
    
    # Frequency settings
    digest_frequency = db.Column(db.String(20), default='daily', nullable=False)  # none, daily, weekly
    
    # Timestamps
    created_at = db.Column(db.DateTime, default=utc_now, nullable=False)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now, nullable=False)
    
    # Relationships
    user = db.relationship('User', backref=db.backref('notification_preferences', uselist=False))
    
    def to_dict(self):
        """Convert preferences to dictionary"""
        return {
            'id': self.id,
            'user_id': self.user_id,
            'document_comments': self.document_comments,
            'document_ratings': self.document_ratings,
            'document_shares': self.document_shares,
            'document_updates': self.document_updates,
            'mentions': self.mentions,
            'follows': self.follows,
            'template_usage': self.template_usage,
            'email_notifications': self.email_notifications,
            'push_notifications': self.push_notifications,
            'digest_frequency': self.digest_frequency,
            'updated_at': self.updated_at.isoformat()
        }
    
    @classmethod
    def get_or_create_preferences(cls, user_id):
        """Get or create notification preferences for user"""
        preferences = cls.query.filter_by(user_id=user_id).first()
        if not preferences:
            preferences = cls(user_id=user_id)
            db.session.add(preferences)
            db.session.commit()
        return preferences
    
    def should_notify(self, notification_type):
        """Check if user should be notified for this type"""
        type_mapping = {
            NotificationType.DOCUMENT_COMMENTED: self.document_comments,
            NotificationType.DOCUMENT_RATED: self.document_ratings,
            NotificationType.DOCUMENT_SHARED: self.document_shares,
            NotificationType.DOCUMENT_UPDATED: self.document_updates,
            NotificationType.MENTION: self.mentions,
            NotificationType.FOLLOW: self.follows,
            NotificationType.TEMPLATE_USED: self.template_usage,
        }
        return type_mapping.get(notification_type, True)