"""OpenAI LLM Provider implementation."""
from typing import List, Dict, Any, Optional
import logging

from openai import OpenAI

from . import register_provider
from .base_provider import (
    BaseLLMProvider,
    LLMMessage,
    LLMResponse,
    ModelInfo,
    ProviderType
)

logger = logging.getLogger(__name__)


@register_provider('openai')
class OpenAIProvider(BaseLLMProvider):
    """OpenAI API provider implementation.

    Supports GPT-3.5-turbo, GPT-4, GPT-4-turbo, and GPT-4o models.
    """

    # Available models with their specifications
    MODELS = {
        'gpt-4o': ModelInfo(
            id='gpt-4o',
            name='GPT-4o',
            max_tokens=128000,
            supports_vision=True,
            supports_functions=True,
            description='Most capable model, multimodal'
        ),
        'gpt-4o-mini': ModelInfo(
            id='gpt-4o-mini',
            name='GPT-4o Mini',
            max_tokens=128000,
            supports_vision=True,
            supports_functions=True,
            description='Smaller, faster GPT-4o variant'
        ),
        'gpt-4-turbo': ModelInfo(
            id='gpt-4-turbo',
            name='GPT-4 Turbo',
            max_tokens=128000,
            supports_vision=True,
            supports_functions=True,
            description='Fast GPT-4 with vision capabilities'
        ),
        'gpt-4': ModelInfo(
            id='gpt-4',
            name='GPT-4',
            max_tokens=8192,
            supports_vision=False,
            supports_functions=True,
            description='Original GPT-4 model'
        ),
        'gpt-3.5-turbo': ModelInfo(
            id='gpt-3.5-turbo',
            name='GPT-3.5 Turbo',
            max_tokens=16385,
            supports_vision=False,
            supports_functions=True,
            description='Fast and cost-effective'
        ),
    }

    DEFAULT_MODEL = 'gpt-4o-mini'

    def __init__(self, api_key: str, base_url: Optional[str] = None, **kwargs):
        """Initialize OpenAI provider.

        Args:
            api_key: OpenAI API key (starts with 'sk-')
            base_url: Optional custom base URL (for Azure or proxies)
            **kwargs: Additional configuration
        """
        super().__init__(api_key, base_url, **kwargs)
        self._client: Optional[OpenAI] = None

    def _initialize_client(self) -> None:
        """Initialize the OpenAI client."""
        try:
            client_kwargs = {'api_key': self.api_key}
            if self.base_url:
                client_kwargs['base_url'] = self.base_url

            self._client = OpenAI(**client_kwargs)
            logger.debug("OpenAI client initialized successfully")
        except Exception as e:
            logger.error(f"Failed to initialize OpenAI client: {e}")
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
        """Generate completion using OpenAI API.

        Args:
            messages: List of conversation messages
            model: Model identifier (defaults to gpt-4o-mini)
            max_tokens: Maximum tokens in response
            temperature: Sampling temperature (0.0 - 2.0)
            stop: Optional stop sequences
            **kwargs: Additional OpenAI-specific parameters

        Returns:
            LLMResponse with generated content
        """
        self._ensure_client()

        model = model or self.default_model

        try:
            api_messages = self._convert_messages(messages)

            response = self._client.chat.completions.create(
                model=model,
                messages=api_messages,
                max_tokens=max_tokens,
                temperature=temperature,
                stop=stop,
                **kwargs
            )

            choice = response.choices[0]
            content = choice.message.content or ''

            usage = {}
            if response.usage:
                usage = {
                    'prompt_tokens': response.usage.prompt_tokens,
                    'completion_tokens': response.usage.completion_tokens,
                    'total_tokens': response.usage.total_tokens
                }

            return LLMResponse(
                content=content,
                model=response.model,
                provider=self.provider_type.value,
                usage=usage,
                finish_reason=choice.finish_reason,
                raw_response={'id': response.id}
            )

        except Exception as e:
            return self._handle_error(e, "completion failed")

    def get_available_models(self) -> List[ModelInfo]:
        """Return list of available OpenAI models."""
        return list(self.MODELS.values())

    def test_connection(self) -> Dict[str, Any]:
        """Test OpenAI API connection.

        Returns:
            Dictionary with connection status
        """
        try:
            self._ensure_client()

            # Make a minimal API call to verify credentials
            response = self._client.chat.completions.create(
                model='gpt-3.5-turbo',
                messages=[{'role': 'user', 'content': 'Hi'}],
                max_tokens=5
            )

            return {
                'success': True,
                'message': 'OpenAI connection successful',
                'details': {
                    'model_used': response.model,
                    'response_id': response.id
                }
            }

        except Exception as e:
            logger.error(f"OpenAI connection test failed: {e}")
            return {
                'success': False,
                'message': 'OpenAI connection failed',
                'error': str(e)
            }

    @property
    def provider_type(self) -> ProviderType:
        """Return OpenAI provider type."""
        return ProviderType.OPENAI

    @property
    def default_model(self) -> str:
        """Return default OpenAI model."""
        return self.config.get('model', self.DEFAULT_MODEL)

    def validate_api_key(self) -> bool:
        """Validate OpenAI API key format."""
        if not self.api_key:
            return False
        # OpenAI keys start with 'sk-' and are typically 51+ characters
        return self.api_key.startswith('sk-') and len(self.api_key) > 40
