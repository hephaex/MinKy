from flask import Blueprint, request, Response
from flask_jwt_extended import jwt_required, get_jwt_identity
from pydantic import ValidationError
from sqlalchemy import func
from app import db, cache
from app.models.tag import Tag
from app.models.document import Document
from app.schemas.tag import TagCreate, TagUpdate
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, get_or_404, success_response, error_response
from app.utils.validation import format_validation_errors, escape_like
from app.utils.auto_tag import detect_auto_tags, merge_tags
import bleach
import logging

logger = logging.getLogger(__name__)

tags_bp = Blueprint('tags', __name__)

@tags_bp.route('/tags', methods=['GET'])
def list_tags() -> Response | tuple[Response, int]:
    """Get all tags with optional filtering
    ---
    tags:
      - Tags
    parameters:
      - name: page
        in: query
        type: integer
        default: 1
        description: Page number
      - name: per_page
        in: query
        type: integer
        default: 20
        description: Items per page
      - name: search
        in: query
        type: string
        description: Search query for tag names
      - name: popular
        in: query
        type: boolean
        default: false
        description: Get popular tags ordered by usage
    responses:
      200:
        description: List of tags
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                tags:
                  type: array
                  items:
                    type: object
                    properties:
                      id:
                        type: integer
                      name:
                        type: string
                      slug:
                        type: string
                      color:
                        type: string
                      document_count:
                        type: integer
                pagination:
                  type: object
    """
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')
        popular = request.args.get('popular', 'false').lower() == 'true'
        
        if popular:
            # Get popular tags ordered by document count
            popular_tags = Tag.get_popular_tags(limit=per_page)
            tags = [{'tag': tag.to_dict(), 'document_count': count} for tag, count in popular_tags]

            return success_response({
                'tags': tags,
                'total': len(tags),
                'popular': True
            })
        
        query = Tag.query
        
        if search:
            search_escaped = escape_like(search)
            query = query.filter(Tag.name.ilike(f'%{search_escaped}%'))
        
        # Order by document count (calculated in to_dict)
        pagination = query.order_by(Tag.name).paginate(
            page=page, per_page=per_page, error_out=False
        )
        
        # Include document count and usage statistics
        tags = []
        for tag in pagination.items:
            tag_dict = tag.to_dict()
            tag_dict['usage_count'] = tag_dict['document_count']  # Alias for compatibility
            tags.append(tag_dict)
        
        # Sort by document count after loading
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
        return error_response(str(e), 500)

@tags_bp.route('/tags', methods=['POST'])
@jwt_required()
def create_tag() -> Response | tuple[Response, int]:
    """Create a new tag
    ---
    tags:
      - Tags
    security:
      - Bearer: []
    parameters:
      - in: body
        name: body
        required: true
        schema:
          type: object
          required:
            - name
          properties:
            name:
              type: string
              description: Tag name (2-50 chars)
              example: Python
            description:
              type: string
              description: Tag description
              example: Python programming language
            color:
              type: string
              pattern: "^#[0-9A-Fa-f]{6}$"
              description: Hex color code
              example: "#3776AB"
    responses:
      201:
        description: Tag created successfully
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                tag:
                  type: object
      400:
        description: Validation error
      401:
        description: Unauthorized
      409:
        description: Tag already exists
    """
    try:
        data = request.get_json()
        current_user_id = get_jwt_identity()

        if not data:
            return error_response('No data provided', 400)

        # Validate with Pydantic schema
        try:
            validated = TagCreate.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        name = bleach.clean(validated.name)
        description = bleach.clean(validated.description) if validated.description else None
        color = validated.color or '#007bff'

        # Check if tag already exists
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
        return error_response(str(e), 500)

@tags_bp.route('/tags/<slug>', methods=['GET'])
def get_tag(slug: str) -> Response | tuple[Response, int]:
    """Get a specific tag and its documents
    ---
    tags:
      - Tags
    parameters:
      - name: slug
        in: path
        type: string
        required: true
        description: Tag slug
      - name: page
        in: query
        type: integer
        default: 1
      - name: per_page
        in: query
        type: integer
        default: 10
      - name: include_private
        in: query
        type: boolean
        default: false
    responses:
      200:
        description: Tag with documents
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                tag:
                  type: object
                documents:
                  type: array
                pagination:
                  type: object
      404:
        description: Tag not found
    """
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        current_user_id = get_current_user_id()
        include_private = request.args.get('include_private', 'false').lower() == 'true'

        # Get documents with this tag
        pagination = Document.search_documents(
            '', page, per_page,
            user_id=current_user_id,
            include_private=include_private and current_user_id is not None,
            tags=[slug]
        )

        documents = [doc.to_dict() for doc in pagination.items]

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
        return error_response(str(e), 500)

