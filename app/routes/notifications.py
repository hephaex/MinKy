import os
import json
from flask import Blueprint, request, current_app
from flask_jwt_extended import jwt_required
from app import limiter
from app.models.notification import Notification, NotificationPreference
from app.utils.auth import get_current_user_id, get_current_user
from app.utils.responses import success_response, error_response
from app.services.notification_service import NotificationService
from marshmallow import Schema, fields, ValidationError
import logging

logger = logging.getLogger(__name__)

notifications_bp = Blueprint('notifications', __name__)


def _log_notification_operation(operation: str, user_id: int, notification_id: int = None,
                                details: dict = None) -> None:
    """SECURITY: Audit log notification operations for compliance."""
    log_entry = {
        'operation': operation,
        'user_id': user_id,
        'notification_id': notification_id,
        'ip_address': request.remote_addr if request else None,
        'details': details or {}
    }
    logger.info(f"AUDIT_NOTIFICATION: {json.dumps(log_entry)}")

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
@limiter.limit("60 per minute")
@jwt_required()
def get_notifications():
    """Get notifications for the current user"""
    current_user_id = get_current_user_id()
    user = get_current_user()

    if not user:
        return error_response('User not found', 404)

    # Get query parameters
    limit = min(request.args.get('limit', 50, type=int), 100)  # Max 100 notifications
    offset = max(request.args.get('offset', 0, type=int), 0)  # Ensure non-negative
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

        return success_response({
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
        return error_response('Failed to fetch notifications', 500)

@notifications_bp.route('/notifications/<int:notification_id>/read', methods=['POST'])
@limiter.limit("60 per minute")
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
        return error_response('Notification not found', 404)

    try:
        notification.mark_as_read()

        # SECURITY: Audit log notification read
        _log_notification_operation('read', current_user_id, notification_id)

        return success_response({
            'message': 'Notification marked as read',
            'notification': notification.to_dict()
        })

    except Exception as e:
        current_app.logger.error(f"Error marking notification as read: {str(e)}")
        return error_response('Failed to mark notification as read', 500)

@notifications_bp.route('/notifications/read-all', methods=['POST'])
@limiter.limit("10 per minute")
@jwt_required()
def mark_all_notifications_read():
    """Mark all notifications as read for the current user"""
    current_user_id = get_current_user_id()

    try:
        Notification.mark_all_read(current_user_id)
        summary = NotificationService.get_notification_summary(current_user_id)

        # SECURITY: Audit log bulk read operation
        _log_notification_operation('read_all', current_user_id)

        return success_response({
            'message': 'All notifications marked as read',
            'summary': summary
        })

    except Exception as e:
        current_app.logger.error(f"Error marking all notifications as read: {str(e)}")
        return error_response('Failed to mark notifications as read', 500)

@notifications_bp.route('/notifications/<int:notification_id>', methods=['DELETE'])
@limiter.limit("30 per minute")
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
        return error_response('Notification not found', 404)

    try:
        notification.soft_delete()

        # SECURITY: Audit log notification deletion
        _log_notification_operation('delete', current_user_id, notification_id)

        return success_response(message='Notification deleted')

    except Exception as e:
        current_app.logger.error(f"Error deleting notification: {str(e)}")
        return error_response('Failed to delete notification', 500)

@notifications_bp.route('/notifications/summary', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
@jwt_required()
def get_notification_summary():
    """Get notification summary for the current user"""
    current_user_id = get_current_user_id()

    try:
        summary = NotificationService.get_notification_summary(current_user_id)
        return success_response(summary)

    except Exception as e:
        current_app.logger.error(f"Error getting notification summary: {str(e)}")
        return error_response('Failed to get notification summary', 500)

@notifications_bp.route('/notifications/preferences', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
@jwt_required()
def get_notification_preferences():
    """Get notification preferences for the current user"""
    current_user_id = get_current_user_id()

    try:
        preferences = NotificationPreference.get_or_create_preferences(current_user_id)
        return success_response({'preferences': preferences.to_dict()})

    except Exception as e:
        current_app.logger.error(f"Error getting notification preferences: {str(e)}")
        return error_response('Failed to get notification preferences', 500)

@notifications_bp.route('/notifications/preferences', methods=['PUT'])
@limiter.limit("30 per minute")
@jwt_required()
def update_notification_preferences():
    """Update notification preferences for the current user"""
    current_user_id = get_current_user_id()

    data = request.get_json()
    if not data:
        return error_response('Request body required', 400)

    # Validate data
    schema = NotificationPreferenceSchema()
    try:
        validated_data = schema.load(data, partial=True)
    except ValidationError as e:
        return error_response('Invalid data', 400, details=e.messages)

    try:
        preferences = NotificationPreference.get_or_create_preferences(current_user_id)

        # SECURITY: Use explicit allowlist for preference fields
        ALLOWED_PREFERENCE_FIELDS = {
            'document_comments', 'document_ratings', 'document_shares',
            'document_updates', 'mentions', 'follows', 'template_usage',
            'email_notifications', 'push_notifications', 'digest_frequency'
        }

        # Update preferences with allowlist validation
        for key, value in validated_data.items():
            if key in ALLOWED_PREFERENCE_FIELDS:
                setattr(preferences, key, value)

        from app import db
        from datetime import datetime, timezone
        preferences.updated_at = datetime.now(timezone.utc)
        db.session.commit()

        # SECURITY: Audit log preference update
        _log_notification_operation(
            'update_preferences',
            current_user_id,
            details={'fields_updated': list(validated_data.keys())}
        )

        return success_response({
            'message': 'Notification preferences updated',
            'preferences': preferences.to_dict()
        })

    except Exception as e:
        current_app.logger.error(f"Error updating notification preferences: {str(e)}")
        return error_response('Failed to update notification preferences', 500)

@notifications_bp.route('/notifications/test', methods=['POST'])
@limiter.limit("5 per minute")
@jwt_required()
def create_test_notification():
    """Create a test notification (for testing purposes)"""
    current_user_id = get_current_user_id()

    # Only allow in development/testing environments
    flask_env = os.getenv('FLASK_ENV', 'production')
    if flask_env == 'production':
        return error_response('Test notifications not allowed in production', 403)

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

        return success_response({
            'message': 'Test notification created',
            'notification': notification.to_dict()
        }, status_code=201)

    except Exception as e:
        current_app.logger.error(f"Error creating test notification: {str(e)}")
        return error_response('Failed to create test notification', 500)

@notifications_bp.route('/notifications/bulk-actions', methods=['POST'])
@limiter.limit("10 per minute")
@jwt_required()
def bulk_notification_actions():
    """Perform bulk actions on notifications"""
    current_user_id = get_current_user_id()

    data = request.get_json()
    if not data:
        return error_response('Request body required', 400)

    action = data.get('action')  # 'read', 'delete'
    notification_ids = data.get('notification_ids', [])

    if not action or not notification_ids:
        return error_response('action and notification_ids required', 400)

    # SECURITY: Limit bulk operation size to prevent resource exhaustion
    MAX_BULK_IDS = 100
    if not isinstance(notification_ids, list):
        return error_response('notification_ids must be a list', 400)
    if len(notification_ids) > MAX_BULK_IDS:
        return error_response(f'Too many notification IDs (max {MAX_BULK_IDS})', 400)

    # SECURITY: Validate each notification_id is a positive integer
    validated_ids = []
    for nid in notification_ids:
        if not isinstance(nid, int) or nid < 1:
            return error_response('Each notification_id must be a positive integer', 400)
        validated_ids.append(nid)
    notification_ids = validated_ids

    if action not in ['read', 'delete']:
        return error_response('Invalid action. Must be "read" or "delete"', 400)

    try:
        # Get notifications belonging to current user
        notifications = Notification.query.filter(
            Notification.id.in_(notification_ids),
            Notification.user_id == current_user_id,
            Notification.is_deleted == False
        ).all()

        if not notifications:
            return error_response('No valid notifications found', 404)

        if action == 'read':
            for notification in notifications:
                notification.mark_as_read()
            message = f"Marked {len(notifications)} notifications as read"

        elif action == 'delete':
            for notification in notifications:
                notification.soft_delete()
            message = f"Deleted {len(notifications)} notifications"

        summary = NotificationService.get_notification_summary(current_user_id)

        # SECURITY: Audit log bulk action
        _log_notification_operation(
            f'bulk_{action}',
            current_user_id,
            details={'count': len(notifications), 'notification_ids': notification_ids}
        )

        return success_response({
            'message': message,
            'affected_count': len(notifications),
            'summary': summary
        })

    except Exception as e:
        current_app.logger.error(f"Error performing bulk notification action: {str(e)}")
        return error_response('Failed to perform bulk action', 500)