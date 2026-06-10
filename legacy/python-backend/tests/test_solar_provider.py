"""Tests for the Upstage Solar LLM provider."""
from app.services.llm_providers import get_provider, is_provider_registered
from app.services.llm_providers.base_provider import LLMMessage, ProviderType


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
        self.model = 'solar-open2-260528'
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


def test_solar_and_upstage_registered():
    assert is_provider_registered('solar')
    assert is_provider_registered('upstage')


def test_solar_defaults_to_upstage_endpoint_and_model():
    provider = get_provider('solar', api_key='up-test-key-1234567890')
    assert provider is not None
    assert provider.base_url == 'https://api.upstage.ai/v1'
    assert provider.default_model == 'solar-open2-260528'
    assert provider.provider_type == ProviderType.SOLAR


def test_solar_strips_reasoning_from_completion():
    provider = get_provider('solar', api_key='up-test-key-1234567890')
    provider._client = _FakeClient("<think>let me pick tags</think>#AI #ML")

    resp = provider.complete([LLMMessage(role='user', content='hi')], max_tokens=2048)

    assert resp.content == '#AI #ML'
    assert resp.provider == 'solar'


def test_solar_passes_model_to_api():
    provider = get_provider('solar', api_key='up-test-key-1234567890')
    fake = _FakeClient('#AI')
    provider._client = fake

    provider.complete(
        [LLMMessage(role='user', content='hi')],
        model='solar-open2-260528',
        max_tokens=2048,
    )

    assert fake.chat.completions.last_kwargs['model'] == 'solar-open2-260528'


def test_solar_validates_non_sk_key():
    provider = get_provider('solar', api_key='up-abcdefghijklmnop')
    assert provider.validate_api_key() is True
    provider_short = get_provider('solar', api_key='short')
    assert provider_short.validate_api_key() is False
