from flask import Blueprint, request, jsonify, send_file, current_app
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db, limiter
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
# SECURITY: Maximum file size for uploads (10MB)
MAX_FILE_SIZE = 10 * 1024 * 1024

# SECURITY: Removed SVG to prevent XSS attacks via embedded JavaScript
ALLOWED_EXTENSIONS = {
    'txt', 'pdf', 'png', 'jpg', 'jpeg', 'gif', 'doc', 'docx',
    'mp4', 'avi', 'mov', 'mp3', 'wav', 'ogg', 'webp',
    'md', 'zip', 'tar', 'gz', 'json', 'xml', 'csv', 'xlsx'
}

# SECURITY: Magic bytes signatures for file type validation
MAGIC_SIGNATURES = {
    b'\x89PNG\r\n\x1a\n': {'png'},
    b'\xff\xd8\xff': {'jpg', 'jpeg'},
    b'GIF87a': {'gif'},
    b'GIF89a': {'gif'},
    b'%PDF': {'pdf'},
    b'PK\x03\x04': {'zip', 'docx', 'xlsx'},  # ZIP-based formats
    b'PK\x05\x06': {'zip', 'docx', 'xlsx'},
    b'\x1f\x8b': {'gz'},
    b'RIFF': {'webp', 'wav', 'avi'},
    b'\x00\x00\x00\x1c': {'mp4', 'mov'},
    b'\x00\x00\x00\x20': {'mp4', 'mov'},
    b'ftyp': {'mp4', 'mov'},
    b'ID3': {'mp3'},
    b'\xff\xfb': {'mp3'},
    b'\xff\xfa': {'mp3'},
    b'OggS': {'ogg'},
}


def allowed_file(filename):
    return '.' in filename and \
           filename.rsplit('.', 1)[1].lower() in ALLOWED_EXTENSIONS


def validate_magic_bytes(file_content: bytes, extension: str) -> tuple[bool, str]:
    """SECURITY: Validate file content matches claimed extension using magic bytes.

    Returns (is_valid, error_message)
    """
    extension = extension.lower().lstrip('.')

    # Text-based formats don't have magic bytes - check for binary content
    text_extensions = {'txt', 'md', 'json', 'xml', 'csv'}
    if extension in text_extensions:
        # Check for null bytes which indicate binary content
        if b'\x00' in file_content[:1024]:
            return False, "Binary content detected in text file"
        return True, ""

    # Check magic bytes for binary formats
    for signature, allowed_exts in MAGIC_SIGNATURES.items():
        if file_content.startswith(signature):
            if extension in allowed_exts:
                return True, ""
            else:
                return False, f"File content does not match .{extension} extension"

    # Special case for MP4/MOV which have ftyp at offset 4
    if len(file_content) > 8 and file_content[4:8] == b'ftyp':
        if extension in {'mp4', 'mov'}:
            return True, ""

    # For doc/docx, check for OLE or ZIP
    if extension in {'doc', 'docx'}:
        if file_content.startswith(b'\xd0\xcf\x11\xe0'):  # OLE
            return True, ""
        if file_content.startswith(b'PK\x03\x04'):  # ZIP
            return True, ""

    # TAR files have various signatures
    if extension == 'tar':
        if len(file_content) >= 262 and file_content[257:262] == b'ustar':
            return True, ""
        return True, ""  # Allow uncompressed tar

    # If no signature matched but extension is in allowed list, warn but allow
    # This handles edge cases and prevents breaking legitimate uploads
    logger.warning(f"SECURITY: No magic signature match for .{extension}, allowing cautiously")
    return True, ""


def _log_file_operation(operation: str, user_id: int, attachment_id: int = None,
                        details: dict = None) -> None:
    """SECURITY: Audit log file operations for compliance."""
    from flask import request
    import json

    log_entry = {
        'operation': operation,
        'user_id': user_id,
        'attachment_id': attachment_id,
        'ip_address': request.remote_addr if request else 'unknown',
        'details': details or {}
    }
    logger.info(f"AUDIT_FILE: {json.dumps(log_entry)}")

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
    """Create thumbnail for image with security protections"""
    try:
        # SECURITY: Set decompression bomb limit
        Image.MAX_IMAGE_PIXELS = 25000000  # ~5000x5000 limit

        with Image.open(image_path) as img:
            # SECURITY: Verify image dimensions before processing
            width, height = img.size
            if width * height > Image.MAX_IMAGE_PIXELS:
                logger.warning("Image too large for thumbnail: %sx%s", width, height)
                return False

            img.thumbnail(size, Image.Resampling.LANCZOS)
            img.save(thumbnail_path, optimize=True, quality=85)
        return True
    except Image.DecompressionBombError:
        logger.warning("Decompression bomb detected: %s", image_path)
        return False
    except Exception as e:
        logger.debug("Thumbnail creation failed for %s: %s", image_path, e)
        return False

