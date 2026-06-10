"""AIService integration with the Upstage Solar reasoning model."""
from app.services.ai_service import AIService


class _Msg:
    def __init__(self, content):
        self.content = content


class _Choice:
    def __init__(self, content):
        self.message = _Msg(content)


class _Resp:
    def __init__(self, content):
        self.choices = [_Choice(content)]


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


def _solar_service(app, content):
    """Build an AIService wired to a fake Solar (reasoning) client."""
    svc = AIService()
    svc.enabled = True
    svc.is_reasoning_model = True
    svc.config['llmModel'] = 'solar-open2-260528'
    svc.openai_client = _FakeClient(content)
    return svc


def test_suggest_tags_strips_reasoning(app):
    svc = _solar_service(app, "<think>analyze the doc</think>#AI #MachineLearning #Python")

    tags = svc.suggest_tags(
        "A sufficiently long document about AI and machine learning systems.",
        "AI and Machine Learning",
    )

    assert 'AI' in tags
    assert 'MachineLearning' in tags
    assert 'Python' in tags
    assert all('think' not in t.lower() for t in tags)


def test_suggest_tags_uses_large_budget_for_reasoning(app):
    svc = _solar_service(app, "<think>x</think>#AI")

    svc.suggest_tags("Long enough content about artificial intelligence here.", "Title")

    # Reasoning models need a generous token budget, not the 50 used for plain LLMs.
    assert svc.openai_client.chat.completions.last_kwargs['max_tokens'] >= 2048


def test_suggest_title_strips_reasoning(app):
    svc = _solar_service(app, "<think>consider</think>Neural Networks Explained")

    title = svc.suggest_title("Document about neural networks and deep learning.")

    assert title == 'Neural Networks Explained'
