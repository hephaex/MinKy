"""
Authentication utility functions for Minky
"""

from functools import wraps
from flask import jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app.models.user import User


def get_current_user_id():
    """
    Get the current authenticated user's ID from JWT token.
    Returns None if not authenticated or user not found.
    Use this to reduce duplicated get_jwt_identity() calls across routes.
    """
    try:
        return get_jwt_identity()
    except Exception:
        return None


def get_current_user():
    """
    Get the current authenticated user object.
    Returns None if not authenticated or user not found.
    """
    user_id = get_current_user_id()
    if user_id is None:
        return None
    return User.query.get(user_id)


def require_auth(f):
    """Decorator to require authentication for routes"""
    @wraps(f)
    @jwt_required()
    def decorated_function(*args, **kwargs):
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        
        if not user or not user.is_active:
            return jsonify({'error': 'User not found or inactive'}), 404
            
        return f(*args, **kwargs)
    return decorated_function


def admin_required(f):
    """Decorator to require admin privileges for routes"""
    @wraps(f)
    @jwt_required()
    def decorated_function(*args, **kwargs):
        current_user_id = get_jwt_identity()
        user = User.query.get(current_user_id)
        
        if not user or not user.is_active:
            return jsonify({'error': 'User not found or inactive'}), 404
            
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required'}), 403
            
        return f(*args, **kwargs)
    return decorated_function