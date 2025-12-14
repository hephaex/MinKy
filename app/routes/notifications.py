from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import jwt_required
from app.models.notification import Notification, NotificationPreference
from app.models.user import User
from app.utils.auth import get_current_user_id, get_current_user
from app.services.notification_service import NotificationService
from marshmallow import Schema, fields, ValidationError

notifications_bp = Blueprint('notifications', __name__)

class NotificationPreferenceSchema(Schema):
    document_comments = fields.Bool()
    document_ratings = fields.Bool()
    document_shares = fields.Bool()
    document_updates = fields.Bool()
    mentions = fields.Bool()
    follows = fields.Bool()
    template_usage = fields.Bool()
    email_notifications = fields.Bool()
    push_notifications = fields.Bool()
    digest_frequency = fields.Str(validate=lambda x: x in ['none', 'daily', 'weekly'])

@notifications_bp.route('/notifications', methods=['GET'])
@jwt_required()
def get_notifications():
    """Get notifications for the current user"""
    current_user_id = get_current_user_id()
    user = get_current_user()

    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    # Get query parameters
    limit = min(int(request.args.get('limit', 50)), 100)  # Max 100 notifications
    offset = int(request.args.get('offset', 0))
    unread_only = request.args.get('unread_only', 'false').lower() == 'true'
    
    try:
        notifications = Notification.get_user_notifications(
            user_id=current_user_id,
            limit=limit,
            offset=offset,
            unread_only=unread_only
        )
        
        # Get summary data
        summary = NotificationService.get_notification_summary(current_user_id)
        
        return jsonify({
            'notifications': [notification.to_dict() for notification in notifications],
            'summary': summary,
            'pagination': {
                'limit': limit,
                'offset': offset,
                'has_more': len(notifications) == limit
            }
        })
        
    except Exception as e:
        current_app.logger.error(f"Error fetching notifications: {str(e)}")
        return jsonify({'error': 'Failed to fetch notifications'}), 500

