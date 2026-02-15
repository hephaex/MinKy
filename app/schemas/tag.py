"""
Tag-related Pydantic schemas for input validation.
"""

from pydantic import BaseModel, Field, field_validator
from typing import Optional
import re


class TagCreate(BaseModel):
    """Schema for creating a new tag."""

    name: str = Field(
        ...,
        min_length=1,
        max_length=100,
        description="Tag name"
    )
    color: Optional[str] = Field(
        default=None,
        max_length=7,
        description="Tag color in hex format (e.g., #FF5733)"
    )
    description: Optional[str] = Field(
        default=None,
        max_length=500,
        description="Tag description"
    )

    @field_validator('name')
    @classmethod
    def name_not_empty(cls, v: str) -> str:
        """Ensure tag name is not empty or whitespace only."""
        stripped = v.strip()
        if not stripped:
            raise ValueError('Tag name cannot be empty or whitespace only')
        return stripped

    @field_validator('color')
    @classmethod
    def color_valid_hex(cls, v: Optional[str]) -> Optional[str]:
        """Ensure color is a valid hex color code."""
        if v is None:
            return v
        v = v.strip()
        if not v:
            return None
        if not re.match(r'^#[0-9A-Fa-f]{6}$', v):
            raise ValueError('Color must be a valid hex color code (e.g., #FF5733)')
        return v.upper()

    model_config = {
        "extra": "forbid",  # SECURITY: Reject unknown fields
        "json_schema_extra": {
            "examples": [
                {
                    "name": "python",
                    "color": "#3776AB",
                    "description": "Python programming language"
                }
            ]
        }
    }


class TagUpdate(BaseModel):
    """Schema for updating an existing tag."""

    name: Optional[str] = Field(
        default=None,
        min_length=1,
        max_length=100,
        description="Tag name"
    )
    color: Optional[str] = Field(
        default=None,
        max_length=7,
        description="Tag color in hex format"
    )
    description: Optional[str] = Field(
        default=None,
        max_length=500,
        description="Tag description"
    )

    @field_validator('name')
    @classmethod
    def name_not_empty(cls, v: Optional[str]) -> Optional[str]:
        """Ensure tag name is not empty or whitespace only if provided."""
        if v is None:
            return v
        stripped = v.strip()
        if not stripped:
            raise ValueError('Tag name cannot be empty or whitespace only')
        return stripped

    @field_validator('color')
    @classmethod
    def color_valid_hex(cls, v: Optional[str]) -> Optional[str]:
        """Ensure color is a valid hex color code if provided."""
        if v is None:
            return v
        v = v.strip()
        if not v:
            return None
        if not re.match(r'^#[0-9A-Fa-f]{6}$', v):
            raise ValueError('Color must be a valid hex color code (e.g., #FF5733)')
        return v.upper()


class TagMerge(BaseModel):
    """Schema for merging tags."""

    source_tag_id: int = Field(..., description="ID of the tag to merge from")
    target_tag_id: int = Field(..., description="ID of the tag to merge into")

    @field_validator('target_tag_id')
    @classmethod
    def tags_different(cls, v: int, info) -> int:
        """Ensure source and target tags are different."""
        if 'source_tag_id' in info.data and v == info.data['source_tag_id']:
            raise ValueError('Source and target tags must be different')
        return v
