from flask import Blueprint, jsonify, Response
from flask_jwt_extended import jwt_required
from app import db, cache, limiter
from app.utils.auth import get_current_user_id
from app.models.user import User
from sqlalchemy import text
import logging

logger = logging.getLogger(__name__)

health_bp = Blueprint('health', __name__)

@health_bp.route('/health', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
@cache.cached(timeout=10)
def health_check() -> tuple[Response, int]:
    """Basic health check endpoint
    ---
    tags:
      - Health
    responses:
      200:
        description: Service is healthy
        schema:
          type: object
          properties:
            status:
              type: string
              example: healthy
            service:
              type: string
              example: minky-api
            database:
              type: string
              example: connected
      503:
        description: Service is unhealthy
    """
    try:
        # Check database connection
        db.session.execute(text('SELECT 1'))
        
        return jsonify({
            'status': 'healthy',
            'service': 'minky-api',
            'database': 'connected'
        }), 200
        
    except Exception as e:
        logger.error("Health check failed: %s", e)
        return jsonify({
            'status': 'unhealthy',
            'service': 'minky-api',
            'database': 'disconnected'
        }), 503

@health_bp.route('/health/detailed', methods=['GET'])
@limiter.limit("30 per minute")  # SECURITY: Rate limiting
@jwt_required()
def detailed_health_check() -> tuple[Response, int]:
    """Detailed health check with more information (admin only)
    ---
    tags:
      - Health
    responses:
      200:
        description: Detailed service health information
        schema:
          type: object
          properties:
            status:
              type: string
              example: healthy
            service:
              type: string
              example: minky-api
            database:
              type: object
              properties:
                status:
                  type: string
                  example: connected
                documents:
                  type: integer
                users:
                  type: integer
            version:
              type: string
              example: 1.0.0
      403:
        description: Admin access required
      503:
        description: Service is unhealthy
    """
    try:
        # SECURITY: Require admin role to view detailed statistics
        # This prevents information disclosure about system size
        current_user_id = get_current_user_id()
        user = db.session.get(User, current_user_id)
        if not user or not user.is_admin:
            return jsonify({
                'error': 'Admin access required for detailed health info'
            }), 403

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
        logger.error("Detailed health check failed: %s", e)
        return jsonify({
            'status': 'unhealthy',
            'service': 'minky-api',
            'database': {
                'status': 'disconnected'
            },
            'version': '1.0.0'
        }), 503