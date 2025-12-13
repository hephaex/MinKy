"""
Response utility functions for Minky API
Provides standardized response building patterns to reduce code duplication.
"""

from flask import jsonify, url_for


def build_pagination_response(query, page, per_page, serializer_func, endpoint=None, **endpoint_kwargs):
    """
    Build a standardized pagination response from a SQLAlchemy query.

    Args:
        query: SQLAlchemy query object
        page: Current page number (1-indexed)
        per_page: Number of items per page
        serializer_func: Function to serialize each item (item -> dict)
        endpoint: Optional Flask endpoint name for URL generation
        endpoint_kwargs: Additional kwargs for url_for()

    Returns:
        dict: Standardized pagination response

    Example:
        def get_documents():
            query = Document.query.filter_by(is_deleted=False)
            return build_pagination_response(
                query, page=1, per_page=20,
                serializer_func=lambda d: d.to_dict(),
                endpoint='documents.list_documents'
            )
    """
    # Execute pagination
    paginated = query.paginate(page=page, per_page=per_page, error_out=False)

    # Serialize items
    items = [serializer_func(item) for item in paginated.items]

    # Build response
    response = {
        'items': items,
        'total': paginated.total,
        'pages': paginated.pages,
        'page': page,
        'per_page': per_page,
        'has_next': paginated.has_next,
        'has_prev': paginated.has_prev
    }

    # Add navigation URLs if endpoint provided
    if endpoint:
        if paginated.has_next:
            response['next_url'] = url_for(endpoint, page=page + 1, per_page=per_page, **endpoint_kwargs)
        if paginated.has_prev:
            response['prev_url'] = url_for(endpoint, page=page - 1, per_page=per_page, **endpoint_kwargs)

    return response


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
