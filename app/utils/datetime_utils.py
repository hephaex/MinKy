"""Datetime utility functions for consistent UTC handling."""
from datetime import datetime, timezone


def utc_now() -> datetime:
    """Return current UTC datetime with timezone info.

    This is the single source of truth for UTC datetime creation.
    Use this instead of datetime.utcnow() which is deprecated in Python 3.12+.

    Returns:
        datetime: Current UTC datetime with timezone awareness
    """
    return datetime.now(timezone.utc)
