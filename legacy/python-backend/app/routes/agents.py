"""Agent API endpoints for task execution and management."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import limiter
from app.services.agent_service import agent_service
from app.services.agents import list_agents, AgentType
from app.services.llm_providers import list_providers
import logging

logger = logging.getLogger(__name__)

agents_bp = Blueprint('agents', __name__)

# SECURITY: Maximum chain steps to prevent resource exhaustion
MAX_CHAIN_STEPS = 10


@agents_bp.route('/agents/execute', methods=['POST'])
@limiter.limit("10 per minute")
@jwt_required()
def execute_agent_task():
    """Execute a single agent task.

    Headers:
        X-LLM-API-Key: API key for the LLM provider

    Request body:
    {
        "agent_type": "research|writing|coding",
        "input_data": {...},
        "provider": "openai|anthropic",
        "model": "optional-model-override"
    }

    Returns:
        AgentTask result
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'Request body is required'}), 400

        agent_type = data.get('agent_type')
        if not agent_type:
            return jsonify({'error': 'agent_type is required'}), 400

        available_agents = list_agents()
        if agent_type not in available_agents:
            return jsonify({
                'error': f'Unknown agent type: {agent_type}',
                'available': available_agents
            }), 400

        input_data = data.get('input_data', {})

        # SECURITY: Validate input_data size to prevent resource exhaustion
        import json
        MAX_INPUT_SIZE = 100 * 1024  # 100KB
        try:
            input_size = len(json.dumps(input_data))
            if input_size > MAX_INPUT_SIZE:
                return jsonify({
                    'error': f'input_data too large (max {MAX_INPUT_SIZE // 1024}KB)'
                }), 400
        except (TypeError, ValueError):
            return jsonify({'error': 'input_data must be JSON serializable'}), 400

        provider_name = data.get('provider', 'openai')
        # Get API key from header instead of request body for security
        api_key = request.headers.get('X-LLM-API-Key', '')
        model = data.get('model')

        if not api_key:
            return jsonify({'error': 'X-LLM-API-Key header is required'}), 400

        # SECURITY: Get current user for task ownership (IDOR protection)
        current_user_id = get_jwt_identity()

        # Execute the task with user_id for ownership tracking
        task = agent_service.execute_task(
            agent_type=agent_type,
            input_data=input_data,
            provider_name=provider_name,
            api_key=api_key,
            model=model,
            user_id=current_user_id
        )

        return jsonify({
            'task': task.to_dict(),
            'success': task.status.value == 'completed'
        })

    except Exception as e:
        logger.error(f"Error executing agent task: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/chain', methods=['POST'])
@limiter.limit("5 per minute")
@jwt_required()
def execute_agent_chain():
    """Execute a chain of agent tasks.

    Headers:
        X-LLM-API-Key: API key for the LLM provider

    Request body:
    {
        "steps": [
            {
                "agent_type": "research",
                "input_data": {"query": "..."},
                "model": "optional"
            },
            {
                "agent_type": "writing",
                "input_data": {"task_type": "summarize"},
                "use_previous_output": true
            }
        ],
        "provider": "openai"
    }

    Returns:
        List of AgentTask results
    """
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'Request body is required'}), 400

        steps = data.get('steps', [])
        if not steps:
            return jsonify({'error': 'steps array is required'}), 400

        # SECURITY: Limit chain steps to prevent resource exhaustion and cost abuse
        if len(steps) > MAX_CHAIN_STEPS:
            return jsonify({
                'error': f'Too many steps. Maximum allowed: {MAX_CHAIN_STEPS}',
                'provided': len(steps)
            }), 400

        # SECURITY: Validate each step has required fields
        available_agents = list_agents()
        for i, step in enumerate(steps):
            if not isinstance(step, dict):
                return jsonify({'error': f'Step {i} must be an object'}), 400
            step_agent = step.get('agent_type')
            if not step_agent:
                return jsonify({'error': f'Step {i} missing agent_type'}), 400
            if step_agent not in available_agents:
                return jsonify({'error': f'Step {i} has invalid agent_type: {step_agent}'}), 400

        provider_name = data.get('provider', 'openai')
        # Get API key from header instead of request body for security
        api_key = request.headers.get('X-LLM-API-Key', '')

        if not api_key:
            return jsonify({'error': 'X-LLM-API-Key header is required'}), 400

        # SECURITY: Get current user for task ownership (IDOR protection)
        current_user_id = get_jwt_identity()

        # Execute the chain with user_id for ownership tracking
        results = agent_service.execute_chain(
            steps=steps,
            provider_name=provider_name,
            api_key=api_key,
            user_id=current_user_id
        )

        return jsonify({
            'tasks': [task.to_dict() for task in results],
            'total_steps': len(steps),
            'completed_steps': sum(1 for t in results if t.status.value == 'completed'),
            'success': all(t.status.value == 'completed' for t in results)
        })

    except Exception as e:
        logger.error(f"Error executing agent chain: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/task/<task_id>', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def get_task_status(task_id: str):
    """Get status of a specific agent task.

    Returns:
        AgentTask details
    """
    try:
        from flask_jwt_extended import get_jwt_identity
        current_user_id = get_jwt_identity()

        task = agent_service.get_task(task_id)
        if not task:
            return jsonify({'error': 'Task not found'}), 404

        # SECURITY: Verify user owns this task (IDOR protection)
        # Deny access if user_id is None (task without ownership) or doesn't match
        if task.user_id is None or str(task.user_id) != str(current_user_id):
            return jsonify({'error': 'Access denied'}), 403

        return jsonify({
            'task': task.to_dict()
        })

    except Exception as e:
        logger.error(f"Error getting task status: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/history', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def get_task_history():
    """Get recent task history for current user.

    Query params:
    - limit: Max number of tasks (default 50, max 100)
    - agent_type: Filter by agent type

    Returns:
        List of recent tasks owned by the current user
    """
    try:
        # SECURITY: Cap limit to prevent resource exhaustion
        limit = min(request.args.get('limit', 50, type=int), 100)
        agent_type = request.args.get('agent_type')

        # SECURITY: Get current user for filtering (IDOR protection)
        current_user_id = get_jwt_identity()

        tasks = agent_service.get_task_history(
            limit=limit,
            agent_type=agent_type,
            user_id=current_user_id  # Filter by user
        )

        return jsonify({
            'tasks': [task.to_dict() for task in tasks],
            'count': len(tasks)
        })

    except Exception as e:
        logger.error(f"Error getting task history: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/types', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def list_agent_types():
    """List available agent types and their capabilities.

    Returns:
        List of agent information
    """
    try:
        agents = agent_service.get_available_agents()

        # Add descriptions for each agent type
        agent_descriptions = {
            'research': {
                'description': 'Information gathering and analysis',
                'capabilities': ['analyze', 'summarize', 'extract', 'compare', 'answer'],
                'best_for': 'Research questions, content analysis, information extraction'
            },
            'writing': {
                'description': 'Content generation and editing',
                'capabilities': ['generate', 'edit', 'rewrite', 'expand', 'condense'],
                'best_for': 'Creating articles, editing text, adapting content'
            },
            'coding': {
                'description': 'Code analysis, generation, and review',
                'capabilities': ['generate', 'review', 'debug', 'explain', 'refactor'],
                'best_for': 'Writing code, fixing bugs, code reviews'
            }
        }

        for agent in agents:
            if agent['name'] in agent_descriptions:
                agent.update(agent_descriptions[agent['name']])

        return jsonify({
            'agents': agents
        })

    except Exception as e:
        logger.error(f"Error listing agent types: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/providers', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def list_llm_providers():
    """List available LLM providers.

    Returns:
        List of provider information
    """
    try:
        providers = agent_service.get_available_providers()

        # Add descriptions for each provider
        provider_descriptions = {
            'openai': {
                'description': 'OpenAI GPT models',
                'models': ['gpt-4o', 'gpt-4o-mini', 'gpt-4-turbo', 'gpt-4', 'gpt-3.5-turbo'],
                'default_model': 'gpt-4o-mini'
            },
            'anthropic': {
                'description': 'Anthropic Claude 4 models',
                'models': ['claude-opus-4-20250514', 'claude-sonnet-4-20250514'],
                'default_model': 'claude-sonnet-4-20250514'
            }
        }

        for provider in providers:
            if provider['name'] in provider_descriptions:
                provider.update(provider_descriptions[provider['name']])

        return jsonify({
            'providers': providers
        })

    except Exception as e:
        logger.error(f"Error listing providers: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/status', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def get_agent_service_status():
    """Get agent service status.

    Returns:
        Service status information
    """
    try:
        status = agent_service.get_service_status()
        return jsonify(status)

    except Exception as e:
        logger.error(f"Error getting service status: {e}")
        return jsonify({'error': 'Internal server error'}), 500
