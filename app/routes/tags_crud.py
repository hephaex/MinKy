"""Tag CRUD operations - Create, Read, Update, Delete, Suggest."""
from flask import Blueprint, request, Response
from flask_jwt_extended import jwt_required, get_jwt_identity
from pydantic import ValidationError
from app import db, limiter
from app.models.tag import Tag
from app.models.document import Document
from app.schemas.tag import TagCreate, TagUpdate
from app.utils.auth import get_current_user_id
from app.utils.responses import success_response, error_response
from app.utils.validation import format_validation_errors, escape_like
import bleach
import logging

logger = logging.getLogger(__name__)

tags_crud_bp = Blueprint('tags_crud', __name__)


@tags_crud_bp.route('/tags', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
def list_tags() -> Response | tuple[Response, int]:
    """Get all tags with optional filtering."""
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')
        popular = request.args.get('popular', 'false').lower() == 'true'

        if popular:
            popular_tags = Tag.get_popular_tags(limit=per_page)
            tags = [{'tag': tag.to_dict(document_count=count), 'document_count': count}
                    for tag, count in popular_tags]
            return success_response({
                'tags': tags,
                'total': len(tags),
                'popular': True
            })

        query = Tag.query

        if search:
            search_escaped = escape_like(search)
            query = query.filter(Tag.name.ilike(f'%{search_escaped}%'))

        pagination = query.order_by(Tag.name).paginate(
            page=page, per_page=per_page, error_out=False
        )

        # Pre-compute document counts in a single query to avoid N+1
        tag_ids = [tag.id for tag in pagination.items]
        counts = Tag.get_tags_with_counts(tag_ids) if tag_ids else {}

        tags = []
        for tag in pagination.items:
            count = counts.get(tag.id, 0)
            tag_dict = tag.to_dict(document_count=count)
            tag_dict['usage_count'] = count
            tags.append(tag_dict)

        tags.sort(key=lambda x: x['document_count'], reverse=True)

        return success_response({
            'tags': tags,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            },
            'search_query': search
        })

    except Exception as e:
        logger.error("Error listing tags: %s", e)
        return error_response('Internal server error', 500)


@tags_crud_bp.route('/tags', methods=['POST'])
@limiter.limit("30 per hour")
@jwt_required()
def create_tag() -> Response | tuple[Response, int]:
    """Create a new tag."""
    try:
        data = request.get_json()
        current_user_id = get_jwt_identity()

        if not data:
            return error_response('No data provided', 400)

        try:
            validated = TagCreate.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        name = bleach.clean(validated.name)
        description = bleach.clean(validated.description) if validated.description else None
        color = validated.color or '#007bff'

        existing_slug = Tag.create_slug(name)
        existing_tag = Tag.query.filter_by(slug=existing_slug).first()
        if existing_tag:
            return error_response('Tag already exists', 409, details={'tag': existing_tag.to_dict()})

        tag = Tag(
            name=name,
            description=description,
            color=color,
            created_by=current_user_id
        )

        db.session.add(tag)
        db.session.commit()

        return success_response({'tag': tag.to_dict()}, status_code=201)

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating tag: %s", e)
        return error_response('Internal server error', 500)


@tags_crud_bp.route('/tags/<slug>', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
def get_tag(slug: str) -> Response | tuple[Response, int]:
    """Get a specific tag and its documents."""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        current_user_id = get_current_user_id()
        include_private = request.args.get('include_private', 'false').lower() == 'true'

        pagination = Document.search_documents(
            '', page, per_page,
            user_id=current_user_id,
            include_private=include_private and current_user_id is not None,
            tags=[slug]
        )

        documents = [doc.to_dict_lite() for doc in pagination.items]

        return success_response({
            'tag': tag.to_dict(),
            'documents': documents,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            }
        })

    except Exception as e:
        logger.error("Error getting tag %s: %s", slug, e)
        return error_response('Internal server error', 500)


@tags_crud_bp.route('/tags/<slug>', methods=['PUT'])
@limiter.limit("60 per hour")
@jwt_required()
def update_tag(slug: str) -> Response | tuple[Response, int]:
    """Update a tag."""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()

        if tag.created_by != current_user_id:
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data:
            return error_response('No data provided', 400)

        try:
            validated = TagUpdate.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        if validated.description is not None:
            tag.description = bleach.clean(validated.description) if validated.description else None

        if validated.color is not None:
            tag.color = validated.color

        db.session.commit()

        return success_response({'tag': tag.to_dict()})

    except Exception as e:
        db.session.rollback()
        logger.error("Error updating tag %s: %s", slug, e)
        return error_response('Internal server error', 500)


@tags_crud_bp.route('/tags/<slug>', methods=['DELETE'])
@limiter.limit("30 per hour")
@jwt_required()
def delete_tag(slug: str) -> Response | tuple[Response, int]:
    """Delete a tag."""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()

        if tag.created_by != current_user_id:
            return error_response('Access denied', 403)

        for document in tag.documents:
            document.tags.remove(tag)

        db.session.delete(tag)
        db.session.commit()

        return success_response({'message': 'Tag deleted successfully'})

    except Exception as e:
        db.session.rollback()
        logger.error("Error deleting tag %s: %s", slug, e)
        return error_response('Internal server error', 500)


@tags_crud_bp.route('/tags/suggest', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
def suggest_tags() -> Response | tuple[Response, int]:
    """Get tag suggestions based on query."""
    try:
        query = request.args.get('q', '').strip()
        limit = request.args.get('limit', 10, type=int)

        if not query or len(query) < 2:
            return success_response({'suggestions': []})

        query_escaped = escape_like(query)
        tags = Tag.query.filter(Tag.name.ilike(f'%{query_escaped}%'))\
            .order_by(Tag.name)\
            .limit(limit)\
            .all()

        suggestions = [{'name': tag.name, 'slug': tag.slug, 'color': tag.color} for tag in tags]

        return success_response({'suggestions': suggestions})

    except Exception as e:
        logger.error("Error suggesting tags: %s", e)
        return error_response('Internal server error', 500)
