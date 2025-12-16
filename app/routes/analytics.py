"""
Analytics API Routes
Provides endpoints for dashboard analytics and reporting
"""

from flask import Blueprint, jsonify, request
from flask_jwt_extended import jwt_required
from app.services.analytics_service import AnalyticsService, get_comprehensive_analytics
from app.utils.auth import get_current_user
import logging

logger = logging.getLogger(__name__)

analytics_bp = Blueprint('analytics', __name__)

def require_admin():
    """Check if current user is admin"""
    user = get_current_user()
    return user and user.is_admin

@analytics_bp.route('/analytics/dashboard', methods=['GET'])
def get_dashboard_analytics():
    """Get comprehensive dashboard analytics"""
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
@jwt_required()
def get_overview_stats():
    """Get basic overview statistics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
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
@jwt_required()
def get_activity_timeline():
    """Get document activity timeline"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        days = request.args.get('days', 30, type=int)
        if days > 365:  # Limit to 1 year
            days = 365
        
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
@jwt_required()
def get_user_analytics():
    """Get user engagement analytics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        engagement_data = AnalyticsService.get_user_engagement_metrics()
        
        return jsonify({
            'success': True,
            'data': engagement_data
        })
        
    except Exception as e:
        logger.error(f"Error in user analytics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/content', methods=['GET'])
@jwt_required()
def get_content_analytics():
    """Get content-related analytics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        content_data = AnalyticsService.get_content_analytics()
        
        return jsonify({
            'success': True,
            'data': content_data
        })
        
    except Exception as e:
        logger.error(f"Error in content analytics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/performance', methods=['GET'])
@jwt_required()
def get_performance_metrics():
    """Get system performance metrics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        performance_data = AnalyticsService.get_performance_metrics()
        
        return jsonify({
            'success': True,
            'data': performance_data
        })
        
    except Exception as e:
        logger.error(f"Error in performance metrics endpoint: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@analytics_bp.route('/analytics/export', methods=['GET'])
@jwt_required()
def export_analytics():
    """Export analytics data in various formats"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        format_type = request.args.get('format', 'json').lower()
        
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