"""DeepSeek LLM Provider implementation.

DeepSeek is served through an OpenAI-compatible API (the Lablup gateway by
default), so this provider reuses the OpenAI client with a custom base URL.
The endpoint can be overridden with ``DEEPSEEK_BASE_URL``.
"""
from typing import Any, Dict, List, Optional
import logging
import os

from . import register_provider
from .base_provider import ModelInfo, ProviderType
from .openai_provider import OpenAIProvider

logger = logging.getLogger(__name__)

DEEPSEEK_DEFAULT_BASE_URL = 'https://api.console.lablup.ai/v1'
DEEPSEEK_DEFAULT_MODEL = 'deepseek-ai/DeepSeek-V4.1-Flash'


def deepseek_base_url() -> str:
    """Resolve the DeepSeek endpoint (``DEEPSEEK_BASE_URL`` or the Lablup gateway)."""
    return os.getenv('DEEPSEEK_BASE_URL') or DEEPSEEK_DEFAULT_BASE_URL


@register_provider('deepseek')
class DeepSeekProvider(OpenAIProvider):
    """DeepSeek provider (OpenAI-compatible)."""

    MODELS = {
        DEEPSEEK_DEFAULT_MODEL: ModelInfo(
            id=DEEPSEEK_DEFAULT_MODEL,
            name='DeepSeek V4.1 Flash',
            max_tokens=384000,
            supports_vision=False,
            supports_functions=False,
            description='DeepSeek V4.1 Flash via the OpenAI-compatible gateway'
        ),
    }

    DEFAULT_MODEL = DEEPSEEK_DEFAULT_MODEL

    def __init__(self, api_key: str, base_url: Optional[str] = None, **kwargs):
        """Initialize the DeepSeek provider, defaulting to the configured endpoint."""
        super().__init__(api_key, base_url or deepseek_base_url(), **kwargs)

    def get_available_models(self) -> List[ModelInfo]:
        """Return list of available DeepSeek models."""
        return list(self.MODELS.values())

    def test_connection(self) -> Dict[str, Any]:
        """Test the DeepSeek API connection with a minimal call."""
        try:
            self._ensure_client()
            response = self._client.chat.completions.create(
                model=self.default_model,
                messages=[{'role': 'user', 'content': 'Hi'}],
                max_tokens=16
            )
            return {
                'success': True,
                'message': 'DeepSeek connection successful',
                'details': {'model_used': response.model}
            }
        except Exception as e:
            logger.error(f"DeepSeek connection test failed: {e}")
            return {
                'success': False,
                'message': 'DeepSeek connection failed',
                'error': str(e)
            }

    @property
    def provider_type(self) -> ProviderType:
        """Return DeepSeek provider type."""
        return ProviderType.DEEPSEEK

    def validate_api_key(self) -> bool:
        """Validate DeepSeek API key (gateway keys are not 'sk-' prefixed)."""
        return bool(self.api_key) and len(self.api_key) > 10
