"""
Category-related Pydantic schemas for input validation.
"""

from pydantic import BaseModel, Field, field_validator
from typing import Optional
import re


class CategoryCreate(BaseModel):
    """Schema for creating a new category."""

    name: str = Field(
        ...,
        min_length=1,
        max_length=100,
        description="Category name"
    )
    description: Optional[str] = Field(
        default=None,
        max_length=500,
        description="Category description"
    )
    parent_id: Optional[int] = Field(
        default=None,
        description="Parent category ID for nesting"
    )
    color: str = Field(
        default="#007bff",
        max_length=7,
        description="Category color in hex format"
    )
    sort_order: int = Field(
        default=0,
        ge=0,
        description="Sort order within parent"
    )

    @field_validator('name')
    @classmethod
    def name_not_empty(cls, v: str) -> str:
        """Ensure category name is not empty or whitespace only."""
        stripped = v.strip()
        if not stripped:
            raise ValueError('Category name cannot be empty or whitespace only')
        return stripped

    @field_validator('color')
    @classmethod
    def color_valid_hex(cls, v: str) -> str:
        """Ensure color is a valid hex color code."""
        v = v.strip()
        if not re.match(r'^#[0-9A-Fa-f]{6}$', v):
            raise ValueError('Color must be a valid hex color code (e.g., #007bff)')
        return v.upper()

    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "name": "Technology",
                    "description": "Technology-related documents",
                    "color": "#007bff",
                    "parent_id": None
                }
            ]
        }
    }


class CategoryUpdate(BaseModel):
    """Schema for updating an existing category."""

    name: Optional[str] = Field(
        default=None,
        min_length=1,
        max_length=100,
        description="Category name"
    )
    description: Optional[str] = Field(
        default=None,
        max_length=500,
        description="Category description"
    )
    parent_id: Optional[int] = Field(
        default=None,
        description="Parent category ID"
    )
    color: Optional[str] = Field(
        default=None,
        max_length=7,
        description="Category color in hex format"
    )
    sort_order: Optional[int] = Field(
        default=None,
        ge=0,
        description="Sort order within parent"
    )
    is_active: Optional[bool] = Field(
        default=None,
        description="Whether category is active"
    )

    @field_validator('name')
    @classmethod
    def name_not_empty(cls, v: Optional[str]) -> Optional[str]:
        """Ensure category name is not empty or whitespace only if provided."""
        if v is None:
            return v
        stripped = v.strip()
        if not stripped:
            raise ValueError('Category name cannot be empty or whitespace only')
        return stripped

    @field_validator('color')
    @classmethod
    def color_valid_hex(cls, v: Optional[str]) -> Optional[str]:
        """Ensure color is a valid hex color code if provided."""
        if v is None:
            return v
        v = v.strip()
        if not re.match(r'^#[0-9A-Fa-f]{6}$', v):
            raise ValueError('Color must be a valid hex color code (e.g., #007bff)')
        return v.upper()


class CategoryMove(BaseModel):
    """Schema for moving a category to a new parent."""

    parent_id: Optional[int] = Field(
        default=None,
        description="New parent category ID (null for root level)"
    )
