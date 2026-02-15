"""Full-text search endpoints for documents."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from sqlalchemy import func, text, or_
from app import db, limiter
from app.models.document import Document
from app.utils.auth import get_current_user_id
import bleach
import logging

logger = logging.getLogger(__name__)

documents_search_bp = Blueprint('documents_search', __name__)


def _build_search_query(query_text, current_user_id, include_private):
    """Build PostgreSQL full-text search query"""
    base_query = Document.query

    # SECURITY: Authorization logic for document visibility
    # - include_private=False: Only public documents
    # - include_private=True AND authenticated: Public + user's own documents
    # - include_private=True AND NOT authenticated: Fall back to public only (prevent auth bypass)
    if not include_private:
        base_query = base_query.filter(Document.is_public == True)
    elif current_user_id:
        # Authenticated user requesting private: show public + their own
        base_query = base_query.filter(
            or_(Document.is_public == True, Document.user_id == current_user_id)
        )
    else:
        # SECURITY: Unauthenticated user requesting include_private=True
        # Fall back to public only to prevent authorization bypass
        base_query = base_query.filter(Document.is_public == True)

    search_vector = func.to_tsvector('english',
        func.coalesce(Document.title, '') + ' ' +
        func.coalesce(Document.markdown_content, '')
    )
    search_query = func.plainto_tsquery('english', query_text)
    rank = func.ts_rank(search_vector, search_query)

    filtered_query = base_query.filter(search_vector.op('@@')(search_query))
    ordered_query = filtered_query.order_by(rank.desc())

    return ordered_query, rank


def _generate_search_headline(doc, query_text):
    """Generate highlighted search snippet with XSS protection"""
    # SECURITY: Only allow <mark> tags in highlight output to prevent XSS
    HIGHLIGHT_ALLOWED_TAGS = ['mark']

    headline_sql = text("""
        SELECT ts_headline('english',
            COALESCE(:content, ''),
            plainto_tsquery('english', :query),
            'MaxWords=50, MinWords=20, StartSel=<mark>, StopSel=</mark>'
        )
    """)
    try:
        headline_result = db.session.execute(
            headline_sql,
            {'content': doc.markdown_content or '', 'query': query_text}
        ).scalar()
        # SECURITY: Sanitize output to only allow <mark> tags, preventing XSS
        if headline_result:
            return bleach.clean(headline_result, tags=HIGHLIGHT_ALLOWED_TAGS, strip=True)
        return None
    except Exception as e:
        logger.debug("Headline generation failed: %s", e)
        return None


def _format_search_results(pagination_items, rank, query_text, include_highlight):
    """Format search results with optional highlighting"""
    results = []
    for doc in pagination_items:
        # Use lite serialization for search results to avoid N+1 queries
        doc_dict = doc.to_dict_lite()
        doc_dict['relevance_score'] = float(rank) if rank else 0.0

        if include_highlight:
            doc_dict['highlight'] = _generate_search_headline(doc, query_text)

        results.append(doc_dict)

    return results


@documents_search_bp.route('/documents/search', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required(optional=True)
def search_documents_fulltext():
    """Full-text search with PostgreSQL tsvector/tsquery
    ---
    tags:
      - Documents
    parameters:
      - name: q
        in: query
        type: string
        required: true
        description: Search query
      - name: page
        in: query
        type: integer
        default: 1
      - name: per_page
        in: query
        type: integer
        default: 10
      - name: highlight
        in: query
        type: boolean
        default: true
        description: Include search highlights
    responses:
      200:
        description: Search results with relevance ranking
    """
    try:
        query_text = request.args.get('q', '').strip()
        page = max(1, request.args.get('page', 1, type=int))
        per_page = min(100, max(1, request.args.get('per_page', 10, type=int)))
        include_highlight = request.args.get('highlight', 'true').lower() == 'true'
        include_private = request.args.get('include_private', 'false').lower() == 'true'

        current_user_id = get_current_user_id()

        if not query_text:
            return jsonify({'error': 'Search query is required'}), 400

        # SECURITY: Validate search query length to prevent DoS
        MAX_SEARCH_QUERY_LENGTH = 500
        if len(query_text) > MAX_SEARCH_QUERY_LENGTH:
            return jsonify({'error': f'Search query too long. Maximum {MAX_SEARCH_QUERY_LENGTH} characters'}), 400

        ordered_query, rank = _build_search_query(query_text, current_user_id, include_private)
        pagination = ordered_query.paginate(page=page, per_page=per_page, error_out=False)
        results = _format_search_results(pagination.items, rank, query_text, include_highlight)

        return jsonify({
            'documents': results,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            },
            # SECURITY: Sanitize search query in response to prevent XSS reflection
            'search_query': bleach.clean(query_text, tags=[], strip=True),
            'search_engine': 'postgresql_fulltext'
        })

    except Exception as e:
        logger.error("Full-text search failed: %s", e)
        return jsonify({'error': 'Internal server error'}), 500
