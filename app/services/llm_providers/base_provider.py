"""Abstract base class for LLM providers."""
from abc import ABC, abstractmethod
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field
from enum import Enum
import logging

logger = logging.getLogger(__name__)


class ProviderType(Enum):
    """Supported LLM provider types."""
    OPENAI = "openai"
    ANTHROPIC = "anthropic"
    GOOGLE = "google"
    LOCAL = "local"


@dataclass
class LLMMessage:
    """Standard message format for LLM conversations."""
    role: str  # "system", "user", "assistant"
    content: str


@dataclass
class LLMResponse:
    """Standardized response from any LLM provider."""
    content: str
    model: str
    provider: str
    usage: Dict[str, int] = field(default_factory=dict)
    finish_reason: Optional[str] = None
    raw_response: Optional[Dict[str, Any]] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            'content': self.content,
            'model': self.model,
            'provider': self.provider,
            'usage': self.usage,
            'finish_reason': self.finish_reason
        }


@dataclass
class ModelInfo:
    """Information about an available model."""
    id: str
    name: str
    max_tokens: int
    supports_vision: bool = False
    supports_functions: bool = False
    description: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            'id': self.id,
            'name': self.name,
            'max_tokens': self.max_tokens,
            'supports_vision': self.supports_vision,
            'supports_functions': self.supports_functions,
            'description': self.description
        }


class BaseLLMProvider(ABC):
    """Abstract base class for LLM providers.

    All LLM providers must implement this interface to ensure
    consistent behavior across different backends.
    """

    def __init__(self, api_key: str, base_url: Optional[str] = None, **kwargs):
        """Initialize provider with API key and optional configuration.

        Args:
            api_key: API key for authentication
            base_url: Optional custom base URL for the API
            **kwargs: Additional provider-specific configuration
        """
        self.api_key = api_key
        self.base_url = base_url
        self.config = kwargs
        self._client = None

    @abstractmethod
    def _initialize_client(self) -> None:
        """Initialize the API client. Called lazily on first use."""
        pass

    @abstractmethod
    def complete(
        self,
        messages: List[LLMMessage],
        model: Optional[str] = None,
        max_tokens: int = 1000,
        temperature: float = 0.7,
        stop: Optional[List[str]] = None,
        **kwargs
    ) -> LLMResponse:
        """Generate a completion from the given messages.

        Args:
            messages: List of conversation messages
            model: Model identifier (uses default if not specified)
            max_tokens: Maximum tokens in response
            temperature: Sampling temperature (0.0 - 1.0)
            stop: Optional stop sequences
            **kwargs: Additional provider-specific parameters

        Returns:
            LLMResponse with generated content
        """
        pass

    @abstractmethod
    def get_available_models(self) -> List[ModelInfo]:
        """Return list of available models for this provider.

        Returns:
            List of ModelInfo objects describing available models
        """
        pass

    @abstractmethod
    def test_connection(self) -> Dict[str, Any]:
        """Test the provider connection and return status.

        Returns:
            Dictionary with 'success', 'message', and optional 'details'
        """
        pass

    @property
    @abstractmethod
    def provider_type(self) -> ProviderType:
        """Return the provider type identifier."""
        pass

    @property
    @abstractmethod
    def default_model(self) -> str:
        """Return the default model identifier for this provider."""
        pass

    def validate_api_key(self) -> bool:
        """Validate API key format (basic validation, not authentication)."""
        if not self.api_key:
            return False
        return len(self.api_key) > 10

    def _ensure_client(self) -> None:
        """Ensure client is initialized before use."""
        if self._client is None:
            self._initialize_client()

    def _convert_messages(self, messages: List[LLMMessage]) -> List[Dict[str, str]]:
        """Convert LLMMessage list to provider-specific format.

        Default implementation returns standard format. Override if needed.
        """
        return [{'role': msg.role, 'content': msg.content} for msg in messages]

    def _handle_error(self, error: Exception, context: str) -> LLMResponse:
        """Handle and log errors consistently.

        Args:
            error: The exception that occurred
            context: Description of what was being attempted

        Returns:
            LLMResponse with error information
        """
        logger.error(f"{self.provider_type.value} {context}: {error}")
        return LLMResponse(
            content=f"Error: {context}",
            model="unknown",
            provider=self.provider_type.value,
            finish_reason="error",
            raw_response={'error': str(error)}
        )
