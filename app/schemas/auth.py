"""
Authentication-related Pydantic schemas for input validation.
"""

from pydantic import BaseModel, Field, field_validator, EmailStr
import re


class RegisterRequest(BaseModel):
    """Schema for user registration."""

    username: str = Field(
        ...,
        min_length=3,
        max_length=50,
        description="Username (alphanumeric and underscores only)"
    )
    email: EmailStr = Field(..., description="Valid email address")
    password: str = Field(
        ...,
        min_length=8,
        max_length=128,
        description="Password (minimum 8 characters)"
    )

    @field_validator('username')
    @classmethod
    def username_valid_characters(cls, v: str) -> str:
        """Ensure username contains only valid characters."""
        if not re.match(r'^[a-zA-Z0-9_]+$', v):
            raise ValueError('Username can only contain letters, numbers, and underscores')
        return v.lower()

    @field_validator('password')
    @classmethod
    def password_strength(cls, v: str) -> str:
        """Ensure password meets strength requirements."""
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        if not re.search(r'[a-zA-Z]', v):
            raise ValueError('Password must contain at least one letter')
        if not re.search(r'\d', v):
            raise ValueError('Password must contain at least one number')
        return v

    model_config = {
        "json_schema_extra": {
            "examples": [
                {
                    "username": "john_doe",
                    "email": "john@example.com",
                    "password": "securepass123"
                }
            ]
        }
    }


class LoginRequest(BaseModel):
    """Schema for user login."""

    username: str = Field(..., min_length=1, description="Username or email")
    password: str = Field(..., min_length=1, description="Password")

    @field_validator('username')
    @classmethod
    def username_not_empty(cls, v: str) -> str:
        """Ensure username is not empty."""
        stripped = v.strip()
        if not stripped:
            raise ValueError('Username cannot be empty')
        return stripped.lower()


class PasswordChange(BaseModel):
    """Schema for changing password."""

    current_password: str = Field(..., min_length=1, description="Current password")
    new_password: str = Field(
        ...,
        min_length=8,
        max_length=128,
        description="New password (minimum 8 characters)"
    )

    @field_validator('new_password')
    @classmethod
    def new_password_strength(cls, v: str) -> str:
        """Ensure new password meets strength requirements."""
        if len(v) < 8:
            raise ValueError('New password must be at least 8 characters long')
        if not re.search(r'[a-zA-Z]', v):
            raise ValueError('New password must contain at least one letter')
        if not re.search(r'\d', v):
            raise ValueError('New password must contain at least one number')
        return v
