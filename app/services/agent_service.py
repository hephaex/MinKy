"""Agent orchestration service.

Provides high-level interface for executing agent tasks and
managing multi-step agent workflows with task chaining.
"""
from typing import Dict, List, Any, Optional
from datetime import datetime
import logging
import uuid

from .agents import (
    get_agent,
    list_agents,
    BaseAgent,
    AgentTask,
    AgentType,
    TaskStatus
)
from .llm_providers import get_provider, list_providers

logger = logging.getLogger(__name__)


class AgentService:
    """Orchestrates agent execution and multi-step workflows.

    Manages agent instances, executes tasks, and supports
    chaining multiple agent tasks together.
    """

    def __init__(self):
        """Initialize the agent service."""
        self._task_history: Dict[str, AgentTask] = {}
        self._active_agents: Dict[str, BaseAgent] = {}

    def execute_task(
        self,
        agent_type: str,
        input_data: Dict[str, Any],
        provider_name: str = "openai",
        api_key: str = "",
        model: Optional[str] = None,
        **agent_config
    ) -> AgentTask:
        """Execute a single agent task.

        Args:
            agent_type: Type of agent to use ('research', 'writing', 'coding')
            input_data: Input data for the task
            provider_name: LLM provider to use
            api_key: API key for the provider
            model: Optional model override
            **agent_config: Additional agent configuration

        Returns:
            AgentTask with result or error
        """
        # Create task
        task = AgentTask(
            type=AgentType(agent_type) if agent_type in [t.value for t in AgentType] else AgentType.GENERAL,
            input_data=input_data
        )

        try:
            # Get provider
            provider = get_provider(provider_name, api_key=api_key)
            if not provider:
                task.mark_failed(f"Provider '{provider_name}' not available")
                return task

            # Get agent
            agent = get_agent(agent_type, provider, model=model, **agent_config)
            if not agent:
                task.mark_failed(f"Agent '{agent_type}' not available")
                return task

            # Execute task
            task.mark_running()
            task = agent.execute(task)

            # Store in history
            self._task_history[task.id] = task

            return task

        except Exception as e:
            logger.error(f"Error executing agent task: {e}")
            task.mark_failed(str(e))
            return task

    def execute_chain(
        self,
        steps: List[Dict[str, Any]],
        provider_name: str = "openai",
        api_key: str = ""
    ) -> List[AgentTask]:
        """Execute a chain of agent tasks.

        Each step can use the output of the previous step as input.
        Steps can specify different agent types and models.

        Args:
            steps: List of step configurations, each containing:
                   - agent_type: Type of agent
                   - input_data: Input for this step (can reference previous outputs)
                   - model: Optional model override
                   - use_previous_output: If True, merge previous output into input
            provider_name: Default LLM provider
            api_key: API key for the provider

        Returns:
            List of AgentTask objects for each step

        Example:
            steps = [
                {'agent_type': 'research', 'input_data': {'query': 'AI trends'}},
                {'agent_type': 'writing', 'use_previous_output': True,
                 'input_data': {'task': 'summarize'}},
            ]
            results = service.execute_chain(steps, 'anthropic', api_key)
        """
        results: List[AgentTask] = []
        previous_output: Optional[Dict[str, Any]] = None

        for i, step in enumerate(steps):
            agent_type = step.get('agent_type', 'general')
            input_data = step.get('input_data', {}).copy()
            model = step.get('model')
            step_provider = step.get('provider', provider_name)
            step_api_key = step.get('api_key', api_key)
            use_previous = step.get('use_previous_output', False)

            # Merge previous output if requested
            if use_previous and previous_output:
                input_data['previous_output'] = previous_output

            # Add chain context
            input_data['_chain_context'] = {
                'step_index': i,
                'total_steps': len(steps),
                'chain_id': str(uuid.uuid4()) if i == 0 else results[0].metadata.get('chain_id')
            }

            # Execute step
            task = self.execute_task(
                agent_type=agent_type,
                input_data=input_data,
                provider_name=step_provider,
                api_key=step_api_key,
                model=model
            )

            # Store chain ID in first task
            if i == 0:
                task.metadata['chain_id'] = input_data['_chain_context']['chain_id']

            results.append(task)

            # Check for failure
            if task.status == TaskStatus.FAILED:
                logger.warning(f"Chain step {i} failed, aborting chain")
                break

            # Store output for next step
            previous_output = task.result

        return results

    def get_task(self, task_id: str) -> Optional[AgentTask]:
        """Get a task by ID.

        Args:
            task_id: Task identifier

        Returns:
            AgentTask or None if not found
        """
        return self._task_history.get(task_id)

    def get_task_history(
        self,
        limit: int = 50,
        agent_type: Optional[str] = None
    ) -> List[AgentTask]:
        """Get recent task history.

        Args:
            limit: Maximum number of tasks to return
            agent_type: Optional filter by agent type

        Returns:
            List of recent tasks
        """
        tasks = list(self._task_history.values())

        if agent_type:
            tasks = [t for t in tasks if t.type.value == agent_type]

        # Sort by created_at descending
        tasks.sort(key=lambda t: t.created_at, reverse=True)

        return tasks[:limit]

    def get_available_agents(self) -> List[Dict[str, Any]]:
        """Get list of available agents with their capabilities.

        Returns:
            List of agent information dictionaries
        """
        agents_info = []
        for agent_name in list_agents():
            agents_info.append({
                'name': agent_name,
                'type': agent_name,
                'available': True
            })
        return agents_info

    def get_available_providers(self) -> List[Dict[str, Any]]:
        """Get list of available LLM providers.

        Returns:
            List of provider information dictionaries
        """
        providers_info = []
        for provider_name in list_providers():
            providers_info.append({
                'name': provider_name,
                'available': True
            })
        return providers_info

    def get_service_status(self) -> Dict[str, Any]:
        """Get overall service status.

        Returns:
            Dictionary with service status information
        """
        return {
            'status': 'operational',
            'available_agents': self.get_available_agents(),
            'available_providers': self.get_available_providers(),
            'task_history_count': len(self._task_history),
            'timestamp': datetime.utcnow().isoformat()
        }


# Global agent service instance
agent_service = AgentService()
