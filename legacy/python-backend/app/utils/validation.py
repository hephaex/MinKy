"""
Validation utilities for Pydantic schema integration with Flask routes.
"""

from functools import wraps
from typing import Type, TypeVar, Callable, Any
from flask import request
from pydantic import BaseModel, ValidationError
from app.utils.responses import error_response


def escape_like(value: str) -> str:
    """
    Escape special characters for SQL LIKE/ILIKE queries.

    Prevents SQL injection via pattern matching characters.

    Args:
        value: The user input string to escape

    Returns:
        Escaped string safe for use in LIKE patterns

    Example:
        search_escaped = escape_like(user_input)
        query.filter(Model.field.ilike(f'%{search_escaped}%'))
    """
    if not value:
        return value
    return (value
            .replace('\\', '\\\\')
            .replace('%', '\\%')
            .replace('_', '\\_'))


T = TypeVar('T', bound=BaseModel)


def validate_request(schema: Type[T]) -> Callable:
    """
    Decorator to validate request JSON body against a Pydantic schema.

    Usage:
        @app.route('/documents', methods=['POST'])
        @validate_request(DocumentCreate)
        def create_document(validated_data: DocumentCreate):
            # validated_data is already validated
            title = validated_data.title
            ...

    Args:
        schema: Pydantic model class to validate against

    Returns:
        Decorated function that receives validated data as first argument
    """
    def decorator(f: Callable) -> Callable:
        @wraps(f)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            data = request.get_json()

            if data is None:
                return error_response('Request body is required', 400)

            try:
                validated = schema.model_validate(data)
            except ValidationError as e:
                errors = format_validation_errors(e)
                return error_response(
                    'Validation failed',
                    400,
                    details={'validation_errors': errors}
                )

            return f(validated, *args, **kwargs)
        return wrapper
    return decorator


def validate_query_params(schema: Type[T]) -> Callable:
    """
    Decorator to validate query parameters against a Pydantic schema.

    Usage:
        @app.route('/documents', methods=['GET'])
        @validate_query_params(DocumentSearch)
        def search_documents(params: DocumentSearch):
            # params is already validated
            query = params.query
            ...

    Args:
        schema: Pydantic model class to validate against

    Returns:
        Decorated function that receives validated params as first argument
    """
    def decorator(f: Callable) -> Callable:
        @wraps(f)
        def wrapper(*args: Any, **kwargs: Any) -> Any:
            # Convert query args to dict, handling list values
            data = {}
            for key in request.args:
                values = request.args.getlist(key)
                data[key] = values if len(values) > 1 else values[0]

            try:
                validated = schema.model_validate(data)
            except ValidationError as e:
                errors = format_validation_errors(e)
                return error_response(
                    'Invalid query parameters',
                    400,
                    details={'validation_errors': errors}
                )

            return f(validated, *args, **kwargs)
        return wrapper
    return decorator


def format_validation_errors(error: ValidationError) -> list[dict]:
    """
    Format Pydantic validation errors into a user-friendly format.

    Args:
        error: Pydantic ValidationError

    Returns:
        List of error dictionaries with field, message, and type
    """
    errors = []
    for err in error.errors():
        field = '.'.join(str(loc) for loc in err['loc'])
        errors.append({
            'field': field,
            'message': err['msg'],
            'type': err['type']
        })
    return errors


def validate_data(schema: Type[T], data: dict) -> tuple[T | None, list[dict] | None]:
    """
    Validate data against a schema without using decorator.

    Usage:
        validated, errors = validate_data(DocumentCreate, request.get_json())
        if errors:
            return error_response('Validation failed', 400, details={'errors': errors})
        # use validated...

    Args:
        schema: Pydantic model class to validate against
        data: Dictionary to validate

    Returns:
        Tuple of (validated_model, None) on success or (None, errors) on failure
    """
    try:
        validated = schema.model_validate(data)
        return validated, None
    except ValidationError as e:
        errors = format_validation_errors(e)
        return None, errors
