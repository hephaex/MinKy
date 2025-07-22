"""
AI Suggestions API Routes
Provides endpoints for AI-powered content suggestions and auto-completion
"""

from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app.services.ai_service import ai_service
from app.utils.auth import require_auth
import logging

logger = logging.getLogger(__name__)

ai_suggestions_bp = Blueprint('ai_suggestions', __name__)

@ai_suggestions_bp.route('/ai/suggestions', methods=['POST'])
@jwt_required(optional=True)
def get_content_suggestions():
    """
    Get AI-powered content suggestions
    """
    try:
        if not ai_service.is_enabled():
            return jsonify({
                'success': False,
                'error': 'AI service is not enabled. Please configure OpenAI API key.'
            }), 503
        
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        content = data.get('content', '')
        cursor_position = data.get('cursor_position')
        max_suggestions = data.get('max_suggestions', 3)
        
        if not content:
            return jsonify({
                'success': False,
                'error': 'Content is required'
            }), 400
        
        suggestions = ai_service.get_content_suggestions(
            content=content,
            cursor_position=cursor_position,
            max_suggestions=max_suggestions
        )
        
        return jsonify({
            'success': True,
            'suggestions': suggestions
        })
        
    except Exception as e:
        logger.error(f"Error getting content suggestions: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/autocomplete', methods=['POST'])
@jwt_required(optional=True)
def get_autocomplete():
    """
    Get auto-completion suggestion
    """
    try:
        if not ai_service.is_enabled():
            return jsonify({
                'success': False,
                'error': 'AI service is not enabled'
            }), 503
        
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        content = data.get('content', '')
        cursor_position = data.get('cursor_position', len(content))
        
        completion = ai_service.get_auto_completion(
            content=content,
            cursor_position=cursor_position
        )
        
        return jsonify({
            'success': True,
            'completion': completion
        })
        
    except Exception as e:
        logger.error(f"Error getting auto-completion: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/suggest-tags', methods=['POST'])
@jwt_required(optional=True)
def suggest_tags():
    """
    Get AI-powered tag suggestions
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        content = data.get('content', '')
        title = data.get('title', '')
        
        if not content and not title:
            return jsonify({
                'success': False,
                'error': 'Content or title is required'
            }), 400
        
        suggested_tags = ai_service.suggest_tags(
            content=content,
            title=title
        )
        
        return jsonify({
            'success': True,
            'suggested_tags': suggested_tags
        })
        
    except Exception as e:
        logger.error(f"Error getting tag suggestions: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/suggest-title', methods=['POST'])
@jwt_required(optional=True)
def suggest_title():
    """
    Get AI-powered title suggestion
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        content = data.get('content', '')
        
        if not content:
            return jsonify({
                'success': False,
                'error': 'Content is required'
            }), 400
        
        suggested_title = ai_service.suggest_title(content=content)
        
        return jsonify({
            'success': True,
            'suggested_title': suggested_title
        })
        
    except Exception as e:
        logger.error(f"Error getting title suggestion: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/writing-suggestions', methods=['POST'])
@jwt_required(optional=True)
def get_writing_suggestions():
    """
    Get AI-powered writing improvement suggestions
    """
    try:
        if not ai_service.is_enabled():
            return jsonify({
                'success': False,
                'error': 'AI service is not enabled'
            }), 503
        
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        content = data.get('content', '')
        
        if not content:
            return jsonify({
                'success': False,
                'error': 'Content is required'
            }), 400
        
        suggestions = ai_service.get_writing_suggestions(content=content)
        
        return jsonify({
            'success': True,
            'suggestions': suggestions
        })
        
    except Exception as e:
        logger.error(f"Error getting writing suggestions: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/config', methods=['POST'])
def save_ai_config():
    """
    Save AI configuration settings
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No configuration data provided'
            }), 400
        
        # Validate required fields
        required_fields = ['ocrService', 'llmProvider', 'llmModel']
        for field in required_fields:
            if field not in data:
                return jsonify({
                    'success': False,
                    'error': f'Missing required field: {field}'
                }), 400
        
        # Save configuration to AI service
        config_saved = ai_service.save_config(data)
        
        if config_saved:
            return jsonify({
                'success': True,
                'message': 'AI configuration saved successfully'
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Failed to save AI configuration'
            }), 500
        
    except Exception as e:
        logger.error(f"Error saving AI configuration: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/test/<service>', methods=['POST'])
def test_ai_service(service):
    """
    Test AI service connection
    """
    try:
        data = request.get_json()
        logger.info(f"AI test endpoint received data: {data}")
        logger.info(f"Request headers: {dict(request.headers)}")
        if not data:
            return jsonify({
                'success': False,
                'error': 'No configuration data provided'
            }), 400
        
        # Test the specific service connection
        test_result = ai_service.test_connection(service, data)
        
        if test_result['success']:
            return jsonify({
                'success': True,
                'message': f'{service.upper()} connection test successful'
            })
        else:
            return jsonify({
                'success': False,
                'error': test_result.get('error', f'{service.upper()} connection test failed')
            }), 400
        
    except Exception as e:
        logger.error(f"Error testing {service} connection: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/status', methods=['GET'])
def get_ai_status():
    """
    Get AI service status
    """
    try:
        return jsonify({
            'success': True,
            'enabled': ai_service.is_enabled(),
            'features': {
                'content_suggestions': ai_service.is_enabled(),
                'auto_completion': ai_service.is_enabled(),
                'tag_suggestions': True,  # Always available (fallback)
                'title_suggestions': True,  # Always available (fallback)
                'writing_suggestions': ai_service.is_enabled()
            }
        })
        
    except Exception as e:
        logger.error(f"Error getting AI status: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500