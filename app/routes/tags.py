from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db
from app.models.tag import Tag
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, get_or_404
from app.utils.auto_tag import detect_auto_tags, merge_tags
import bleach

tags_bp = Blueprint('tags', __name__)

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

@tags_bp.route('/tags/statistics', methods=['GET'])
def get_tags_statistics():
    """Get comprehensive tag statistics"""
    try:
        
        # Total tags
        total_tags = Tag.query.count()
        
        # Most popular tags
        popular_tags_data = Tag.get_popular_tags(limit=10)
        popular_list = [{'name': tag.name, 'usage_count': count, 'color': tag.color} for tag, count in popular_tags_data]
        
        # Recently created tags
        recent_tags = Tag.query.order_by(Tag.created_at.desc()).limit(5).all()
        recent_list = [{'name': tag.name, 'created_at': tag.created_at.isoformat(), 'color': tag.color} for tag in recent_tags]
        
        # Tag usage distribution - simplified approach
        all_tags = Tag.query.all()
        usage_distribution = {'unused': 0, 'low': 0, 'medium': 0, 'high': 0}
        
        for tag in all_tags:
            doc_count = tag.get_document_count()
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
        
        return jsonify({
            'total_tags': total_tags,
            'auto_generated_tags': auto_generated_count,
            'manual_tags': total_tags - auto_generated_count,
            'popular_tags': popular_list,
            'recent_tags': recent_list,
            'usage_distribution': usage_distribution
        })
        
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/auto-generate', methods=['POST'])
@jwt_required(optional=True)
def generate_auto_tags():
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
                return jsonify({'error': 'Document not found'}), 404
            
            # Check access permissions
            if not document.can_view(current_user_id):
                return jsonify({'error': 'Access denied'}), 403
            
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
        
        print(f"[AUTO_TAG_GENERATION] Processing {len(documents)} documents")
        
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
                print(f"[AUTO_TAG_GENERATION] Error processing document {doc.id}: {e}")
        
        if not dry_run:
            db.session.commit()
            print("[AUTO_TAG_GENERATION] Committed changes to database")
        
        return jsonify({
            'success': True,
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
        print(f"[AUTO_TAG_GENERATION] Fatal error: {e}")
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/tagless-documents', methods=['GET'])
@jwt_required(optional=True)
def get_tagless_documents():
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
        return jsonify({'error': str(e)}), 500

@tags_bp.route('/tags/preview-auto-tags/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def preview_auto_tags(document_id):
    """Preview what auto tags would be generated for a specific document"""
    try:
        current_user_id = get_current_user_id()
        
        document = get_or_404(Document, document_id)
        
        # Check access permissions
        if not document.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        # Detect auto tags
        content = document.markdown_content or document.content or ''
        auto_tags = detect_auto_tags(content)
        
        # Get existing tags
        existing_tags = [tag.name for tag in document.tags]
        
        # Merge tags to see final result
        merged_tags = merge_tags(existing_tags, auto_tags)
        
        return jsonify({
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
        return jsonify({'error': str(e)}), 500