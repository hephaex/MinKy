from flask import Blueprint, request
from flask_jwt_extended import jwt_required
from app import db, limiter
from app.models.template import DocumentTemplate
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, success_response, error_response
from app.utils.validation import escape_like
import bleach
import logging
import json

logger = logging.getLogger(__name__)

templates_bp = Blueprint('templates', __name__)


def _log_template_operation(operation: str, user_id: int, template_id: int = None,
                            details: dict = None) -> None:
    """SECURITY: Audit log template operations for compliance."""
    log_entry = {
        'operation': operation,
        'user_id': user_id,
        'template_id': template_id,
        'ip_address': request.remote_addr if request else None,
        'details': details or {}
    }
    logger.info(f"AUDIT_TEMPLATE: {json.dumps(log_entry)}")

@templates_bp.route('/templates', methods=['GET'])
@limiter.limit("60 per minute")
def list_templates():
    """Get all public templates with optional filtering"""
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        # SECURITY: Enforce pagination bounds
        page = max(1, page)
        per_page = max(1, min(per_page, 100))
        category = request.args.get('category', '')
        search = request.args.get('search', '')
        featured = request.args.get('featured', 'false').lower() == 'true'
        popular = request.args.get('popular', 'false').lower() == 'true'
        include_content = request.args.get('include_content', 'false').lower() == 'true'
        
        if popular:
            templates = DocumentTemplate.get_popular_templates(limit=per_page)
            return success_response({
                'templates': [t.to_dict(include_content=include_content) for t in templates],
                'type': 'popular'
            })

        if featured:
            templates = DocumentTemplate.get_featured_templates()
            return success_response({
                'templates': [t.to_dict(include_content=include_content) for t in templates],
                'type': 'featured'
            })
        
        query = DocumentTemplate.query.filter_by(is_public=True)
        
        if category:
            query = query.filter_by(category=category)
        
        if search:
            search_escaped = escape_like(search)
            query = query.filter(
                db.or_(
                    DocumentTemplate.name.ilike(f'%{search_escaped}%'),
                    DocumentTemplate.description.ilike(f'%{search_escaped}%'),
                    DocumentTemplate.content_template.ilike(f'%{search_escaped}%')
                )
            )
        
        query = query.order_by(DocumentTemplate.updated_at.desc())
        return paginate_query(
            query, page, per_page,
            serializer_func=lambda t: t.to_dict(include_content=include_content),
            items_key='templates',
            extra_fields={'search_query': search, 'category': category}
        )
        
    except Exception as e:
        logger.error("Error listing templates: %s", e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates', methods=['POST'])
@limiter.limit("30 per hour")
@jwt_required()
def create_template():
    """Create a new template"""
    try:
        data = request.get_json()
        # SECURITY: Use get_current_user_id() for consistent int type
        current_user_id = get_current_user_id()
        
        if not data or 'name' not in data or 'title_template' not in data or 'content_template' not in data:
            return error_response('Name, title_template, and content_template are required', 400)

        # SECURITY: Validate input length to prevent abuse
        MAX_NAME_LENGTH = 200
        MAX_DESCRIPTION_LENGTH = 2000
        MAX_TEMPLATE_LENGTH = 100000  # 100KB

        if len(data.get('name', '')) > MAX_NAME_LENGTH:
            return error_response(f'Name too long (max {MAX_NAME_LENGTH} characters)', 400)
        if len(data.get('description', '') or '') > MAX_DESCRIPTION_LENGTH:
            return error_response(f'Description too long (max {MAX_DESCRIPTION_LENGTH} characters)', 400)
        if len(data.get('title_template', '')) > MAX_NAME_LENGTH:
            return error_response(f'Title template too long (max {MAX_NAME_LENGTH} characters)', 400)
        if len(data.get('content_template', '')) > MAX_TEMPLATE_LENGTH:
            return error_response(f'Content template too long (max {MAX_TEMPLATE_LENGTH} characters)', 400)

        # Sanitize input
        name = bleach.clean(data['name'].strip())
        description = bleach.clean(data.get('description', '').strip()) if data.get('description') else None
        category = bleach.clean(data.get('category', 'General').strip())

        if not name:
            return error_response('Template name cannot be empty', 400)

        template = DocumentTemplate(
            name=name,
            title_template=bleach.clean(data['title_template']),
            content_template=bleach.clean(data['content_template']),
            created_by=current_user_id,
            description=description,
            category=category,
            is_public=data.get('is_public', True)
        )

        db.session.add(template)
        db.session.commit()

        # SECURITY: Audit log template creation
        _log_template_operation(
            'create',
            current_user_id,
            template.id,
            {'name': template.name, 'is_public': template.is_public}
        )

        return success_response(template.to_dict(), status_code=201)

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating template: %s", e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/<int:template_id>', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required(optional=True)
def get_template(template_id):
    """Get a specific template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_current_user_id()

        # Check visibility
        if not template.is_public and template.created_by != current_user_id:
            return error_response('Template not found', 404)

        return success_response(template.to_dict())

    except Exception as e:
        logger.error("Error getting template %s: %s", template_id, e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/<int:template_id>', methods=['PUT'])
@limiter.limit("60 per hour")
@jwt_required()
def update_template(template_id):
    """Update a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        # SECURITY: Use get_current_user_id() for consistent int type comparison
        current_user_id = get_current_user_id()

        if not template.can_edit(current_user_id):
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data:
            return error_response('No data provided', 400)

        # SECURITY: Validate input length on update (same as create)
        MAX_NAME_LENGTH = 200
        MAX_DESCRIPTION_LENGTH = 2000
        MAX_TEMPLATE_LENGTH = 100000  # 100KB

        if 'name' in data and len(data.get('name', '')) > MAX_NAME_LENGTH:
            return error_response(f'Name too long (max {MAX_NAME_LENGTH} characters)', 400)
        if 'description' in data and len(data.get('description', '') or '') > MAX_DESCRIPTION_LENGTH:
            return error_response(f'Description too long (max {MAX_DESCRIPTION_LENGTH} characters)', 400)
        if 'title_template' in data and len(data.get('title_template', '')) > MAX_NAME_LENGTH:
            return error_response(f'Title template too long (max {MAX_NAME_LENGTH} characters)', 400)
        if 'content_template' in data and len(data.get('content_template', '')) > MAX_TEMPLATE_LENGTH:
            return error_response(f'Content template too long (max {MAX_TEMPLATE_LENGTH} characters)', 400)

        # Update fields
        if 'name' in data:
            name = bleach.clean(data['name'].strip())
            if name:
                template.name = name

        if 'description' in data:
            template.description = bleach.clean(data['description'].strip()) if data['description'] else None

        if 'category' in data:
            template.category = bleach.clean(data['category'].strip())

        if 'title_template' in data:
            template.title_template = bleach.clean(data['title_template'])

        if 'content_template' in data:
            template.content_template = bleach.clean(data['content_template'])

        # SECURITY: Track visibility changes for audit
        old_visibility = template.is_public
        if 'is_public' in data:
            template.is_public = bool(data['is_public'])

        template.updated_at = db.func.now()
        db.session.commit()

        # SECURITY: Audit log template update with visibility change tracking
        _log_template_operation(
            'update',
            current_user_id,
            template_id,
            {
                'fields_updated': list(data.keys()),
                'visibility_changed': old_visibility != template.is_public
            }
        )

        return success_response(template.to_dict())

    except Exception as e:
        db.session.rollback()
        logger.error("Error updating template %s: %s", template_id, e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/<int:template_id>', methods=['DELETE'])
@limiter.limit("30 per hour")
@jwt_required()
def delete_template(template_id):
    """Delete a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        # SECURITY: Use get_current_user_id() for consistent int type comparison
        current_user_id = get_current_user_id()

        if not template.can_delete(current_user_id):
            return error_response('Access denied', 403)

        template_name = template.name
        db.session.delete(template)
        db.session.commit()

        # SECURITY: Audit log template deletion
        _log_template_operation(
            'delete',
            current_user_id,
            template_id,
            {'name': template_name}
        )

        return success_response(message='Template deleted successfully')

    except Exception as e:
        db.session.rollback()
        logger.error("Error deleting template %s: %s", template_id, e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/<int:template_id>/create-document', methods=['POST'])
@limiter.limit("20 per hour")
@jwt_required()
def create_document_from_template(template_id):
    """Create a new document from a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        # SECURITY: Use get_current_user_id() for consistent int type
        current_user_id = get_current_user_id()

        # Check if template is accessible
        if not template.is_public and template.created_by != current_user_id:
            return error_response('Template not found', 404)

        data = request.get_json() or {}
        variables = data.get('variables', {})
        author = data.get('author')
        tags = data.get('tags', [])
        is_public = data.get('is_public', True)

        # Create document from template
        document = template.create_document(
            variables=variables,
            author=author,
            user_id=current_user_id,
            tags=tags
        )

        document.is_public = is_public

        db.session.add(document)
        db.session.commit()

        # SECURITY: Audit log document creation from template
        _log_template_operation(
            'create_document',
            current_user_id,
            template_id,
            {'document_id': document.id}
        )

        return success_response({
            'message': 'Document created from template successfully',
            'document': document.to_dict(),
            'template': {
                'id': template.id,
                'name': template.name
            }
        }, status_code=201)

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating document from template %s: %s", template_id, e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/categories', methods=['GET'])
@limiter.limit("60 per minute")
def get_template_categories():
    """Get all template categories"""
    try:
        categories = DocumentTemplate.get_categories()
        return success_response({'categories': categories})

    except Exception as e:
        logger.error("Error getting template categories: %s", e)
        return error_response('Internal server error', 500)

@templates_bp.route('/templates/<int:template_id>/preview', methods=['POST'])
@limiter.limit("30 per minute")
@jwt_required(optional=True)
def preview_template(template_id):
    """Preview a template with variables"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_current_user_id()

        # Check if template is accessible
        if not template.is_public and template.created_by != current_user_id:
            return error_response('Template not found', 404)

        data = request.get_json() or {}
        variables = data.get('variables', {})

        rendered_title = template.render_title(variables)
        rendered_content = template.render_content(variables)

        # Convert to HTML for preview with XSS sanitization
        import markdown
        import bleach
        from urllib.parse import urlparse

        # SECURITY: URL protocol validator to prevent javascript: XSS
        def validate_url_protocol(tag, name, value):
            """Only allow safe URL protocols"""
            if name in ('href', 'src'):
                if not value:
                    return False
                parsed = urlparse(value)
                # Allow only http, https, mailto, and relative URLs
                if parsed.scheme and parsed.scheme.lower() not in ('http', 'https', 'mailto'):
                    return False
            return True

        raw_html = markdown.markdown(rendered_content)
        # SECURITY: Sanitize HTML to prevent XSS
        allowed_tags = [
            'p', 'br', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
            'strong', 'em', 'u', 's', 'code', 'pre', 'blockquote',
            'ul', 'ol', 'li', 'a', 'img', 'table', 'thead', 'tbody',
            'tr', 'th', 'td', 'hr', 'div', 'span'
        ]
        # SECURITY: Use callable for href/src to validate URL protocols
        allowed_attrs = {
            'a': validate_url_protocol,
            'img': validate_url_protocol,
            '*': ['class']
        }
        rendered_html = bleach.clean(raw_html, tags=allowed_tags, attributes=allowed_attrs)

        return success_response({
            'title': rendered_title,
            'content': rendered_content,
            'html': rendered_html,
            'variables_used': template.get_template_variables()
        })

    except Exception as e:
        logger.error("Error previewing template %s: %s", template_id, e)
        return error_response('Internal server error', 500)

@templates_bp.route('/my-templates', methods=['GET'])
@limiter.limit("60 per minute")
@jwt_required()
def get_my_templates():
    """Get current user's templates"""
    try:
        # SECURITY: Use get_current_user_id() for consistent int type
        current_user_id = get_current_user_id()

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        # SECURITY: Enforce pagination bounds
        page = max(1, page)
        per_page = max(1, min(per_page, 100))
        include_content = request.args.get('include_content', 'false').lower() == 'true'

        query = DocumentTemplate.query.filter_by(created_by=current_user_id)\
            .order_by(DocumentTemplate.updated_at.desc())

        return paginate_query(
            query, page, per_page,
            serializer_func=lambda t: t.to_dict(include_content=include_content),
            items_key='templates'
        )

    except Exception as e:
        logger.error("Error getting user templates: %s", e)
        return error_response('Internal server error', 500)