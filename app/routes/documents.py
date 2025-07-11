from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity, jwt_required
from app import db
from app.models.document import Document
from app.models.user import User
from sqlalchemy import or_
import bleach
import os
import re
from datetime import datetime

documents_bp = Blueprint('documents', __name__)

def get_current_user_id():
    try:
        return get_jwt_identity()
    except:
        return None

@documents_bp.route('/documents', methods=['POST'])
@jwt_required(optional=True)
def create_document():
    try:
        data = request.get_json()
        
        if not data or 'title' not in data or 'markdown_content' not in data:
            return jsonify({'error': 'Title and markdown_content are required'}), 400
        
        current_user_id = get_current_user_id()
        
        # Sanitize input to prevent XSS
        title = bleach.clean(data['title'].strip())
        author = bleach.clean(data.get('author', '').strip()) if data.get('author') else None
        is_public = data.get('is_public', True)
        tags = data.get('tags', [])
        
        if not title:
            return jsonify({'error': 'Title cannot be empty'}), 400
        
        document = Document(
            title=title,
            markdown_content=data['markdown_content'],  # Markdown content is not sanitized as it's rendered safely
            author=author,
            user_id=current_user_id,
            is_public=is_public
        )
        
        # Add tags if provided
        if tags:
            document.add_tags(tags)
        
        db.session.add(document)
        db.session.commit()
        
        return jsonify(document.to_dict()), 201
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents', methods=['GET'])
@jwt_required(optional=True)
def list_documents():
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        search = request.args.get('search', '')
        include_private = request.args.get('include_private', 'false').lower() == 'true'
        
        current_user_id = get_current_user_id()
        
        tags_filter = request.args.getlist('tags')  # Support multiple tags
        
        pagination = Document.search_documents(
            search, page, per_page, 
            user_id=current_user_id, 
            include_private=include_private and current_user_id is not None,
            tags=tags_filter if tags_filter else None
        )
        documents = [doc.to_dict() for doc in pagination.items]
        
        return jsonify({
            'documents': documents,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            },
            'search_query': search,
            'include_private': include_private and current_user_id is not None
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def get_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        return jsonify(document.to_dict())
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['PUT'])
@jwt_required(optional=True)
def update_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        data = request.get_json()
        
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
        # Sanitize input
        title = bleach.clean(data.get('title', '').strip()) if data.get('title') else None
        author = bleach.clean(data.get('author', '').strip()) if data.get('author') else None
        
        document.update_content(
            title=title,
            markdown_content=data.get('markdown_content'),
            author=author,
            change_summary=data.get('change_summary'),
            updated_by=current_user_id
        )
        
        # Update visibility if provided and user owns the document
        if 'is_public' in data and document.user_id == current_user_id:
            document.is_public = bool(data['is_public'])
        
        # Update tags if provided
        if 'tags' in data:
            # Clear existing tags and add new ones
            document.tags.clear()
            if data['tags']:
                document.add_tags(data['tags'])
        
        db.session.commit()
        
        return jsonify(document.to_dict())
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['DELETE'])
@jwt_required(optional=True)
def delete_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        db.session.delete(document)
        db.session.commit()
        
        return jsonify({'message': 'Document deleted successfully'})
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

def extract_markdown_metadata(content):
    """Extract metadata from markdown frontmatter"""
    frontmatter_pattern = r'^---\s*\n(.*?)\n---\s*\n'
    match = re.match(frontmatter_pattern, content, re.DOTALL)
    
    if match:
        yaml_content = match.group(1)
        # Remove frontmatter from content
        content = re.sub(frontmatter_pattern, '', content, flags=re.DOTALL)
        
        # Parse simple YAML-like metadata
        metadata = {}
        for line in yaml_content.split('\n'):
            if ':' in line:
                key, value = line.split(':', 1)
                metadata[key.strip()] = value.strip().strip('"\'')
        
        return metadata, content
    
    return {}, content

def validate_markdown_file(file):
    """Validate uploaded markdown file"""
    if not file or not file.filename:
        return False, "No file provided"
    
    # Check file extension
    if not file.filename.lower().endswith('.md'):
        return False, "File must be a markdown (.md) file"
    
    # Check file size (max 10MB)
    file.seek(0, os.SEEK_END)
    file_size = file.tell()
    file.seek(0)
    
    if file_size > 10 * 1024 * 1024:  # 10MB
        return False, "File size must be less than 10MB"
    
    if file_size == 0:
        return False, "File cannot be empty"
    
    return True, None

@documents_bp.route('/documents/upload', methods=['POST'])
@jwt_required(optional=True)
def upload_markdown_file():
    """Upload a markdown file and create a document"""
    try:
        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400
        
        file = request.files['file']
        current_user_id = get_current_user_id()
        
        # Validate file
        is_valid, error_message = validate_markdown_file(file)
        if not is_valid:
            return jsonify({'error': error_message}), 400
        
        # Read file content
        try:
            content = file.read().decode('utf-8')
        except UnicodeDecodeError:
            return jsonify({'error': 'File must be UTF-8 encoded'}), 400
        
        # Extract metadata from frontmatter
        metadata, markdown_content = extract_markdown_metadata(content)
        
        # Get title from metadata or filename
        title = metadata.get('title', '')
        if not title:
            # Use filename without extension as title
            title = os.path.splitext(file.filename)[0]
        
        # Clean and validate title
        title = bleach.clean(title.strip())
        if not title:
            return jsonify({'error': 'Document title cannot be empty'}), 400
        
        # Get other metadata
        author = metadata.get('author', '')
        if author:
            author = bleach.clean(author.strip())
        
        # Get tags from metadata
        tags = []
        if 'tags' in metadata:
            tags_str = metadata['tags']
            if tags_str:
                # Parse tags (assume comma-separated or array-like format)
                tags_str = tags_str.strip('[]')
                tags = [tag.strip().strip('"\',') for tag in tags_str.split(',') if tag.strip()]
        
        # Get visibility setting
        is_public = True
        if 'public' in metadata:
            is_public = metadata['public'].lower() in ['true', 'yes', '1']
        elif 'private' in metadata:
            is_public = metadata['private'].lower() not in ['true', 'yes', '1']
        
        # Create document
        document = Document(
            title=title,
            markdown_content=markdown_content,
            author=author if author else None,
            user_id=current_user_id,
            is_public=is_public
        )
        
        # Add tags if provided
        if tags:
            document.add_tags(tags)
        
        # Store additional metadata
        if metadata:
            # Remove processed metadata
            stored_metadata = {k: v for k, v in metadata.items() 
                             if k not in ['title', 'author', 'tags', 'public', 'private']}
            if stored_metadata:
                document.document_metadata = stored_metadata
        
        db.session.add(document)
        db.session.commit()
        
        return jsonify({
            'message': 'File uploaded successfully',
            'document': document.to_dict(),
            'metadata_found': bool(metadata)
        }), 201
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'Upload failed: {str(e)}'}), 500