@tags_bp.route('/tags/<slug>', methods=['PUT'])
@jwt_required()
def update_tag(slug: str) -> Response | tuple[Response, int]:
    """Update a tag"""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()

        # Only tag creator can update (or admin in future)
        if tag.created_by != current_user_id:
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data:
            return error_response('No data provided', 400)

        # Validate with Pydantic schema
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
        return error_response(str(e), 500)

@tags_bp.route('/tags/<slug>', methods=['DELETE'])
@jwt_required()
def delete_tag(slug: str) -> Response | tuple[Response, int]:
    """Delete a tag"""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()

        # Only tag creator can delete (or admin in future)
        if tag.created_by != current_user_id:
            return error_response('Access denied', 403)

        # Remove tag from all documents first
        for document in tag.documents:
            document.tags.remove(tag)

        db.session.delete(tag)
        db.session.commit()

        return success_response({'message': 'Tag deleted successfully'})

    except Exception as e:
        db.session.rollback()
        return error_response(str(e), 500)

@tags_bp.route('/tags/suggest', methods=['GET'])
def suggest_tags() -> Response | tuple[Response, int]:
    """Get tag suggestions based on query"""
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
        return error_response(str(e), 500)

@tags_bp.route('/tags/statistics', methods=['GET'])
@cache.cached(timeout=60)
def get_tags_statistics() -> Response | tuple[Response, int]:
    """Get comprehensive tag statistics
    ---
    tags:
      - Tags
    responses:
      200:
        description: Tag statistics
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                total_tags:
                  type: integer
                auto_generated_tags:
                  type: integer
                manual_tags:
                  type: integer
                popular_tags:
                  type: array
                recent_tags:
                  type: array
                usage_distribution:
                  type: object
                  properties:
                    unused:
                      type: integer
                    low:
                      type: integer
                    medium:
                      type: integer
                    high:
                      type: integer
    """
    try:
        
        # Total tags
        total_tags = Tag.query.count()
        
        # Most popular tags
        popular_tags_data = Tag.get_popular_tags(limit=10)
        popular_list = [{'name': tag.name, 'usage_count': count, 'color': tag.color} for tag, count in popular_tags_data]
        
        # Recently created tags
        recent_tags = Tag.query.order_by(Tag.created_at.desc()).limit(5).all()
        recent_list = [{'name': tag.name, 'created_at': tag.created_at.isoformat(), 'color': tag.color} for tag in recent_tags]
        
        # Tag usage distribution - single query with LEFT JOIN instead of N+1
        tag_counts = db.session.query(
            Tag.id,
            func.count(Document.id).label('doc_count')
        ).outerjoin(
            Tag.documents
        ).group_by(Tag.id).all()

        usage_distribution = {'unused': 0, 'low': 0, 'medium': 0, 'high': 0}
        for _, doc_count in tag_counts:
            if doc_count == 0:
                usage_distribution['unused'] += 1
            elif doc_count <= 5:
                usage_distribution['low'] += 1
            elif doc_count <= 20:
                usage_distribution['medium'] += 1
            else:
                usage_distribution['high'] += 1
        
        # Auto-generated tags (tags without description)
        auto_generated_count = Tag.query.filter(Tag.description.is_(None)).count()
        
        return success_response({
            'total_tags': total_tags,
            'auto_generated_tags': auto_generated_count,
            'manual_tags': total_tags - auto_generated_count,
            'popular_tags': popular_list,
            'recent_tags': recent_list,
            'usage_distribution': usage_distribution
        })

    except Exception as e:
        return error_response(str(e), 500)

