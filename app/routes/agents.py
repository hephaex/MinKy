"""Agent API endpoints for task execution and management."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import limiter
from app.services.agent_service import agent_service
from app.services.agents import list_agents, AgentType
from app.services.llm_providers import list_providers
from app.utils.auth import get_current_user_id
import logging

logger = logging.getLogger(__name__)

agents_bp = Blueprint('agents', __name__)


@agents_bp.route('/agents/execute', methods=['POST'])
@limiter.limit("10 per minute")
@jwt_required()
def execute_agent_task():
    """Execute a single agent task.

    Request body:
    {
        "agent_type": "research|writing|coding",
        "input_data": {...},
        "provider": "openai|anthropic",
        "api_key": "sk-...",
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
        provider_name = data.get('provider', 'openai')
        api_key = data.get('api_key', '')
        model = data.get('model')

        if not api_key:
            return jsonify({'error': 'api_key is required'}), 400

        # Execute the task
        task = agent_service.execute_task(
            agent_type=agent_type,
            input_data=input_data,
            provider_name=provider_name,
            api_key=api_key,
            model=model
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
        "provider": "openai",
        "api_key": "sk-..."
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

        provider_name = data.get('provider', 'openai')
        api_key = data.get('api_key', '')

        if not api_key:
            return jsonify({'error': 'api_key is required'}), 400

        # Execute the chain
        results = agent_service.execute_chain(
            steps=steps,
            provider_name=provider_name,
            api_key=api_key
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
@jwt_required()
def get_task_status(task_id: str):
    """Get status of a specific agent task.

    Returns:
        AgentTask details
    """
    try:
        task = agent_service.get_task(task_id)
        if not task:
            return jsonify({'error': 'Task not found'}), 404

        return jsonify({
            'task': task.to_dict()
        })

    except Exception as e:
        logger.error(f"Error getting task status: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/history', methods=['GET'])
@jwt_required()
def get_task_history():
    """Get recent task history.

    Query params:
    - limit: Max number of tasks (default 50)
    - agent_type: Filter by agent type

    Returns:
        List of recent tasks
    """
    try:
        limit = request.args.get('limit', 50, type=int)
        agent_type = request.args.get('agent_type')

        tasks = agent_service.get_task_history(limit=limit, agent_type=agent_type)

        return jsonify({
            'tasks': [task.to_dict() for task in tasks],
            'count': len(tasks)
        })

    except Exception as e:
        logger.error(f"Error getting task history: {e}")
        return jsonify({'error': 'Internal server error'}), 500


@agents_bp.route('/agents/types', methods=['GET'])
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
                'description': 'Anthropic Claude models',
                'models': ['claude-opus-4-20250514', 'claude-sonnet-4-20250514', 'claude-3-7-sonnet-20250219', 'claude-3-5-sonnet-20241022', 'claude-3-5-haiku-20241022'],
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
