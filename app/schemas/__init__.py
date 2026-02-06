"""
Pydantic schemas for API input validation.
"""

from app.schemas.document import (
    DocumentCreate,
    DocumentUpdate,
    DocumentSearch,
)
from app.schemas.auth import (
    RegisterRequest,
    LoginRequest,
    PasswordChange,
)
from app.schemas.tag import (
    TagCreate,
    TagUpdate,
)
from app.schemas.category import (
    CategoryCreate,
    CategoryUpdate,
    CategoryMove,
)

__all__ = [
    # Document
    'DocumentCreate',
    'DocumentUpdate',
    'DocumentSearch',
    # Auth
    'RegisterRequest',
    'LoginRequest',
    'PasswordChange',
    # Tag
    'TagCreate',
    'TagUpdate',
    # Category
    'CategoryCreate',
    'CategoryUpdate',
    'CategoryMove',
]