@attachments_bp.route('/attachments/upload', methods=['POST'])
@limiter.limit("20 per hour")
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

        # SECURITY: Validate magic bytes to prevent file type spoofing
        extension = file.filename.rsplit('.', 1)[1].lower() if '.' in file.filename else ''
        file_header = file.read(1024)
        file.seek(0)

        is_valid, error_msg = validate_magic_bytes(file_header, extension)
        if not is_valid:
            logger.warning(f"SECURITY: Magic bytes validation failed for {file.filename}: {error_msg}")
            return jsonify({'error': 'File content does not match file type'}), 400
        
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
            # Remove the newly uploaded file
            os.remove(file_path)
            # SECURITY: Only return existing attachment if owned by current user (prevent IDOR)
            if existing.uploaded_by == current_user_id:
                return jsonify({
                    'message': 'File already exists',
                    'attachment': existing.to_dict(),
                    'duplicate': True
                })
            else:
                # SECURITY: Don't reveal that file exists in system - return generic success
                # This prevents attackers from probing for existence of specific files
                logger.info(f"Duplicate file upload blocked (different owner): hash={file_hash[:16]}...")
                return jsonify({
                    'message': 'File uploaded successfully',
                    'duplicate': False
                }), 201
        
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

        # SECURITY: Audit log successful upload
        _log_file_operation('upload', current_user_id, attachment.id, {
            'filename': original_filename,
            'size': file_size,
            'mime_type': mime_type
        })

        return jsonify({
            'message': 'File uploaded successfully',
            'attachment': attachment.to_dict()
        }), 201

    except Exception as e:
        db.session.rollback()
        logger.error("Error uploading file: %s", e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments/<int:attachment_id>/download', methods=['GET'])
@limiter.limit("100 per hour")
@jwt_required(optional=True)  # SECURITY: Explicit optional auth for public files
def download_file(attachment_id):
    """Download a file"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_current_user_id()
        
        if not attachment.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        if not os.path.exists(attachment.file_path):
            return jsonify({'error': 'File not found on disk'}), 404

        # SECURITY: Audit log download
        _log_file_operation('download', current_user_id or 0, attachment_id)

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
@limiter.limit("200 per hour")
@jwt_required(optional=True)  # SECURITY: Explicit optional auth for public files
def preview_file(attachment_id):
    """Preview a file (for images, videos, etc.)"""
    # MIME types that can execute scripts - force download to prevent XSS
    DANGEROUS_MIME_TYPES = ['text/html', 'image/svg+xml', 'application/xhtml+xml', 'text/xml']

    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_current_user_id()

        if not attachment.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403

        if not os.path.exists(attachment.file_path):
            return jsonify({'error': 'File not found on disk'}), 404

        # Force download for potentially dangerous file types to prevent stored XSS
        if attachment.mime_type in DANGEROUS_MIME_TYPES:
            return send_file(
                attachment.file_path,
                as_attachment=True,
                download_name=attachment.original_filename
            )

        # For images, try to serve thumbnail if it exists
        if attachment.is_image():
            # SECURITY: Use secure_filename for path construction
            from werkzeug.utils import secure_filename
            safe_filename = secure_filename(attachment.filename)
            base_dir = os.path.abspath(os.path.dirname(attachment.file_path))
            upload_root = os.path.abspath(UPLOAD_FOLDER)

            # SECURITY: Verify paths are within upload root
            if base_dir.startswith(upload_root):
                thumbnail_path = os.path.join(base_dir, f"thumb_{safe_filename}")
                if os.path.abspath(thumbnail_path).startswith(upload_root):
                    if os.path.exists(thumbnail_path):
                        response = current_app.make_response(
                            send_file(thumbnail_path, mimetype=attachment.mime_type)
                        )
                        response.headers['X-Content-Type-Options'] = 'nosniff'
                        return response

        # Serve original file for preview with security headers
        response = current_app.make_response(
            send_file(attachment.file_path, mimetype=attachment.mime_type)
        )
        response.headers['X-Content-Type-Options'] = 'nosniff'
        return response
        
    except Exception as e:
        logger.error("Error previewing file %s: %s", attachment_id, e)
        return jsonify({'error': 'Internal server error'}), 500

@attachments_bp.route('/attachments', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Add rate limiting
@jwt_required()
def list_attachments():
    """List user's attachments"""
    try:
        current_user_id = get_jwt_identity()
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        # SECURITY: Enforce pagination bounds
        page = max(1, page)
        per_page = max(1, min(per_page, 100))
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
@limiter.limit("100 per hour")
@jwt_required(optional=True)  # SECURITY: Explicit optional auth for public files
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
@limiter.limit("30 per hour")  # SECURITY: Add rate limiting
@jwt_required()
def delete_attachment(attachment_id):
    """Delete an attachment"""
    try:
        attachment = Attachment.query.get_or_404(attachment_id)
        current_user_id = get_jwt_identity()
        
        if not attachment.can_delete(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        # SECURITY: Audit log deletion before removing file
        _log_file_operation('delete', current_user_id, attachment_id, {
            'filename': attachment.original_filename,
            'size': attachment.file_size
        })

        # Delete physical file
        attachment.delete_file()

        # Delete thumbnail if exists with path validation
        if attachment.is_image():
            # SECURITY: Use secure_filename for thumbnail path construction
            from werkzeug.utils import secure_filename
            safe_filename = secure_filename(attachment.filename)
            base_dir = os.path.abspath(os.path.dirname(attachment.file_path))
            upload_root = os.path.abspath(UPLOAD_FOLDER)

            # SECURITY: Verify base_dir is within upload root
            if base_dir.startswith(upload_root):
                thumbnail_path = os.path.join(base_dir, f"thumb_{safe_filename}")
                # SECURITY: Verify final path is within upload root
                if os.path.abspath(thumbnail_path).startswith(upload_root):
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
@limiter.limit("60 per minute")
@jwt_required(optional=True)  # SECURITY: Add explicit JWT handling for consistent auth behavior
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