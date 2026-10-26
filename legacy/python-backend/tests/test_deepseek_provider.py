"""Tests for the DeepSeek LLM provider (OpenAI-compatible, Lablup gateway)."""
from app.services.llm_providers import get_provider, is_provider_registered
from app.services.llm_providers.base_provider import LLMMessage, ProviderType

MODEL = 'deepseek-ai/DeepSeek-V4.1-Flash'


class _Usage:
    prompt_tokens = 10
    completion_tokens = 5
    total_tokens = 15


class _Msg:
    def __init__(self, content):
        self.content = content


class _Choice:
    def __init__(self, content):
        self.message = _Msg(content)
        self.finish_reason = 'stop'


class _Resp:
    def __init__(self, content):
        self.choices = [_Choice(content)]
        self.model = MODEL
        self.usage = _Usage()
        self.id = 'resp_1'


class _Completions:
    def __init__(self, content):
        self._content = content
        self.last_kwargs = None

    def create(self, **kwargs):
        self.last_kwargs = kwargs
        return _Resp(self._content)


class _FakeClient:
    def __init__(self, content):
        self.chat = type('Chat', (), {'completions': _Completions(content)})()


def test_deepseek_registered_and_anthropic_removed():
    assert is_provider_registered('deepseek')
    assert not is_provider_registered('anthropic')


def test_deepseek_defaults(monkeypatch):
    monkeypatch.delenv('DEEPSEEK_BASE_URL', raising=False)
    provider = get_provider('deepseek', api_key='ds-test-key-1234567890')
    assert provider.base_url == 'https://api.console.lablup.ai/v1'
    assert provider.default_model == MODEL
    assert provider.provider_type == ProviderType.DEEPSEEK


def test_deepseek_base_url_env_override(monkeypatch):
    monkeypatch.setenv('DEEPSEEK_BASE_URL', 'https://example.test/v1')
    provider = get_provider('deepseek', api_key='ds-test-key-1234567890')
    assert provider.base_url == 'https://example.test/v1'


def test_deepseek_completion_passes_model_and_returns_content():
    provider = get_provider('deepseek', api_key='ds-test-key-1234567890')
    fake = _FakeClient('#AI #ML')
    provider._client = fake

    resp = provider.complete([LLMMessage(role='user', content='hi')], max_tokens=256)

    assert resp.content == '#AI #ML'
    assert resp.provider == 'deepseek'
    assert fake.chat.completions.last_kwargs['model'] == MODEL


def test_deepseek_validates_non_sk_key():
    provider = get_provider('deepseek', api_key='ds-abcdefghijklmnop')
    assert provider.validate_api_key() is True
    assert get_provider('deepseek', api_key='short').validate_api_key() is False
