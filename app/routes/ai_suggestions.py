"""
AI Suggestions API Routes
Provides endpoints for AI-powered content suggestions and auto-completion
"""

from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from app import limiter
from app.services.ai_service import ai_service
from app.utils.auth import get_current_user_id, get_current_user
import logging

logger = logging.getLogger(__name__)

ai_suggestions_bp = Blueprint('ai_suggestions', __name__)

# Rate limit constants for AI endpoints (expensive operations)
AI_RATE_LIMIT = "10 per minute"
AI_BURST_LIMIT = "30 per hour"

# Maximum content size for AI endpoints (50KB)
MAX_CONTENT_SIZE = 50 * 1024

@ai_suggestions_bp.route('/ai/suggestions', methods=['POST'])
@limiter.limit(AI_RATE_LIMIT)
@limiter.limit(AI_BURST_LIMIT)
@jwt_required()  # SECURITY: Require auth to prevent cost exhaustion attacks
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

        # SECURITY: Validate max_suggestions parameter
        if not isinstance(max_suggestions, int) or not (1 <= max_suggestions <= 10):
            max_suggestions = 3  # Use default if invalid

        # SECURITY: Validate cursor_position parameter
        if cursor_position is not None:
            if not isinstance(cursor_position, int) or cursor_position < 0:
                cursor_position = len(content) if content else 0
            else:
                cursor_position = min(cursor_position, len(content) if content else 0)

        if not content:
            return jsonify({
                'success': False,
                'error': 'Content is required'
            }), 400

        if len(content) > MAX_CONTENT_SIZE:
            return jsonify({
                'success': False,
                'error': f'Content too large. Maximum size is {MAX_CONTENT_SIZE // 1024}KB'
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
@limiter.limit(AI_RATE_LIMIT)
@limiter.limit(AI_BURST_LIMIT)
@jwt_required()  # SECURITY: Require auth to prevent cost exhaustion attacks
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
        cursor_position = data.get('cursor_position')

        # SECURITY: Validate cursor_position parameter
        if cursor_position is None:
            cursor_position = len(content) if content else 0
        elif not isinstance(cursor_position, int) or cursor_position < 0:
            cursor_position = len(content) if content else 0
        else:
            cursor_position = min(cursor_position, len(content) if content else 0)

        if len(content) > MAX_CONTENT_SIZE:
            return jsonify({
                'success': False,
                'error': f'Content too large. Maximum size is {MAX_CONTENT_SIZE // 1024}KB'
            }), 400

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

# SECURITY: Maximum title length for AI endpoints
MAX_TITLE_SIZE = 500

@ai_suggestions_bp.route('/ai/suggest-tags', methods=['POST'])
@limiter.limit(AI_RATE_LIMIT)
@limiter.limit(AI_BURST_LIMIT)
@jwt_required()  # SECURITY: Require auth to prevent cost exhaustion attacks
def suggest_tags():
    """
    Get AI-powered tag suggestions
    """
    try:
        # SECURITY: Check if AI service is enabled for consistency
        ai_enabled = ai_service.is_enabled()

        data = request.get_json()
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400

        content = data.get('content', '')
        title = data.get('title', '')

        # SECURITY: Validate title length
        if len(title) > MAX_TITLE_SIZE:
            return jsonify({
                'success': False,
                'error': f'Title too long. Maximum size is {MAX_TITLE_SIZE} characters'
            }), 400

        if not content and not title:
            return jsonify({
                'success': False,
                'error': 'Content or title is required'
            }), 400

        if len(content) > MAX_CONTENT_SIZE:
            return jsonify({
                'success': False,
                'error': f'Content too large. Maximum size is {MAX_CONTENT_SIZE // 1024}KB'
            }), 400

        suggested_tags = ai_service.suggest_tags(
            content=content,
            title=title
        )

        return jsonify({
            'success': True,
            'suggested_tags': suggested_tags,
            'ai_powered': ai_enabled  # Inform client whether AI was used
        })
        
    except Exception as e:
        logger.error(f"Error getting tag suggestions: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/suggest-title', methods=['POST'])
@limiter.limit(AI_RATE_LIMIT)
@limiter.limit(AI_BURST_LIMIT)
@jwt_required()  # SECURITY: Require auth to prevent cost exhaustion attacks
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

        if len(content) > MAX_CONTENT_SIZE:
            return jsonify({
                'success': False,
                'error': f'Content too large. Maximum size is {MAX_CONTENT_SIZE // 1024}KB'
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
@limiter.limit(AI_RATE_LIMIT)
@limiter.limit(AI_BURST_LIMIT)
@jwt_required()  # SECURITY: Require auth to prevent cost exhaustion attacks
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

        if len(content) > MAX_CONTENT_SIZE:
            return jsonify({
                'success': False,
                'error': f'Content too large. Maximum size is {MAX_CONTENT_SIZE // 1024}KB'
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

@ai_suggestions_bp.route('/ai/config', methods=['GET'])
@limiter.limit("30 per minute")  # SECURITY: Rate limit config endpoint
@jwt_required()
def get_ai_config():
    """
    Get current AI configuration settings (requires authentication)
    """
    try:
        config = ai_service.get_config()
        return jsonify({
            'success': True,
            'config': config
        })

    except Exception as e:
        logger.error(f"Error getting AI configuration: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/config', methods=['POST'])
@limiter.limit("10 per minute")  # SECURITY: Rate limit config changes
@jwt_required()
def save_ai_config():
    """
    Save AI configuration settings (requires admin privileges)
    """
    try:
        # Require admin privileges for config changes
        user = get_current_user()
        if not user or not user.is_admin:
            return jsonify({
                'success': False,
                'error': 'Admin privileges required'
            }), 403

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

        # SECURITY: Validate config field values
        VALID_LLM_PROVIDERS = {'openai', 'anthropic', 'google', 'local'}
        VALID_OCR_SERVICES = {'tesseract', 'google-vision', 'none'}
        MAX_MODEL_NAME_LENGTH = 100

        llm_provider = data.get('llmProvider', '')
        if llm_provider not in VALID_LLM_PROVIDERS:
            return jsonify({
                'success': False,
                'error': f'Invalid LLM provider. Allowed: {", ".join(VALID_LLM_PROVIDERS)}'
            }), 400

        ocr_service = data.get('ocrService', '')
        if ocr_service not in VALID_OCR_SERVICES:
            return jsonify({
                'success': False,
                'error': f'Invalid OCR service. Allowed: {", ".join(VALID_OCR_SERVICES)}'
            }), 400

        llm_model = data.get('llmModel', '')
        if len(llm_model) > MAX_MODEL_NAME_LENGTH:
            return jsonify({
                'success': False,
                'error': f'Model name too long (max {MAX_MODEL_NAME_LENGTH} characters)'
            }), 400

        # Save configuration to AI service
        config_saved = ai_service.save_config(data)

        if config_saved:
            logger.info(f"AI configuration updated by user {get_current_user_id()}")
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

# SECURITY: Whitelist of allowed services for testing
ALLOWED_TEST_SERVICES = {'llm', 'ocr'}

@ai_suggestions_bp.route('/ai/test/<service>', methods=['POST'])
@limiter.limit("5 per minute")
@jwt_required()
def test_ai_service(service):
    """
    Test AI service connection (requires authentication)
    """
    try:
        # SECURITY: Validate service parameter against whitelist
        if service not in ALLOWED_TEST_SERVICES:
            return jsonify({
                'success': False,
                'error': f'Invalid service. Allowed: {", ".join(ALLOWED_TEST_SERVICES)}'
            }), 400

        data = request.get_json()
        # Removed debug logging that exposed sensitive data
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

@ai_suggestions_bp.route('/ai/health', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def ai_health_check():
    """
    Perform health check on AI services (authenticated)
    """
    try:
        # Use internal config (with real API keys) instead of masked config
        config = ai_service.config
        health_status = {
            'ocr': None,
            'llm': None
        }
        
        # Test OCR service
        ocr_result = ai_service.test_connection('ocr', config)
        health_status['ocr'] = ocr_result['success']
        
        # Test LLM service
        llm_result = ai_service.test_connection('llm', config)
        health_status['llm'] = llm_result['success']
        
        return jsonify({
            'success': True,
            'health': health_status,
            'details': {
                'ocr': ocr_result.get('error') if not ocr_result['success'] else 'Connected',
                'llm': llm_result.get('error') if not llm_result['success'] else 'Connected'
            }
        })
        
    except Exception as e:
        logger.error(f"Error performing health check: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500

@ai_suggestions_bp.route('/ai/status', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def get_ai_status():
    """
    Get AI service status (authenticated)
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