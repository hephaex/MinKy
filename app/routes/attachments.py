from flask import Blueprint, request, jsonify, send_file
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db
from app.models.attachment import Attachment
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query
import os
import mimetypes
import logging
from PIL import Image

logger = logging.getLogger(__name__)

attachments_bp = Blueprint('attachments', __name__)

# Configuration
UPLOAD_FOLDER = 'uploads'
MAX_FILE_SIZE = 50 * 1024 * 1024  # 50MB
ALLOWED_EXTENSIONS = {
    'txt', 'pdf', 'png', 'jpg', 'jpeg', 'gif', 'doc', 'docx',
    'mp4', 'avi', 'mov', 'mp3', 'wav', 'ogg', 'svg', 'webp',
    'md', 'zip', 'tar', 'gz', 'json', 'xml', 'csv', 'xlsx'
}

def allowed_file(filename):
    return '.' in filename and \
           filename.rsplit('.', 1)[1].lower() in ALLOWED_EXTENSIONS

def ensure_upload_folder():
    """Ensure upload folder exists"""
    if not os.path.exists(UPLOAD_FOLDER):
        os.makedirs(UPLOAD_FOLDER)
    
    # Create subdirectories
    subdirs = ['images', 'documents', 'videos', 'audio', 'other']
    for subdir in subdirs:
        path = os.path.join(UPLOAD_FOLDER, subdir)
        if not os.path.exists(path):
            os.makedirs(path)

def get_upload_path(file_type):
    """Get upload path based on file type"""
    ensure_upload_folder()
    return os.path.join(UPLOAD_FOLDER, file_type)

def create_thumbnail(image_path, thumbnail_path, size=(300, 300)):
    """Create thumbnail for image"""
    try:
        with Image.open(image_path) as img:
            img.thumbnail(size, Image.Resampling.LANCZOS)
            img.save(thumbnail_path, optimize=True, quality=85)
        return True
    except Exception:
        return False

