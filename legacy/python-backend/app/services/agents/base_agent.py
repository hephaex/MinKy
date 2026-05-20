"""Abstract base class for AI agents."""
from abc import ABC, abstractmethod
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field
from enum import Enum
from datetime import datetime, timezone
import uuid
import logging

from ..llm_providers import BaseLLMProvider, LLMMessage, LLMResponse


def _utc_now() -> datetime:
    """Return current UTC datetime with timezone info."""
    return datetime.now(timezone.utc)

logger = logging.getLogger(__name__)


class AgentType(Enum):
    """Types of specialized agents."""
    RESEARCH = "research"
    WRITING = "writing"
    CODING = "coding"
    GENERAL = "general"


class TaskStatus(Enum):
    """Status of an agent task."""
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"


@dataclass
class AgentTask:
    """Represents a task to be executed by an agent."""
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    type: AgentType = AgentType.GENERAL
    input_data: Dict[str, Any] = field(default_factory=dict)
    status: TaskStatus = TaskStatus.PENDING
    result: Optional[Dict[str, Any]] = None
    error: Optional[str] = None
    created_at: datetime = field(default_factory=_utc_now)
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None
    metadata: Dict[str, Any] = field(default_factory=dict)
    user_id: Optional[int] = None  # SECURITY: Track task owner for authorization

    def to_dict(self, include_user_id: bool = False) -> Dict[str, Any]:
        """Convert task to dictionary.

        Args:
            include_user_id: Whether to include user_id in output (for internal use only)
        """
        result = {
            'id': self.id,
            'type': self.type.value,
            'input_data': self.input_data,
            'status': self.status.value,
            'result': self.result,
            'error': self.error,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'started_at': self.started_at.isoformat() if self.started_at else None,
            'completed_at': self.completed_at.isoformat() if self.completed_at else None,
            'metadata': self.metadata
        }
        if include_user_id:
            result['user_id'] = self.user_id
        return result

    def mark_running(self) -> None:
        """Mark task as running."""
        self.status = TaskStatus.RUNNING
        self.started_at = _utc_now()

    def mark_completed(self, result: Dict[str, Any]) -> None:
        """Mark task as completed with result."""
        self.status = TaskStatus.COMPLETED
        self.result = result
        self.completed_at = _utc_now()

    def mark_failed(self, error: str) -> None:
        """Mark task as failed with error."""
        self.status = TaskStatus.FAILED
        self.error = error
        self.completed_at = _utc_now()


@dataclass
class AgentStep:
    """Represents a single step in agent execution."""
    index: int
    action: str
    input_data: Dict[str, Any]
    output_data: Optional[Dict[str, Any]] = None
    reasoning: Optional[str] = None
    timestamp: datetime = field(default_factory=_utc_now)

    def to_dict(self) -> Dict[str, Any]:
        """Convert step to dictionary."""
        return {
            'index': self.index,
            'action': self.action,
            'input_data': self.input_data,
            'output_data': self.output_data,
            'reasoning': self.reasoning,
            'timestamp': self.timestamp.isoformat()
        }


class BaseAgent(ABC):
    """Abstract base class for specialized AI agents.

    Agents encapsulate specific capabilities and system prompts
    for different types of tasks (research, writing, coding).
    """

    def __init__(
        self,
        provider: BaseLLMProvider,
        model: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None
    ):
        """Initialize agent with LLM provider.

        Args:
            provider: LLM provider instance
            model: Optional model override
            config: Additional configuration
        """
        self.provider = provider
        self.model = model
        self.config = config or {}
        self.conversation_history: List[LLMMessage] = []
        self.execution_steps: List[AgentStep] = []
        self._step_counter = 0

    @abstractmethod
    def execute(self, task: AgentTask) -> AgentTask:
        """Execute the agent's primary task.

        Args:
            task: The task to execute

        Returns:
            Updated task with result or error
        """
        pass

    @abstractmethod
    def get_system_prompt(self) -> str:
        """Return the agent's system prompt.

        Returns:
            System prompt string defining agent behavior
        """
        pass

    @property
    @abstractmethod
    def agent_type(self) -> AgentType:
        """Return the agent type identifier."""
        pass

    @property
    def name(self) -> str:
        """Return agent name."""
        return f"{self.agent_type.value}_agent"

    def _add_step(
        self,
        action: str,
        input_data: Dict[str, Any],
        output_data: Optional[Dict[str, Any]] = None,
        reasoning: Optional[str] = None
    ) -> AgentStep:
        """Add an execution step.

        Args:
            action: Description of the action
            input_data: Input for this step
            output_data: Output from this step
            reasoning: Agent's reasoning for this step

        Returns:
            The created AgentStep
        """
        step = AgentStep(
            index=self._step_counter,
            action=action,
            input_data=input_data,
            output_data=output_data,
            reasoning=reasoning
        )
        self.execution_steps.append(step)
        self._step_counter += 1
        return step

    def _call_llm(
        self,
        user_message: str,
        include_history: bool = True,
        max_tokens: int = 1000,
        temperature: float = 0.7
    ) -> LLMResponse:
        """Make an LLM call with the agent's system prompt.

        Args:
            user_message: User message to send
            include_history: Whether to include conversation history
            max_tokens: Maximum tokens in response
            temperature: Sampling temperature

        Returns:
            LLM response
        """
        messages = [LLMMessage(role='system', content=self.get_system_prompt())]

        if include_history:
            messages.extend(self.conversation_history)

        user_msg = LLMMessage(role='user', content=user_message)
        messages.append(user_msg)

        response = self.provider.complete(
            messages=messages,
            model=self.model,
            max_tokens=max_tokens,
            temperature=temperature
        )

        # Add to conversation history
        self.conversation_history.append(user_msg)
        self.conversation_history.append(
            LLMMessage(role='assistant', content=response.content)
        )

        return response

    def reset_conversation(self) -> None:
        """Clear conversation history and execution steps."""
        self.conversation_history = []
        self.execution_steps = []
        self._step_counter = 0

    def get_execution_trace(self) -> List[Dict[str, Any]]:
        """Get the execution trace as a list of dictionaries."""
        return [step.to_dict() for step in self.execution_steps]

    def get_capabilities(self) -> Dict[str, Any]:
        """Return agent capabilities and configuration.

        Returns:
            Dictionary describing agent capabilities
        """
        return {
            'type': self.agent_type.value,
            'name': self.name,
            'provider': self.provider.provider_type.value,
            'model': self.model or self.provider.default_model,
            'config': self.config
        }