@tags_bp.route('/tags/auto-generate', methods=['POST'])
@jwt_required(optional=True)
def generate_auto_tags() -> Response | tuple[Response, int]:
    """Generate automatic tags for documents without tags"""
    try:
        current_user_id = get_current_user_id()
        data = request.get_json() or {}
        
        # Parameters
        document_id = data.get('document_id')  # Single document
        limit = data.get('limit', 100)  # Batch processing limit
        dry_run = data.get('dry_run', False)  # Preview mode
        
        results = {
            'processed': 0,
            'tagged': 0,
            'errors': 0,
            'documents': []
        }
        
        if document_id:
            # Process single document
            document = db.session.get(Document, document_id)
            if not document:
                return error_response('Document not found', 404)

            # Check access permissions
            if not document.can_view(current_user_id):
                return error_response('Access denied', 403)
            
            documents = [document]
        else:
            # Process documents without tags (batch)
            query = Document.query.filter(~Document.tags.any())
            
            # Filter by access permissions
            if current_user_id:
                query = query.filter(
                    (Document.is_public == True) | (Document.user_id == current_user_id)
                )
            else:
                query = query.filter(Document.is_public == True)
            
            documents = query.limit(limit).all()
        
        logger.info("AUTO_TAG_GENERATION: Processing %d documents", len(documents))
        
        for doc in documents:
            try:
                results['processed'] += 1
                
                # Skip if document already has tags
                if doc.tags:
                    results['documents'].append({
                        'id': doc.id,
                        'title': doc.title,
                        'status': 'skipped',
                        'reason': 'already_has_tags',
                        'existing_tags': [tag.name for tag in doc.tags]
                    })
                    continue
                
                # Detect auto tags
                content = doc.markdown_content or doc.content or ''
                auto_tags = detect_auto_tags(content)
                
                doc_result = {
                    'id': doc.id,
                    'title': doc.title,
                    'detected_tags': auto_tags,
                    'status': 'processed'
                }
                
                if auto_tags:
                    if not dry_run:
                        # Apply tags to document
                        doc.add_tags(auto_tags)
                        doc_result['status'] = 'tagged'
                        doc_result['added_tags'] = auto_tags
                    else:
                        doc_result['status'] = 'preview'
                        doc_result['would_add_tags'] = auto_tags
                    
                    results['tagged'] += 1
                else:
                    doc_result['status'] = 'no_tags_detected'
                
                results['documents'].append(doc_result)
                
            except Exception as e:
                results['errors'] += 1
                results['documents'].append({
                    'id': doc.id,
                    'title': doc.title,
                    'status': 'error',
                    'error': str(e)
                })
                logger.error("AUTO_TAG_GENERATION: Error processing document %s: %s", doc.id, e)
        
        if not dry_run:
            db.session.commit()
            logger.info("AUTO_TAG_GENERATION: Committed changes to database")

        return success_response({
            'dry_run': dry_run,
            'results': results,
            'summary': {
                'total_processed': results['processed'],
                'documents_tagged': results['tagged'],
                'errors': results['errors']
            }
        })

    except Exception as e:
        if not dry_run:
            db.session.rollback()
        logger.error("AUTO_TAG_GENERATION: Fatal error: %s", e)
        return error_response(str(e), 500)

@tags_bp.route('/tags/tagless-documents', methods=['GET'])
@jwt_required(optional=True)
def get_tagless_documents() -> Response | tuple[Response, int]:
    """Get documents that don't have any tags"""
    try:
        current_user_id = get_current_user_id()
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        include_private = request.args.get('include_private', 'false').lower() == 'true'
        
        # Query documents without tags
        query = Document.query.filter(~Document.tags.any())
        
        # Filter by access permissions
        if current_user_id and include_private:
            query = query.filter(
                (Document.is_public == True) | (Document.user_id == current_user_id)
            )
        else:
            query = query.filter(Document.is_public == True)
        
        def serialize_with_preview(doc):
            doc_dict = doc.to_dict()
            content = doc.markdown_content or doc.content or ''
            doc_dict['preview_auto_tags'] = detect_auto_tags(content)
            return doc_dict

        query = query.order_by(Document.created_at.desc())
        return paginate_query(
            query, page, per_page,
            serializer_func=serialize_with_preview,
            items_key='documents',
            extra_fields={'include_private': include_private and current_user_id is not None}
        )

    except Exception as e:
        return error_response(str(e), 500)

@tags_bp.route('/tags/preview-auto-tags/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def preview_auto_tags(document_id: int) -> Response | tuple[Response, int]:
    """Preview what auto tags would be generated for a specific document"""
    try:
        current_user_id = get_current_user_id()
        
        document = get_or_404(Document, document_id)
        
        # Check access permissions
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        # Detect auto tags
        content = document.markdown_content or document.content or ''
        auto_tags = detect_auto_tags(content)

        # Get existing tags
        existing_tags = [tag.name for tag in document.tags]

        # Merge tags to see final result
        merged_tags = merge_tags(existing_tags, auto_tags)

        return success_response({
            'document': {
                'id': document.id,
                'title': document.title,
                'has_tags': len(existing_tags) > 0
            },
            'existing_tags': existing_tags,
            'detected_auto_tags': auto_tags,
            'merged_tags': merged_tags,
            'new_tags': [tag for tag in merged_tags if tag not in existing_tags]
        })

    except Exception as e:
        return error_response(str(e), 500)