from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db
from app.models.template import DocumentTemplate
from app.models.document import Document
import bleach

templates_bp = Blueprint('templates', __name__)

def get_current_user_id():
    try:
        return get_jwt_identity()
    except:
        return None

@templates_bp.route('/templates', methods=['GET'])
def list_templates():
    """Get all public templates with optional filtering"""
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        category = request.args.get('category', '')
        search = request.args.get('search', '')
        featured = request.args.get('featured', 'false').lower() == 'true'
        popular = request.args.get('popular', 'false').lower() == 'true'
        include_content = request.args.get('include_content', 'false').lower() == 'true'
        
        if popular:
            templates = DocumentTemplate.get_popular_templates(limit=per_page)
            return jsonify({
                'templates': [t.to_dict(include_content=include_content) for t in templates],
                'type': 'popular'
            })
        
        if featured:
            templates = DocumentTemplate.get_featured_templates()
            return jsonify({
                'templates': [t.to_dict(include_content=include_content) for t in templates],
                'type': 'featured'
            })
        
        query = DocumentTemplate.query.filter_by(is_public=True)
        
        if category:
            query = query.filter_by(category=category)
        
        if search:
            query = query.filter(
                db.or_(
                    DocumentTemplate.name.ilike(f'%{search}%'),
                    DocumentTemplate.description.ilike(f'%{search}%'),
                    DocumentTemplate.content_template.ilike(f'%{search}%')
                )
            )
        
        pagination = query.order_by(DocumentTemplate.updated_at.desc()).paginate(
            page=page, per_page=per_page, error_out=False
        )
        
        templates = [t.to_dict(include_content=include_content) for t in pagination.items]
        
        return jsonify({
            'templates': templates,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            },
            'search_query': search,
            'category': category
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates', methods=['POST'])
@jwt_required()
def create_template():
    """Create a new template"""
    try:
        data = request.get_json()
        current_user_id = get_jwt_identity()
        
        if not data or 'name' not in data or 'title_template' not in data or 'content_template' not in data:
            return jsonify({'error': 'Name, title_template, and content_template are required'}), 400
        
        # Sanitize input
        name = bleach.clean(data['name'].strip())
        description = bleach.clean(data.get('description', '').strip()) if data.get('description') else None
        category = bleach.clean(data.get('category', 'General').strip())
        
        if not name:
            return jsonify({'error': 'Template name cannot be empty'}), 400
        
        template = DocumentTemplate(
            name=name,
            title_template=data['title_template'],
            content_template=data['content_template'],
            created_by=current_user_id,
            description=description,
            category=category,
            is_public=data.get('is_public', True)
        )
        
        db.session.add(template)
        db.session.commit()
        
        return jsonify(template.to_dict()), 201
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/<int:template_id>', methods=['GET'])
def get_template(template_id):
    """Get a specific template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_current_user_id()
        
        # Check visibility
        if not template.is_public and template.created_by != current_user_id:
            return jsonify({'error': 'Template not found'}), 404
        
        return jsonify(template.to_dict())
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/<int:template_id>', methods=['PUT'])
@jwt_required()
def update_template(template_id):
    """Update a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_jwt_identity()
        
        if not template.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        data = request.get_json()
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
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
            template.title_template = data['title_template']
        
        if 'content_template' in data:
            template.content_template = data['content_template']
        
        if 'is_public' in data:
            template.is_public = bool(data['is_public'])
        
        template.updated_at = db.func.now()
        db.session.commit()
        
        return jsonify(template.to_dict())
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/<int:template_id>', methods=['DELETE'])
@jwt_required()
def delete_template(template_id):
    """Delete a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_jwt_identity()
        
        if not template.can_delete(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        db.session.delete(template)
        db.session.commit()
        
        return jsonify({'message': 'Template deleted successfully'})
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/<int:template_id>/create-document', methods=['POST'])
@jwt_required()
def create_document_from_template(template_id):
    """Create a new document from a template"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_jwt_identity()
        
        # Check if template is accessible
        if not template.is_public and template.created_by != current_user_id:
            return jsonify({'error': 'Template not found'}), 404
        
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
        
        return jsonify({
            'message': 'Document created from template successfully',
            'document': document.to_dict(),
            'template': {
                'id': template.id,
                'name': template.name
            }
        }), 201
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/categories', methods=['GET'])
def get_template_categories():
    """Get all template categories"""
    try:
        categories = DocumentTemplate.get_categories()
        return jsonify({'categories': categories})
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/templates/<int:template_id>/preview', methods=['POST'])
def preview_template(template_id):
    """Preview a template with variables"""
    try:
        template = DocumentTemplate.query.get_or_404(template_id)
        current_user_id = get_current_user_id()
        
        # Check if template is accessible
        if not template.is_public and template.created_by != current_user_id:
            return jsonify({'error': 'Template not found'}), 404
        
        data = request.get_json() or {}
        variables = data.get('variables', {})
        
        rendered_title = template.render_title(variables)
        rendered_content = template.render_content(variables)
        
        # Convert to HTML for preview
        import markdown
        rendered_html = markdown.markdown(rendered_content)
        
        return jsonify({
            'title': rendered_title,
            'content': rendered_content,
            'html': rendered_html,
            'variables_used': template.get_template_variables()
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@templates_bp.route('/my-templates', methods=['GET'])
@jwt_required()
def get_my_templates():
    """Get current user's templates"""
    try:
        current_user_id = get_jwt_identity()
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        include_content = request.args.get('include_content', 'false').lower() == 'true'
        
        pagination = DocumentTemplate.query.filter_by(created_by=current_user_id)\
            .order_by(DocumentTemplate.updated_at.desc())\
            .paginate(page=page, per_page=per_page, error_out=False)
        
        templates = [t.to_dict(include_content=include_content) for t in pagination.items]
        
        return jsonify({
            'templates': templates,
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
        return jsonify({'error': str(e)}), 500