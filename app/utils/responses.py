"""
Response utility functions for Minky API
Provides standardized response building patterns to reduce code duplication.
"""

from flask import jsonify, url_for, abort
from app import db


def build_pagination_response(query, page, per_page, serializer_func,
                               items_key='items', wrap_pagination=False,
                               endpoint=None, extra_fields=None, **endpoint_kwargs):
    """
    Build a standardized pagination response from a SQLAlchemy query.

    Args:
        query: SQLAlchemy query object
        page: Current page number (1-indexed)
        per_page: Number of items per page
        serializer_func: Function to serialize each item (item -> dict)
        items_key: Key name for the items list (default 'items', can be 'documents', 'tags', etc.)
        wrap_pagination: If True, wrap pagination metadata in 'pagination' object (for backward compat)
        endpoint: Optional Flask endpoint name for URL generation
        extra_fields: Optional dict of additional fields to include in response
        endpoint_kwargs: Additional kwargs for url_for()

    Returns:
        dict: Standardized pagination response

    Example:
        # Simple usage
        return build_pagination_response(
            query, page=1, per_page=20,
            serializer_func=lambda d: d.to_dict()
        )

        # With custom item key and wrapped pagination (backward compatible)
        return build_pagination_response(
            query, page=1, per_page=20,
            serializer_func=lambda d: d.to_dict(),
            items_key='documents',
            wrap_pagination=True,
            extra_fields={'search_query': search}
        )
    """
    # Execute pagination
    paginated = query.paginate(page=page, per_page=per_page, error_out=False)

    # Serialize items
    items = [serializer_func(item) for item in paginated.items]

    # Build pagination metadata
    pagination_data = {
        'page': page,
        'per_page': per_page,
        'total': paginated.total,
        'pages': paginated.pages,
        'has_next': paginated.has_next,
        'has_prev': paginated.has_prev
    }

    # Add navigation URLs if endpoint provided
    if endpoint:
        if paginated.has_next:
            pagination_data['next_url'] = url_for(endpoint, page=page + 1, per_page=per_page, **endpoint_kwargs)
        if paginated.has_prev:
            pagination_data['prev_url'] = url_for(endpoint, page=page - 1, per_page=per_page, **endpoint_kwargs)

    # Build response based on structure preference
    if wrap_pagination:
        response = {
            items_key: items,
            'pagination': pagination_data
        }
    else:
        response = {items_key: items, **pagination_data}

    # Add extra fields if provided
    if extra_fields:
        response.update(extra_fields)

    return response


def paginate_query(query, page, per_page, serializer_func, items_key='items', extra_fields=None):
    """
    Simplified pagination helper that returns a Flask response.

    Args:
        query: SQLAlchemy query object
        page: Current page number
        per_page: Items per page
        serializer_func: Function to serialize each item
        items_key: Key name for items list
        extra_fields: Additional fields to include

    Returns:
        Flask JSON response
    """
    response = build_pagination_response(
        query, page, per_page, serializer_func,
        items_key=items_key, wrap_pagination=True,
        extra_fields=extra_fields
    )
    return jsonify(response)


def success_response(data=None, message=None, status_code=200):
    """
    Build a standardized success response.

    Args:
        data: Response data (dict or list)
        message: Optional success message
        status_code: HTTP status code (default 200)

    Returns:
        tuple: (response_dict, status_code)
    """
    response = {'success': True}
    if data is not None:
        response['data'] = data
    if message:
        response['message'] = message
    return jsonify(response), status_code


def error_response(error, status_code=400, details=None):
    """
    Build a standardized error response.

    Args:
        error: Error message
        status_code: HTTP status code (default 400)
        details: Optional additional details

    Returns:
        tuple: (response_dict, status_code)
    """
    response = {
        'success': False,
        'error': error
    }
    if details:
        response['details'] = details
    return jsonify(response), status_code


def list_response(items, total=None, serializer_func=None):
    """
    Build a standardized list response.

    Args:
        items: List of items
        total: Optional total count (defaults to len(items))
        serializer_func: Optional function to serialize each item

    Returns:
        dict: Response with items and count
    """
    if serializer_func:
        items = [serializer_func(item) for item in items]

    return {
        'success': True,
        'items': items,
        'total': total if total is not None else len(items)
    }


def get_or_404(model, id, description=None):
    """
    Get a model instance by ID or abort with 404.

    This is a drop-in replacement for Flask-SQLAlchemy's Query.get_or_404()
    that uses the modern db.session.get() method to avoid deprecation warnings.

    Args:
        model: SQLAlchemy model class
        id: Primary key value
        description: Optional description for the 404 error

    Returns:
        Model instance

    Raises:
        404: If the instance is not found

    Example:
        from app.utils.responses import get_or_404
        document = get_or_404(Document, document_id)
    """
    instance = db.session.get(model, id)
    if instance is None:
        abort(404, description=description or f'{model.__name__} not found')
    return instance
