"""Helpers for handling reasoning-model output.

Reasoning models (e.g. Upstage ``solar-open2``) emit their chain-of-thought
inside ``<think>...</think>`` blocks, sometimes followed by an English
preamble, before the actual answer. ``strip_reasoning`` removes that so the
downstream parser sees only the final answer. It is a no-op on plain output
from non-reasoning models.
"""
import re

# Models that wrap their answer in reasoning output and need stripping.
REASONING_MODELS = frozenset({'solar-open2-260528'})

_THINK_BLOCK = re.compile(r"<think>.*?</think>", re.DOTALL)
_PREAMBLE = re.compile(
    r"(?:Here'?s?\s+(?:a\s+)?thinking\s+process|Let me (?:think|analyze))"
    r".*?(?=\[|\{\")",
    re.DOTALL | re.IGNORECASE,
)


def is_reasoning_model(model: str) -> bool:
    """Return True when the model wraps answers in reasoning output."""
    return model in REASONING_MODELS


def strip_reasoning(text: str) -> str:
    """Remove a reasoning model's ``<think>`` output and any thinking preamble.

    Safe on plain output: text without reasoning markers is returned unchanged
    (apart from surrounding whitespace).
    """
    if not text:
        return text

    # 1) Remove fully-closed <think>...</think> blocks.
    text = _THINK_BLOCK.sub("", text).strip()

    # 2) Drop an unclosed <think> tail (token budget exhausted mid-thought).
    if "<think>" in text:
        text = text[: text.index("<think>")].strip()

    # 3) Remove an English thinking preamble that precedes a JSON payload.
    match = _PREAMBLE.search(text)
    if match:
        text = text[match.end():].strip()

    return text
