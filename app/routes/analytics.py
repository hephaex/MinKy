"""
Analytics API Routes
Provides endpoints for dashboard analytics and reporting
"""

from flask import Blueprint, jsonify, request
from flask_jwt_extended import jwt_required
from app import limiter
from app.services.analytics_service import AnalyticsService, get_comprehensive_analytics
from app.utils.auth import get_current_user
import logging

logger = logging.getLogger(__name__)

analytics_bp = Blueprint('analytics', __name__)

def require_admin():
    """Check if current user is admin and active"""
    user = get_current_user()
    # SECURITY: Check both is_admin AND is_active
    return user and user.is_active and user.is_admin

def _log_analytics_access(user, endpoint: str) -> None:
    """SECURITY: Audit log analytics access for compliance"""
    if user:
        logger.info(f"AUDIT: Admin user {user.username} (id={user.id}) accessed analytics/{endpoint}")

@analytics_bp.route('/analytics/dashboard', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_dashboard_analytics():
    """Get comprehensive dashboard analytics"""
    # SECURITY: Require admin access for system-wide analytics
    if not require_admin():
        return jsonify({'error': 'Admin access required'}), 403

    # SECURITY: Audit log access
    _log_analytics_access(get_current_user(), 'dashboard')

    try:
        analytics_data = get_comprehensive_analytics()
        
        if analytics_data:
            return jsonify({
                'success': True,
                'data': analytics_data
            })
        else:
            return jsonify({'error': 'Failed to generate analytics'}), 500
            
    except Exception as e:
        logger.error(f"Error in dashboard analytics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/overview', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_overview_stats():
    """Get basic overview statistics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        # SECURITY: Audit log access
        _log_analytics_access(get_current_user(), 'overview')

        stats = AnalyticsService.get_dashboard_stats()

        if stats:
            return jsonify({
                'success': True,
                'data': stats
            })
        else:
            return jsonify({'error': 'Failed to generate overview stats'}), 500

    except Exception as e:
        logger.error(f"Error in overview stats endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/activity', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_activity_timeline():
    """Get document activity timeline"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        days = request.args.get('days', 30, type=int)
        # SECURITY: Validate days parameter bounds - return error instead of silently correcting
        if days < 1 or days > 365:
            return jsonify({
                'error': 'Invalid days parameter',
                'message': 'days must be between 1 and 365'
            }), 400

        # SECURITY: Audit log access
        _log_analytics_access(get_current_user(), f'activity?days={days}')
        
        timeline = AnalyticsService.get_document_activity_timeline(days)
        
        return jsonify({
            'success': True,
            'data': timeline,
            'period_days': days
        })
        
    except Exception as e:
        logger.error(f"Error in activity timeline endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/users', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_user_analytics():
    """Get user engagement analytics (anonymized)"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        # SECURITY: Audit log access to user analytics
        _log_analytics_access(get_current_user(), 'users')

        engagement_data = AnalyticsService.get_user_engagement_metrics()
        
        return jsonify({
            'success': True,
            'data': engagement_data
        })
        
    except Exception as e:
        logger.error(f"Error in user analytics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/content', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_content_analytics():
    """Get content-related analytics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        # SECURITY: Audit log access
        _log_analytics_access(get_current_user(), 'content')

        content_data = AnalyticsService.get_content_analytics()

        return jsonify({
            'success': True,
            'data': content_data
        })

    except Exception as e:
        logger.error(f"Error in content analytics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/performance', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_performance_metrics():
    """Get system performance metrics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        # SECURITY: Audit log access
        _log_analytics_access(get_current_user(), 'performance')

        performance_data = AnalyticsService.get_performance_metrics()

        return jsonify({
            'success': True,
            'data': performance_data
        })

    except Exception as e:
        logger.error(f"Error in performance metrics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/export', methods=['GET'])
@limiter.limit("3 per hour")  # SECURITY: Stricter rate limit for expensive export operation
@jwt_required()
def export_analytics():
    """Export analytics data in various formats"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        # SECURITY: Audit log export access
        _log_analytics_access(get_current_user(), 'export')

        format_type = request.args.get('format', 'json').lower()

        # SECURITY: Validate format parameter against whitelist
        # Note: Only json is currently implemented
        ALLOWED_FORMATS = {'json'}
        if format_type not in ALLOWED_FORMATS:
            # SECURITY: Don't reveal internal implementation details
            return jsonify({
                'error': 'Invalid format parameter'
            }), 400

        analytics_data = get_comprehensive_analytics()
        
        if format_type == 'json':
            return jsonify({
                'success': True,
                'data': analytics_data,
                'format': 'json'
            })
        else:
            return jsonify({'error': 'Unsupported export format'}), 400
            
    except Exception as e:
        logger.error(f"Error in analytics export endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500