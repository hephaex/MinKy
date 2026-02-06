from flask import Blueprint, jsonify, Response
from app import db
from sqlalchemy import text

health_bp = Blueprint('health', __name__)

@health_bp.route('/health', methods=['GET'])
def health_check() -> tuple[Response, int]:
    """Basic health check endpoint"""
    try:
        # Check database connection
        db.session.execute(text('SELECT 1'))
        
        return jsonify({
            'status': 'healthy',
            'service': 'minky-api',
            'database': 'connected'
        }), 200
        
    except Exception as e:
        return jsonify({
            'status': 'unhealthy',
            'service': 'minky-api',
            'database': 'disconnected',
            'error': str(e)
        }), 503

@health_bp.route('/health/detailed', methods=['GET'])
def detailed_health_check() -> tuple[Response, int]:
    """Detailed health check with more information"""
    try:
        # Check database connection and get some stats
        result = db.session.execute(text('SELECT COUNT(*) as doc_count FROM documents'))
        doc_count = result.fetchone()[0]
        
        result = db.session.execute(text('SELECT COUNT(*) as user_count FROM users'))
        user_count = result.fetchone()[0]
        
        return jsonify({
            'status': 'healthy',
            'service': 'minky-api',
            'database': {
                'status': 'connected',
                'documents': doc_count,
                'users': user_count
            },
            'version': '1.0.0'
        }), 200
        
    except Exception as e:
        return jsonify({
            'status': 'unhealthy',
            'service': 'minky-api',
            'database': {
                'status': 'disconnected',
                'error': str(e)
            },
            'version': '1.0.0'
        }), 503