@attachments_bp.route('/attachments/upload', methods=['POST'])
@jwt_required()
def upload_file():
    """Upload a file"""
    try:
        current_user_id = get_jwt_identity()
        
        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400
        
        file = request.files['file']
        if file.filename == '':
            return jsonify({'error': 'No file selected'}), 400
        
        if not allowed_file(file.filename):
            return jsonify({'error': 'File type not allowed'}), 400
        
        # Check file size
        file.seek(0, os.SEEK_END)
        file_size = file.tell()
        file.seek(0)
        
        if file_size > MAX_FILE_SIZE:
            return jsonify({'error': f'File too large. Maximum size is {MAX_FILE_SIZE // (1024*1024)}MB'}), 400
        
        # Generate secure filename
        original_filename = file.filename
        filename = Attachment.generate_filename(original_filename)
        
        # Determine file type and upload path
        mime_type = mimetypes.guess_type(original_filename)[0] or 'application/octet-stream'
        
        # Create temporary attachment to determine file type
        temp_attachment = Attachment(
            filename=filename,
            original_filename=original_filename,
            file_path='',
            file_size=file_size,
            mime_type=mime_type,
            file_hash='',
            uploaded_by=current_user_id
        )
        
        file_type = temp_attachment.get_file_type()
        upload_path = get_upload_path(file_type)
        file_path = os.path.join(upload_path, filename)
        
        # Save file
        file.save(file_path)
        
        # Calculate file hash
        file_hash = Attachment.calculate_file_hash(file_path)
        
        # Check for duplicate files
        existing = Attachment.query.filter_by(file_hash=file_hash).first()
        if existing:
            # Remove the newly uploaded file and return existing
            os.remove(file_path)
            return jsonify({
                'message': 'File already exists',
                'attachment': existing.to_dict(),
                'duplicate': True
            })
        
        # Create thumbnail for images
        thumbnail_path = None
        if temp_attachment.is_image():
            thumbnail_filename = f"thumb_{filename}"
            thumbnail_path = os.path.join(upload_path, thumbnail_filename)
            create_thumbnail(file_path, thumbnail_path)
        
        # Get document_id if provided
        document_id = request.form.get('document_id', type=int)
        if document_id:
            document = db.session.get(Document, document_id)
            if not document or not document.can_edit(current_user_id):
                os.remove(file_path)
                if thumbnail_path and os.path.exists(thumbnail_path):
                    os.remove(thumbnail_path)
                return jsonify({'error': 'Cannot attach to this document'}), 403
        
        # Create attachment record
        attachment = Attachment(
            filename=filename,
            original_filename=original_filename,
            file_path=file_path,
            file_size=file_size,
            mime_type=mime_type,
            file_hash=file_hash,
            uploaded_by=current_user_id,
            document_id=document_id,
            is_public=request.form.get('is_public', 'true').lower() == 'true'
        )
        
        db.session.add(attachment)
        db.session.commit()
        
        return jsonify({
            'message': 'File uploaded successfully',
            'attachment': attachment.to_dict()
        }), 201
        
    except Exception as e:
        db.session.rollback()
        logger.error("Error uploading file: %s", e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/<int:attachment_id>/download', methods=['GET'])
def download_file(attachment_id):
    """Download a file"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_current_user_id()
        
        if not attachment.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        if not os.path.exists(attachment.file_path):
            return jsonify({'error': 'File not found on disk'}), 404
        
        return send_file(
            attachment.file_path,
            as_attachment=True,
            download_name=attachment.original_filename,
            mimetype=attachment.mime_type
        )
        
    except Exception as e:
        logger.error("Error downloading file %s: %s", attachment_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/<int:attachment_id>/preview', methods=['GET'])
def preview_file(attachment_id):
    """Preview a file (for images, videos, etc.)"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_current_user_id()
        
        if not attachment.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        if not os.path.exists(attachment.file_path):
            return jsonify({'error': 'File not found on disk'}), 404
        
        # For images, try to serve thumbnail if it exists
        if attachment.is_image():
            thumbnail_path = os.path.join(
                os.path.dirname(attachment.file_path),
                f"thumb_{attachment.filename}"
            )
            if os.path.exists(thumbnail_path):
                return send_file(thumbnail_path, mimetype=attachment.mime_type)
        
        # Serve original file for preview
        return send_file(attachment.file_path, mimetype=attachment.mime_type)
        
    except Exception as e:
        logger.error("Error previewing file %s: %s", attachment_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments', methods=['GET'])
@jwt_required()
def list_attachments():
    """List user's attachments"""
    try:
        current_user_id = get_jwt_identity()
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        file_type = request.args.get('file_type', '')
        document_id = request.args.get('document_id', type=int)
        
        query = Attachment.query.filter_by(uploaded_by=current_user_id)
        
        if file_type:
            # Filter by file type category
            if file_type == 'image':
                query = query.filter(Attachment.mime_type.like('image/%'))
            elif file_type == 'document':
                query = query.filter(Attachment.mime_type.in_([
                    'application/pdf', 'application/msword',
                    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
                    'text/plain', 'application/rtf'
                ]))
            elif file_type == 'video':
                query = query.filter(Attachment.mime_type.like('video/%'))
            elif file_type == 'audio':
                query = query.filter(Attachment.mime_type.like('audio/%'))
        
        if document_id:
            query = query.filter_by(document_id=document_id)

        query = query.order_by(Attachment.created_at.desc())
        return paginate_query(
            query, page, per_page,
            serializer_func=lambda a: a.to_dict(),
            items_key='attachments'
        )
        
    except Exception as e:
        logger.error("Error listing attachments: %s", e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/<int:attachment_id>', methods=['GET'])
def get_attachment(attachment_id):
    """Get attachment details"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_current_user_id()
        
        if not attachment.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        return jsonify(attachment.to_dict())
        
    except Exception as e:
        logger.error("Error getting attachment %s: %s", attachment_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/<int:attachment_id>', methods=['DELETE'])
@jwt_required()
def delete_attachment(attachment_id):
    """Delete an attachment"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_jwt_identity()
        
        if not attachment.can_delete(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        # Delete physical file
        attachment.delete_file()
        
        # Delete thumbnail if exists
        if attachment.is_image():
            thumbnail_path = os.path.join(
                os.path.dirname(attachment.file_path),
                f"thumb_{attachment.filename}"
            )
            if os.path.exists(thumbnail_path):
                os.remove(thumbnail_path)
        
        # Delete database record
        db.session.delete(attachment)
        db.session.commit()
        
        return jsonify({'message': 'Attachment deleted successfully'})
        
    except Exception as e:
        db.session.rollback()
        logger.error("Error deleting attachment %s: %s", attachment_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/documents/<int:document_id>/attachments', methods=['GET'])
def get_document_attachments(document_id):
    """Get attachments for a specific document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        attachments = Attachment.query.filter_by(document_id=document_id)\
            .order_by(Attachment.created_at.desc()).all()
        
        return jsonify({
            'attachments': [attachment.to_dict() for attachment in attachments],
            'document': {
                'id': document.id,
                'title': document.title
            }
        })
        
    except Exception as e:
        logger.error("Error getting document attachments for document %s: %s", document_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/stats', methods=['GET'])
@jwt_required()
def get_attachment_stats():
    """Get attachment statistics"""
    try:
        current_user_id = get_jwt_identity()
        
        user_stats = db.session.query(
            db.func.count(Attachment.id).label('total_files'),
            db.func.sum(Attachment.file_size).label('total_size')
        ).filter_by(uploaded_by=current_user_id).first()
        
        type_stats = db.session.query(
            Attachment.mime_type,
            db.func.count(Attachment.id).label('count'),
            db.func.sum(Attachment.file_size).label('total_size')
        ).filter_by(uploaded_by=current_user_id)\
         .group_by(Attachment.mime_type).all()
        
        return jsonify({
            'user_stats': {
                'total_files': user_stats.total_files or 0,
                'total_size': user_stats.total_size or 0
            },
            'by_type': [
                {
                    'mime_type': stat.mime_type,
                    'count': stat.count,
                    'total_size': stat.total_size
                }
                for stat in type_stats
            ]
        })
        
    except Exception as e:
        logger.error("Error getting attachment stats: %s", e)
        return jsonify({'error': 'Internal server error'}), 500