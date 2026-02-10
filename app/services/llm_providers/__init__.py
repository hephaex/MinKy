"""LLM Provider registry and factory.

This module provides a unified interface for managing multiple LLM providers.
Providers are registered using the @register_provider decorator and can be
instantiated via the get_provider factory function.

Example:
    from app.services.llm_providers import get_provider, list_providers

    # Get a provider instance
    provider = get_provider('openai', api_key='sk-...')
    response = provider.complete([LLMMessage(role='user', content='Hello')])

    # List available providers
    providers = list_providers()
"""
from typing import Dict, Type, Optional, List
import logging

from .base_provider import (
    BaseLLMProvider,
    LLMMessage,
    LLMResponse,
    ModelInfo,
    ProviderType
)

logger = logging.getLogger(__name__)

# Provider registry: maps provider name to provider class
_provider_registry: Dict[str, Type[BaseLLMProvider]] = {}


def register_provider(name: str):
    """Decorator to register a provider class.

    Args:
        name: Unique identifier for the provider (e.g., 'openai', 'anthropic')

    Example:
        @register_provider('openai')
        class OpenAIProvider(BaseLLMProvider):
            ...
    """
    def decorator(cls: Type[BaseLLMProvider]):
        if name in _provider_registry:
            logger.warning(f"Provider '{name}' is being re-registered")
        _provider_registry[name] = cls
        logger.debug(f"Registered LLM provider: {name}")
        return cls
    return decorator


def get_provider(
    name: str,
    api_key: str,
    base_url: Optional[str] = None,
    **kwargs
) -> Optional[BaseLLMProvider]:
    """Factory function to create a provider instance.

    Args:
        name: Provider identifier (e.g., 'openai', 'anthropic')
        api_key: API key for authentication
        base_url: Optional custom base URL
        **kwargs: Additional provider-specific configuration

    Returns:
        Provider instance or None if provider not found
    """
    provider_cls = _provider_registry.get(name)
    if provider_cls is None:
        logger.error(f"Provider '{name}' not found. Available: {list(_provider_registry.keys())}")
        return None

    try:
        return provider_cls(api_key=api_key, base_url=base_url, **kwargs)
    except Exception as e:
        logger.error(f"Failed to instantiate provider '{name}': {e}")
        return None


def list_providers() -> List[str]:
    """Return list of registered provider names."""
    return list(_provider_registry.keys())


def get_provider_class(name: str) -> Optional[Type[BaseLLMProvider]]:
    """Get the provider class without instantiating.

    Args:
        name: Provider identifier

    Returns:
        Provider class or None if not found
    """
    return _provider_registry.get(name)


def is_provider_registered(name: str) -> bool:
    """Check if a provider is registered."""
    return name in _provider_registry


# Import providers to trigger registration
# These imports must be after the registry is defined
try:
    from .openai_provider import OpenAIProvider
except ImportError as e:
    logger.warning(f"OpenAI provider not available: {e}")

try:
    from .anthropic_provider import AnthropicProvider
except ImportError as e:
    logger.debug(f"Anthropic provider not available: {e}")

try:
    from .gemini_provider import GeminiProvider
except ImportError as e:
    logger.debug(f"Gemini provider not available: {e}")

try:
    from .local_provider import LocalProvider
except ImportError as e:
    logger.debug(f"Local provider not available: {e}")


__all__ = [
    'BaseLLMProvider',
    'LLMMessage',
    'LLMResponse',
    'ModelInfo',
    'ProviderType',
    'register_provider',
    'get_provider',
    'list_providers',
    'get_provider_class',
    'is_provider_registered',
]
