"""Upstage Solar LLM Provider implementation.

Upstage exposes an OpenAI-compatible API, so this provider reuses the OpenAI
client with the Upstage base URL. The default model ``solar-open2-260528`` is a
reasoning model, so completions are post-processed to strip the ``<think>``
output and leave only the final answer.
"""
from typing import List, Dict, Any, Optional
import logging

from . import register_provider
from .base_provider import LLMMessage, LLMResponse, ModelInfo, ProviderType
from .openai_provider import OpenAIProvider
from app.utils.llm_reasoning import strip_reasoning

logger = logging.getLogger(__name__)


@register_provider('upstage')
@register_provider('solar')
class SolarProvider(OpenAIProvider):
    """Upstage Solar provider (OpenAI-compatible, reasoning-aware)."""

    UPSTAGE_BASE_URL = 'https://api.upstage.ai/v1'

    MODELS = {
        'solar-open2-260528': ModelInfo(
            id='solar-open2-260528',
            name='Solar Open2',
            max_tokens=32768,
            supports_vision=False,
            supports_functions=False,
            description='Upstage reasoning model (emits <think> output)'
        ),
        'solar-pro2': ModelInfo(
            id='solar-pro2',
            name='Solar Pro2',
            max_tokens=32768,
            supports_vision=False,
            supports_functions=False,
            description='Upstage general-purpose model'
        ),
    }

    DEFAULT_MODEL = 'solar-open2-260528'

    def __init__(self, api_key: str, base_url: Optional[str] = None, **kwargs):
        """Initialize the Solar provider, defaulting to the Upstage endpoint."""
        super().__init__(api_key, base_url or self.UPSTAGE_BASE_URL, **kwargs)

    def complete(
        self,
        messages: List[LLMMessage],
        model: Optional[str] = None,
        max_tokens: int = 1000,
        temperature: float = 0.7,
        stop: Optional[List[str]] = None,
        **kwargs
    ) -> LLMResponse:
        """Complete via Upstage and strip reasoning output from the answer."""
        response = super().complete(
            messages,
            model=model,
            max_tokens=max_tokens,
            temperature=temperature,
            stop=stop,
            **kwargs
        )
        response.content = strip_reasoning(response.content)
        return response

    def get_available_models(self) -> List[ModelInfo]:
        """Return list of available Upstage Solar models."""
        return list(self.MODELS.values())

    def test_connection(self) -> Dict[str, Any]:
        """Test the Upstage API connection with a minimal call."""
        try:
            self._ensure_client()
            response = self._client.chat.completions.create(
                model=self.default_model,
                messages=[{'role': 'user', 'content': 'Hi'}],
                max_tokens=16
            )
            return {
                'success': True,
                'message': 'Upstage Solar connection successful',
                'details': {'model_used': response.model}
            }
        except Exception as e:
            logger.error(f"Upstage Solar connection test failed: {e}")
            return {
                'success': False,
                'message': 'Upstage Solar connection failed',
                'error': str(e)
            }

    @property
    def provider_type(self) -> ProviderType:
        """Return Solar provider type."""
        return ProviderType.SOLAR

    def validate_api_key(self) -> bool:
        """Validate Upstage API key (Upstage keys are not 'sk-' prefixed)."""
        return bool(self.api_key) and len(self.api_key) > 10
