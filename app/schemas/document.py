"""
Document-related Pydantic schemas for input validation.
"""

from pydantic import BaseModel, Field, field_validator
from typing import Optional


class DocumentCreate(BaseModel):
    """Schema for creating a new document."""

    title: str = Field(..., min_length=1, max_length=500, description="Document title")
    markdown_content: str = Field(default="", description="Markdown content")
    author: Optional[str] = Field(default=None, max_length=200, description="Author name")
    is_public: bool = Field(default=True, description="Whether document is publicly visible")
    tags: list[str] = Field(default_factory=list, description="List of tag names")
    category_id: Optional[int] = Field(default=None, description="Category ID")

    @field_validator('title')
    @classmethod
    def title_not_empty(cls, v: str) -> str:
        """Ensure title is not empty or whitespace only."""
        stripped = v.strip()
        if not stripped:
            raise ValueError('Title cannot be empty or whitespace only')
        return stripped

    @field_validator('tags')
    @classmethod
    def tags_not_empty_strings(cls, v: list[str]) -> list[str]:
        """Ensure tags are not empty strings."""
        result = []
        for tag in v:
            stripped = tag.strip()
            if stripped:
                result.append(stripped)
        return result

    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "title": "My Document",
                    "markdown_content": "# Hello World\n\nThis is content.",
                    "author": "John Doe",
                    "is_public": True,
                    "tags": ["python", "tutorial"]
                }
            ]
        }
    }


class DocumentUpdate(BaseModel):
    """Schema for updating an existing document."""

    title: Optional[str] = Field(default=None, min_length=1, max_length=500)
    markdown_content: Optional[str] = Field(default=None)
    author: Optional[str] = Field(default=None, max_length=200)
    is_public: Optional[bool] = Field(default=None)
    tags: Optional[list[str]] = Field(default=None)
    category_id: Optional[int] = Field(default=None)

    @field_validator('title')
    @classmethod
    def title_not_empty(cls, v: Optional[str]) -> Optional[str]:
        """Ensure title is not empty or whitespace only if provided."""
        if v is None:
            return v
        stripped = v.strip()
        if not stripped:
            raise ValueError('Title cannot be empty or whitespace only')
        return stripped

    @field_validator('tags')
    @classmethod
    def tags_not_empty_strings(cls, v: Optional[list[str]]) -> Optional[list[str]]:
        """Ensure tags are not empty strings if provided."""
        if v is None:
            return v
        result = []
        for tag in v:
            stripped = tag.strip()
            if stripped:
                result.append(stripped)
        return result


class DocumentSearch(BaseModel):
    """Schema for document search parameters."""

    query: str = Field(default="", description="Search query string")
    tags: list[str] = Field(default_factory=list, description="Filter by tags")
    author: Optional[str] = Field(default=None, description="Filter by author")
    is_public: Optional[bool] = Field(default=None, description="Filter by visibility")
    category_id: Optional[int] = Field(default=None, description="Filter by category")
    page: int = Field(default=1, ge=1, description="Page number")
    per_page: int = Field(default=20, ge=1, le=100, description="Items per page")
    sort_by: str = Field(default="updated_at", description="Sort field")
    sort_order: str = Field(default="desc", pattern="^(asc|desc)$", description="Sort order")

    @field_validator('sort_by')
    @classmethod
    def validate_sort_by(cls, v: str) -> str:
        """Validate sort_by field."""
        allowed = {'created_at', 'updated_at', 'title', 'author'}
        if v not in allowed:
            raise ValueError(f'sort_by must be one of: {", ".join(allowed)}')
        return v
