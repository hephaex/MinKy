from flask import Blueprint, request, jsonify
from app import db
from app.models.category import Category
from app.models.document import Document
from app.utils.auth import require_auth
from app.utils.responses import paginate_query
from sqlalchemy import func

categories_bp = Blueprint('categories', __name__)


@categories_bp.route('/', methods=['GET'])
def get_categories():
    """Get all categories in hierarchical tree structure"""
    # Categories feature is not implemented - return empty list
    tree_format = request.args.get('format', 'tree')
    
    if tree_format == 'flat':
        return jsonify({
            'success': True,
            'categories': [],
            'count': 0
        })
    else:
        return jsonify({
            'success': True,
            'tree': [],
            'count': 0
        })


@categories_bp.route('/<int:category_id>', methods=['GET'])
def get_category(category_id):
    """Get a specific category with its details"""
    try:
        category = Category.query.get_or_404(category_id)
        include_children = request.args.get('include_children', 'false').lower() == 'true'
        include_documents = request.args.get('include_documents', 'false').lower() == 'true'
        
        return jsonify({
            'success': True,
            'category': category.to_dict(include_children=include_children, include_documents=include_documents)
        })
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/', methods=['POST'])
@require_auth
def create_category():
    """Create a new category"""
    try:
        data = request.get_json()
        
        if not data or 'name' not in data:
            return jsonify({
                'success': False,
                'error': 'Category name is required'
            }), 400
        
        name = data['name'].strip()
        if not name:
            return jsonify({
                'success': False,
                'error': 'Category name cannot be empty'
            }), 400
        
        # Check if category already exists
        slug = Category.create_slug(name)
        existing = Category.query.filter_by(slug=slug).first()
        if existing:
            return jsonify({
                'success': False,
                'error': 'Category with this name already exists'
            }), 409
        
        parent_id = data.get('parent_id')
        if parent_id:
            parent = Category.query.get(parent_id)
            if not parent:
                return jsonify({
                    'success': False,
                    'error': 'Parent category not found'
                }), 404
        
        category = Category(
            name=name,
            description=data.get('description'),
            parent_id=parent_id,
            created_by=request.user.id if hasattr(request, 'user') else None,
            color=data.get('color', '#007bff')
        )
        
        db.session.add(category)
        db.session.commit()
        
        return jsonify({
            'success': True,
            'category': category.to_dict(),
            'message': 'Category created successfully'
        }), 201
        
    except Exception as e:
        db.session.rollback()
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/<int:category_id>', methods=['PUT'])
@require_auth
def update_category(category_id):
    """Update an existing category"""
    try:
        category = Category.query.get_or_404(category_id)
        data = request.get_json()
        
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        # Update fields
        if 'name' in data:
            name = data['name'].strip()
            if not name:
                return jsonify({
                    'success': False,
                    'error': 'Category name cannot be empty'
                }), 400
            
            # Check if new name conflicts with existing category
            new_slug = Category.create_slug(name)
            existing = Category.query.filter_by(slug=new_slug).first()
            if existing and existing.id != category_id:
                return jsonify({
                    'success': False,
                    'error': 'Category with this name already exists'
                }), 409
            
            category.name = name
            category.slug = new_slug
        
        if 'description' in data:
            category.description = data['description']
        
        if 'color' in data:
            category.color = data['color']
        
        if 'sort_order' in data:
            category.sort_order = data['sort_order']
        
        if 'is_active' in data:
            category.is_active = data['is_active']
        
        # Handle parent change
        if 'parent_id' in data:
            parent_id = data['parent_id']
            
            # Check for circular reference
            if parent_id and not category.can_have_parent(parent_id):
                return jsonify({
                    'success': False,
                    'error': 'Invalid parent: would create circular reference'
                }), 400
            
            category.parent_id = parent_id
        
        db.session.commit()
        
        return jsonify({
            'success': True,
            'category': category.to_dict(),
            'message': 'Category updated successfully'
        })
        
    except Exception as e:
        db.session.rollback()
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/<int:category_id>', methods=['DELETE'])
@require_auth
def delete_category(category_id):
    """Delete a category"""
    try:
        category = Category.query.get_or_404(category_id)
        
        # Check if category has children
        if category.children:
            return jsonify({
                'success': False,
                'error': 'Cannot delete category with subcategories. Please move or delete subcategories first.'
            }), 400
        
        # Check if category has documents
        if category.documents:
            return jsonify({
                'success': False,
                'error': 'Cannot delete category with documents. Please move or delete documents first.'
            }), 400
        
        db.session.delete(category)
        db.session.commit()
        
        return jsonify({
            'success': True,
            'message': 'Category deleted successfully'
        })
        
    except Exception as e:
        db.session.rollback()
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/<int:category_id>/documents', methods=['GET'])
def get_category_documents(category_id):
    """Get all documents in a category"""
    try:
        category = Category.query.get_or_404(category_id)
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        include_descendants = request.args.get('include_descendants', 'true').lower() == 'true'
        
        if include_descendants:
            descendant_ids = [cat.id for cat in category.get_all_descendants()]
            descendant_ids.append(category_id)
            query = Document.query.filter(
                Document.category_id.in_(descendant_ids),
                Document.is_public == True
            )
        else:
            query = Document.query.filter_by(category_id=category_id, is_public=True)

        query = query.order_by(Document.updated_at.desc())
        return paginate_query(
            query, page, per_page,
            serializer_func=lambda d: d.to_dict(),
            items_key='documents',
            extra_fields={'success': True, 'category': category.to_dict()}
        )
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/<int:category_id>/move', methods=['POST'])
@require_auth
def move_category(category_id):
    """Move a category to a new parent"""
    try:
        category = Category.query.get_or_404(category_id)
        data = request.get_json()
        
        if not data:
            return jsonify({
                'success': False,
                'error': 'No data provided'
            }), 400
        
        new_parent_id = data.get('parent_id')
        
        # Check for circular reference
        if new_parent_id and not category.can_have_parent(new_parent_id):
            return jsonify({
                'success': False,
                'error': 'Invalid parent: would create circular reference'
            }), 400
        
        category.parent_id = new_parent_id
        db.session.commit()
        
        return jsonify({
            'success': True,
            'category': category.to_dict(),
            'message': 'Category moved successfully'
        })
        
    except Exception as e:
        db.session.rollback()
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@categories_bp.route('/stats', methods=['GET'])
def get_category_stats():
    """Get category statistics"""
    try:
        total_categories = Category.query.count()
        active_categories = Category.query.filter_by(is_active=True).count()
        root_categories = Category.query.filter_by(parent_id=None).count()
        
        # Get categories with most documents
        top_categories = db.session.query(
            Category.id,
            Category.name,
            func.count(Document.id).label('doc_count')
        ).outerjoin(Document).group_by(Category.id, Category.name)\
         .order_by(func.count(Document.id).desc()).limit(10).all()
        
        return jsonify({
            'success': True,
            'stats': {
                'total_categories': total_categories,
                'active_categories': active_categories,
                'root_categories': root_categories,
                'top_categories': [
                    {'id': cat.id, 'name': cat.name, 'document_count': cat.doc_count}
                    for cat in top_categories
                ]
            }
        })
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500