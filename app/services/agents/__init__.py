"""Agent registry and factory.

This module provides a unified interface for managing AI agents.
Agents are registered using the @register_agent decorator and can be
instantiated via the get_agent factory function.

Example:
    from app.services.agents import get_agent, list_agents
    from app.services.llm_providers import get_provider

    # Get a provider
    provider = get_provider('anthropic', api_key='sk-ant-...')

    # Get an agent
    agent = get_agent('research', provider)
    task = AgentTask(type=AgentType.RESEARCH, input_data={'query': '...'})
    result = agent.execute(task)

    # List available agents
    agents = list_agents()
"""
from typing import Dict, Type, Optional, List
import logging

from .base_agent import (
    BaseAgent,
    AgentTask,
    AgentStep,
    AgentType,
    TaskStatus
)

logger = logging.getLogger(__name__)

# Agent registry: maps agent type to agent class
_agent_registry: Dict[str, Type[BaseAgent]] = {}


def register_agent(name: str):
    """Decorator to register an agent class.

    Args:
        name: Unique identifier for the agent (e.g., 'research', 'writing')

    Example:
        @register_agent('research')
        class ResearchAgent(BaseAgent):
            ...
    """
    def decorator(cls: Type[BaseAgent]):
        if name in _agent_registry:
            logger.warning(f"Agent '{name}' is being re-registered")
        _agent_registry[name] = cls
        logger.debug(f"Registered agent: {name}")
        return cls
    return decorator


def get_agent(
    name: str,
    provider: 'BaseLLMProvider',
    model: Optional[str] = None,
    **kwargs
) -> Optional[BaseAgent]:
    """Factory function to create an agent instance.

    Args:
        name: Agent identifier (e.g., 'research', 'writing')
        provider: LLM provider instance
        model: Optional model override
        **kwargs: Additional agent configuration

    Returns:
        Agent instance or None if agent not found
    """
    agent_cls = _agent_registry.get(name)
    if agent_cls is None:
        logger.error(f"Agent '{name}' not found. Available: {list(_agent_registry.keys())}")
        return None

    try:
        return agent_cls(provider=provider, model=model, config=kwargs)
    except Exception as e:
        logger.error(f"Failed to instantiate agent '{name}': {e}")
        return None


def list_agents() -> List[str]:
    """Return list of registered agent names."""
    return list(_agent_registry.keys())


def get_agent_class(name: str) -> Optional[Type[BaseAgent]]:
    """Get the agent class without instantiating.

    Args:
        name: Agent identifier

    Returns:
        Agent class or None if not found
    """
    return _agent_registry.get(name)


def is_agent_registered(name: str) -> bool:
    """Check if an agent is registered."""
    return name in _agent_registry


# Import agents to trigger registration
try:
    from .research_agent import ResearchAgent
except ImportError as e:
    logger.debug(f"Research agent not available: {e}")

try:
    from .writing_agent import WritingAgent
except ImportError as e:
    logger.debug(f"Writing agent not available: {e}")

try:
    from .coding_agent import CodingAgent
except ImportError as e:
    logger.debug(f"Coding agent not available: {e}")


__all__ = [
    'BaseAgent',
    'AgentTask',
    'AgentStep',
    'AgentType',
    'TaskStatus',
    'register_agent',
    'get_agent',
    'list_agents',
    'get_agent_class',
    'is_agent_registered',
]
