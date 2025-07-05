from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db
from app.models.tag import Tag
from app.models.document import Document
import bleach

tags_bp = Blueprint('tags', __name__)

def get_current_user_id():
    try:
        return get_jwt_identity()
    except:
        return None

@tags_bp.route('/tags', methods=['GET'])
def list_tags():
    """Get all tags with optional filtering"""
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')
        popular = request.args.get('popular', 'false').lower() == 'true'
        
        if popular:
            # Get popular tags ordered by document count
            popular_tags = Tag.get_popular_tags(limit=per_page)
            tags = [{'tag': tag.to_dict(), 'document_count': count} for tag, count in popular_tags]
            
            return jsonify({
                'tags': tags,
                'total': len(tags),
                'popular': True
            })
        
        query = Tag.query
        
        if search:
            query = query.filter(Tag.name.ilike(f'%{search}%'))
        
        pagination = query.order_by(Tag.name).paginate(
            page=page, per_page=per_page, error_out=False
        )
        
        tags = [tag.to_dict() for tag in pagination.items]
        
        return jsonify({
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
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags', methods=['POST'])
@jwt_required()
def create_tag():
    """Create a new tag"""
    try:
        data = request.get_json()
        current_user_id = get_jwt_identity()
        
        if not data or 'name' not in data:
            return jsonify({'error': 'Tag name is required'}), 400
        
        name = bleach.clean(data['name'].strip())
        description = bleach.clean(data.get('description', '').strip()) if data.get('description') else None
        color = data.get('color', '#007bff')
        
        if not name:
            return jsonify({'error': 'Tag name cannot be empty'}), 400
        
        # Validate color format
        import re
        if not re.match(r'^#[0-9A-Fa-f]{6}$', color):
            color = '#007bff'
        
        # Check if tag already exists
        existing_slug = Tag.create_slug(name)
        existing_tag = Tag.query.filter_by(slug=existing_slug).first()
        if existing_tag:
            return jsonify({'error': 'Tag already exists', 'tag': existing_tag.to_dict()}), 409
        
        tag = Tag(
            name=name,
            description=description,
            color=color,
            created_by=current_user_id
        )
        
        db.session.add(tag)
        db.session.commit()
        
        return jsonify(tag.to_dict()), 201
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/<slug>', methods=['GET'])
def get_tag(slug):
    """Get a specific tag and its documents"""
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
        
        return jsonify({
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
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/<slug>', methods=['PUT'])
@jwt_required()
def update_tag(slug):
    """Update a tag"""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()
        
        # Only tag creator can update (or admin in future)
        if tag.created_by != current_user_id:
            return jsonify({'error': 'Access denied'}), 403
        
        data = request.get_json()
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
        if 'description' in data:
            tag.description = bleach.clean(data['description'].strip()) if data['description'] else None
        
        if 'color' in data:
            color = data['color']
            import re
            if re.match(r'^#[0-9A-Fa-f]{6}$', color):
                tag.color = color
        
        db.session.commit()
        
        return jsonify(tag.to_dict())
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/<slug>', methods=['DELETE'])
@jwt_required()
def delete_tag(slug):
    """Delete a tag"""
    try:
        tag = Tag.query.filter_by(slug=slug).first_or_404()
        current_user_id = get_jwt_identity()
        
        # Only tag creator can delete (or admin in future)
        if tag.created_by != current_user_id:
            return jsonify({'error': 'Access denied'}), 403
        
        # Remove tag from all documents first
        for document in tag.documents:
            document.tags.remove(tag)
        
        db.session.delete(tag)
        db.session.commit()
        
        return jsonify({'message': 'Tag deleted successfully'})
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/suggest', methods=['GET'])
def suggest_tags():
    """Get tag suggestions based on query"""
    try:
        query = request.args.get('q', '').strip()
        limit = request.args.get('limit', 10, type=int)
        
        if not query or len(query) < 2:
            return jsonify({'suggestions': []})
        
        tags = Tag.query.filter(Tag.name.ilike(f'%{query}%'))\
            .order_by(Tag.name)\
            .limit(limit)\
            .all()
        
        suggestions = [{'name': tag.name, 'slug': tag.slug, 'color': tag.color} for tag in tags]
        
        return jsonify({'suggestions': suggestions})
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500