@notifications_bp.route('/notifications/<int:notification_id>/read', methods=['POST'])
@jwt_required()
def mark_notification_read(notification_id):
    """Mark a specific notification as read"""
    current_user_id = get_current_user_id()
    
    notification = Notification.query.filter(
        Notification.id == notification_id,
        Notification.user_id == current_user_id,
        Notification.is_deleted == False
    ).first()
    
    if not notification:
        return jsonify({'error': 'Notification not found'}), 404
    
    try:
        notification.mark_as_read()
        return jsonify({
            'message': 'Notification marked as read',
            'notification': notification.to_dict()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error marking notification as read: {str(e)}")
        return jsonify({'error': 'Failed to mark notification as read'}), 500

@notifications_bp.route('/notifications/read-all', methods=['POST'])
@jwt_required()
def mark_all_notifications_read():
    """Mark all notifications as read for the current user"""
    current_user_id = get_current_user_id()
    
    try:
        Notification.mark_all_read(current_user_id)
        summary = NotificationService.get_notification_summary(current_user_id)
        
        return jsonify({
            'message': 'All notifications marked as read',
            'summary': summary
        })
        
    except Exception as e:
        current_app.logger.error(f"Error marking all notifications as read: {str(e)}")
        return jsonify({'error': 'Failed to mark notifications as read'}), 500

@notifications_bp.route('/notifications/<int:notification_id>', methods=['DELETE'])
@jwt_required()
def delete_notification(notification_id):
    """Delete a specific notification"""
    current_user_id = get_current_user_id()
    
    notification = Notification.query.filter(
        Notification.id == notification_id,
        Notification.user_id == current_user_id,
        Notification.is_deleted == False
    ).first()
    
    if not notification:
        return jsonify({'error': 'Notification not found'}), 404
    
    try:
        notification.soft_delete()
        return jsonify({'message': 'Notification deleted'})
        
    except Exception as e:
        current_app.logger.error(f"Error deleting notification: {str(e)}")
        return jsonify({'error': 'Failed to delete notification'}), 500

@notifications_bp.route('/notifications/summary', methods=['GET'])
@jwt_required()
def get_notification_summary():
    """Get notification summary for the current user"""
    current_user_id = get_current_user_id()
    
    try:
        summary = NotificationService.get_notification_summary(current_user_id)
        return jsonify(summary)
        
    except Exception as e:
        current_app.logger.error(f"Error getting notification summary: {str(e)}")
        return jsonify({'error': 'Failed to get notification summary'}), 500

@notifications_bp.route('/notifications/preferences', methods=['GET'])
@jwt_required()
def get_notification_preferences():
    """Get notification preferences for the current user"""
    current_user_id = get_current_user_id()
    
    try:
        preferences = NotificationPreference.get_or_create_preferences(current_user_id)
        return jsonify({'preferences': preferences.to_dict()})
        
    except Exception as e:
        current_app.logger.error(f"Error getting notification preferences: {str(e)}")
        return jsonify({'error': 'Failed to get notification preferences'}), 500

@notifications_bp.route('/notifications/preferences', methods=['PUT'])
@jwt_required()
def update_notification_preferences():
    """Update notification preferences for the current user"""
    current_user_id = get_current_user_id()
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    # Validate data
    schema = NotificationPreferenceSchema()
    try:
        validated_data = schema.load(data, partial=True)
    except ValidationError as e:
        return jsonify({'error': 'Invalid data', 'details': e.messages}), 400
    
    try:
        preferences = NotificationPreference.get_or_create_preferences(current_user_id)
        
        # Update preferences
        for key, value in validated_data.items():
            setattr(preferences, key, value)
        
        from app import db
        from datetime import datetime
        preferences.updated_at = datetime.utcnow()
        db.session.commit()
        
        return jsonify({
            'message': 'Notification preferences updated',
            'preferences': preferences.to_dict()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error updating notification preferences: {str(e)}")
        return jsonify({'error': 'Failed to update notification preferences'}), 500

@notifications_bp.route('/notifications/test', methods=['POST'])
@jwt_required()
def create_test_notification():
    """Create a test notification (for testing purposes)"""
    current_user_id = get_current_user_id()
    
    # Only allow in development/testing environments
    if current_app.config.get('ENV') == 'production':
        return jsonify({'error': 'Test notifications not allowed in production'}), 403
    
    data = request.get_json()
    title = data.get('title', 'Test Notification')
    message = data.get('message', 'This is a test notification')
    
    try:
        from app.models.notification import NotificationType
        notification = Notification.create_notification(
            user_id=current_user_id,
            notification_type=NotificationType.DOCUMENT_COMMENTED,  # Use any type for testing
            title=title,
            message=message,
            actor_id=current_user_id,
            data={'test': True}
        )
        
        return jsonify({
            'message': 'Test notification created',
            'notification': notification.to_dict()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error creating test notification: {str(e)}")
        return jsonify({'error': 'Failed to create test notification'}), 500

@notifications_bp.route('/notifications/bulk-actions', methods=['POST'])
@jwt_required()
def bulk_notification_actions():
    """Perform bulk actions on notifications"""
    current_user_id = get_current_user_id()
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    action = data.get('action')  # 'read', 'delete'
    notification_ids = data.get('notification_ids', [])
    
    if not action or not notification_ids:
        return jsonify({'error': 'action and notification_ids required'}), 400
    
    if action not in ['read', 'delete']:
        return jsonify({'error': 'Invalid action. Must be "read" or "delete"'}), 400
    
    try:
        # Get notifications belonging to current user
        notifications = Notification.query.filter(
            Notification.id.in_(notification_ids),
            Notification.user_id == current_user_id,
            Notification.is_deleted == False
        ).all()
        
        if not notifications:
            return jsonify({'error': 'No valid notifications found'}), 404
        
        if action == 'read':
            for notification in notifications:
                notification.mark_as_read()
            message = f"Marked {len(notifications)} notifications as read"
            
        elif action == 'delete':
            for notification in notifications:
                notification.soft_delete()
            message = f"Deleted {len(notifications)} notifications"
        
        summary = NotificationService.get_notification_summary(current_user_id)
        
        return jsonify({
            'message': message,
            'affected_count': len(notifications),
            'summary': summary
        })
        
    except Exception as e:
        current_app.logger.error(f"Error performing bulk notification action: {str(e)}")
        return jsonify({'error': 'Failed to perform bulk action'}), 500