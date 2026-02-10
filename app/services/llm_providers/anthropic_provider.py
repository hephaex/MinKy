"""Anthropic Claude LLM Provider implementation."""
from typing import List, Dict, Any, Optional
import logging

try:
    import anthropic
    ANTHROPIC_AVAILABLE = True
except ImportError:
    ANTHROPIC_AVAILABLE = False

from . import register_provider
from .base_provider import (
    BaseLLMProvider,
    LLMMessage,
    LLMResponse,
    ModelInfo,
    ProviderType
)

logger = logging.getLogger(__name__)


@register_provider('anthropic')
class AnthropicProvider(BaseLLMProvider):
    """Anthropic Claude API provider implementation.

    Supports Claude 4, Claude 3.7, Claude 3.5, and Claude 3 model families.
    """

    # Available models with their specifications
    MODELS = {
        # Claude 4 models (latest)
        'claude-opus-4-20250514': ModelInfo(
            id='claude-opus-4-20250514',
            name='Claude Opus 4',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Most capable model with deepest reasoning'
        ),
        'claude-sonnet-4-20250514': ModelInfo(
            id='claude-sonnet-4-20250514',
            name='Claude Sonnet 4',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Best balance of performance and speed'
        ),
        # Claude 3.7 models
        'claude-3-7-sonnet-20250219': ModelInfo(
            id='claude-3-7-sonnet-20250219',
            name='Claude 3.7 Sonnet',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Enhanced Sonnet with extended thinking'
        ),
        # Claude 3.5 models
        'claude-3-5-sonnet-20241022': ModelInfo(
            id='claude-3-5-sonnet-20241022',
            name='Claude 3.5 Sonnet',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='High performance Sonnet model'
        ),
        'claude-3-5-haiku-20241022': ModelInfo(
            id='claude-3-5-haiku-20241022',
            name='Claude 3.5 Haiku',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Fast and efficient Haiku model'
        ),
        # Claude 3 models (legacy)
        'claude-3-opus-20240229': ModelInfo(
            id='claude-3-opus-20240229',
            name='Claude 3 Opus',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Legacy powerful model'
        ),
        'claude-3-sonnet-20240229': ModelInfo(
            id='claude-3-sonnet-20240229',
            name='Claude 3 Sonnet',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Legacy balanced model'
        ),
        'claude-3-haiku-20240307': ModelInfo(
            id='claude-3-haiku-20240307',
            name='Claude 3 Haiku',
            max_tokens=200000,
            supports_vision=True,
            supports_functions=True,
            description='Legacy fast model'
        ),
    }

    DEFAULT_MODEL = 'claude-sonnet-4-20250514'

    def __init__(self, api_key: str, base_url: Optional[str] = None, **kwargs):
        """Initialize Anthropic provider.

        Args:
            api_key: Anthropic API key (starts with 'sk-ant-')
            base_url: Optional custom base URL
            **kwargs: Additional configuration
        """
        super().__init__(api_key, base_url, **kwargs)

        if not ANTHROPIC_AVAILABLE:
            raise ImportError(
                "anthropic package not installed. "
                "Install with: pip install anthropic"
            )

        self._client = None

    def _initialize_client(self) -> None:
        """Initialize the Anthropic client."""
        try:
            client_kwargs = {'api_key': self.api_key}
            if self.base_url:
                client_kwargs['base_url'] = self.base_url

            self._client = anthropic.Anthropic(**client_kwargs)
            logger.debug("Anthropic client initialized successfully")
        except Exception as e:
            logger.error(f"Failed to initialize Anthropic client: {e}")
            raise

    def complete(
        self,
        messages: List[LLMMessage],
        model: Optional[str] = None,
        max_tokens: int = 1000,
        temperature: float = 0.7,
        stop: Optional[List[str]] = None,
        **kwargs
    ) -> LLMResponse:
        """Generate completion using Anthropic API.

        Args:
            messages: List of conversation messages
            model: Model identifier (defaults to claude-3-5-sonnet)
            max_tokens: Maximum tokens in response
            temperature: Sampling temperature (0.0 - 1.0)
            stop: Optional stop sequences
            **kwargs: Additional Anthropic-specific parameters

        Returns:
            LLMResponse with generated content
        """
        self._ensure_client()

        model = model or self.default_model

        try:
            # Extract system message if present
            system_message = None
            api_messages = []

            for msg in messages:
                if msg.role == 'system':
                    system_message = msg.content
                else:
                    api_messages.append({
                        'role': msg.role,
                        'content': msg.content
                    })

            # Build request kwargs
            request_kwargs = {
                'model': model,
                'messages': api_messages,
                'max_tokens': max_tokens,
                'temperature': temperature,
            }

            if system_message:
                request_kwargs['system'] = system_message

            if stop:
                request_kwargs['stop_sequences'] = stop

            # Add any additional kwargs
            request_kwargs.update(kwargs)

            response = self._client.messages.create(**request_kwargs)

            # Extract content from response
            content = ''
            if response.content:
                for block in response.content:
                    if hasattr(block, 'text'):
                        content += block.text

            usage = {
                'prompt_tokens': response.usage.input_tokens,
                'completion_tokens': response.usage.output_tokens,
                'total_tokens': response.usage.input_tokens + response.usage.output_tokens
            }

            return LLMResponse(
                content=content,
                model=response.model,
                provider=self.provider_type.value,
                usage=usage,
                finish_reason=response.stop_reason,
                raw_response={'id': response.id}
            )

        except Exception as e:
            return self._handle_error(e, "completion failed")

    def get_available_models(self) -> List[ModelInfo]:
        """Return list of available Anthropic models."""
        return list(self.MODELS.values())

    def test_connection(self) -> Dict[str, Any]:
        """Test Anthropic API connection.

        Returns:
            Dictionary with connection status
        """
        try:
            self._ensure_client()

            # Make a minimal API call to verify credentials
            response = self._client.messages.create(
                model='claude-3-haiku-20240307',
                messages=[{'role': 'user', 'content': 'Hi'}],
                max_tokens=5
            )

            return {
                'success': True,
                'message': 'Anthropic connection successful',
                'details': {
                    'model_used': response.model,
                    'response_id': response.id
                }
            }

        except Exception as e:
            logger.error(f"Anthropic connection test failed: {e}")
            return {
                'success': False,
                'message': 'Anthropic connection failed',
                'error': str(e)
            }

    @property
    def provider_type(self) -> ProviderType:
        """Return Anthropic provider type."""
        return ProviderType.ANTHROPIC

    @property
    def default_model(self) -> str:
        """Return default Anthropic model."""
        return self.config.get('model', self.DEFAULT_MODEL)

    def validate_api_key(self) -> bool:
        """Validate Anthropic API key format."""
        if not self.api_key:
            return False
        # Anthropic keys start with 'sk-ant-' and are typically 100+ characters
        return self.api_key.startswith('sk-ant-') and len(self.api_key) > 50
