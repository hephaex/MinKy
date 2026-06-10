"""Tests for reasoning-model output stripping."""
from app.utils.llm_reasoning import strip_reasoning, is_reasoning_model


def test_removes_closed_think_block():
    assert strip_reasoning("<think>weighing options</think>#AI #ML") == "#AI #ML"


def test_removes_multiline_think_block():
    text = "<think>\nline1\nline2\n</think>\n#AI #Data"
    assert strip_reasoning(text) == "#AI #Data"


def test_drops_unclosed_think_tail():
    # Token budget exhausted mid-thought: no answer, just an open <think>.
    assert strip_reasoning("answer<think>still thinking and truncated") == "answer"


def test_plain_output_unchanged():
    assert strip_reasoning("#AI #MachineLearning #Python") == "#AI #MachineLearning #Python"


def test_removes_english_preamble_before_json():
    text = "Here's a thinking process about tags. [\"AI\", \"ML\"]"
    assert strip_reasoning(text) == '["AI", "ML"]'


def test_empty_input():
    assert strip_reasoning("") == ""
    assert strip_reasoning(None) is None


def test_is_reasoning_model():
    assert is_reasoning_model("solar-open2-260528") is True
    assert is_reasoning_model("gpt-3.5-turbo") is False
    assert is_reasoning_model("solar-pro2") is False
