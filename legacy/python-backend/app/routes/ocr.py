"""
OCR API Routes
Provides endpoints for optical character recognition on uploaded files
"""

from flask import Blueprint, request, jsonify, Response, current_app
from flask_jwt_extended import jwt_required
from werkzeug.utils import secure_filename
from app.services.ocr_service import ocr_service
from app.models.document import Document
from app.models.attachment import Attachment
from app.utils.auth import get_current_user_id, get_optional_user_id
from app import db, limiter
import logging
import os
from datetime import datetime, timezone

logger = logging.getLogger(__name__)

ocr_bp = Blueprint('ocr', __name__)

@ocr_bp.route('/ocr/extract', methods=['POST'])
@limiter.limit("20 per hour")
@jwt_required(optional=True)
def extract_text() -> Response | tuple[Response, int]:
    """
    Extract text from uploaded image or PDF file
    """
    try:
        if not ocr_service.is_available():
            return jsonify({
                'success': False,
                'error': 'OCR service is not available. Please install Tesseract or configure cloud OCR.'
            }), 503
        
        # Check if file was uploaded
        if 'file' not in request.files:
            return jsonify({
                'success': False,
                'error': 'No file uploaded'
            }), 400
        
        file = request.files['file']
        if file.filename == '':
            return jsonify({
                'success': False,
                'error': 'No file selected'
            }), 400
        
        # SECURITY: Validate language parameter against whitelist
        ALLOWED_LANGUAGES = frozenset({
            'eng', 'kor', 'jpn', 'chi_sim', 'chi_tra',
            'fra', 'deu', 'spa', 'ita', 'por', 'rus'
        })
        language = request.form.get('language', 'eng')
        if language not in ALLOWED_LANGUAGES:
            language = 'eng'  # Default to English if invalid

        # Validate file type
        allowed_extensions = {'.pdf', '.png', '.jpg', '.jpeg', '.tiff', '.bmp', '.gif'}
        file_ext = os.path.splitext(secure_filename(file.filename))[1].lower()
        
        if file_ext not in allowed_extensions:
            return jsonify({
                'success': False,
                'error': f'Unsupported file type: {file_ext}. Supported: {", ".join(allowed_extensions)}'
            }), 400
        
        # Read file data
        file_data = file.read()
        
        # Check file size (limit to 10MB)
        if len(file_data) > 10 * 1024 * 1024:
            return jsonify({
                'success': False,
                'error': 'File too large. Maximum size is 10MB.'
            }), 400
        
        # Process file with OCR
        result = ocr_service.process_uploaded_file(
            file_data=file_data,
            filename=file.filename,
            language=language
        )
        
        # Log OCR activity
        user_id = get_optional_user_id()
        
        logger.info(f"OCR processing: user={user_id}, file={file.filename}, "
                   f"method={result.get('method')}, success={result.get('success')}")
        
        return jsonify(result)
        
    except Exception as e:
        logger.error(f"OCR extraction error: {e}")
        return jsonify({
            'success': False,
            'error': 'Internal server error during OCR processing'
        }), 500

@ocr_bp.route('/ocr/languages', methods=['GET'])
def get_supported_languages() -> Response | tuple[Response, int]:
    """
    Get list of supported OCR languages
    """
    try:
        languages = ocr_service.get_supported_languages()
        
        # Language code to name mapping
        language_names = {
            'eng': 'English',
            'kor': 'Korean',
            'jpn': 'Japanese',
            'chi_sim': 'Chinese Simplified',
            'chi_tra': 'Chinese Traditional',
            'fra': 'French',
            'deu': 'German',
            'spa': 'Spanish',
            'ita': 'Italian',
            'por': 'Portuguese',
            'rus': 'Russian',
            'ara': 'Arabic',
            'hin': 'Hindi',
            'tha': 'Thai',
            'vie': 'Vietnamese'
        }
        
        language_list = []
        for lang_code in sorted(languages):
            language_list.append({
                'code': lang_code,
                'name': language_names.get(lang_code, lang_code.title())
            })
        
        return jsonify({
            'success': True,
            'languages': language_list,
            'available_methods': {
                'tesseract': ocr_service.tesseract_available,
                'cloud_ocr': ocr_service._check_cloud_ocr(),
                'pdf_tools': ocr_service.pdf_tools_available
            }
        })
        
    except Exception as e:
        logger.error(f"Error getting OCR languages: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to get supported languages'
        }), 500

@ocr_bp.route('/ocr/status', methods=['GET'])
def get_ocr_status() -> Response | tuple[Response, int]:
    """
    Get OCR service status and capabilities
    """
    try:
        status = {
            'available': ocr_service.is_available(),
            'tesseract': ocr_service.tesseract_available,
            'pdf_tools': ocr_service.pdf_tools_available,
            'cloud_ocr': ocr_service._check_cloud_ocr(),
            'supported_formats': ['.pdf', '.png', '.jpg', '.jpeg', '.tiff', '.bmp', '.gif'],
            'max_file_size': '10MB',
            'features': {
                'image_ocr': ocr_service.tesseract_available or ocr_service._check_cloud_ocr(),
                'pdf_ocr': ocr_service.pdf_tools_available,
                'multi_language': True,
                'confidence_scores': True
            }
        }
        
        return jsonify({
            'success': True,
            'status': status
        })
        
    except Exception as e:
        logger.error(f"Error getting OCR status: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to get OCR status'
        }), 500

@ocr_bp.route('/ocr/extract-and-create', methods=['POST'])
@limiter.limit("10 per hour")
@jwt_required()
def extract_and_create_document() -> Response | tuple[Response, int]:
    """
    Extract text from file and create a new document
    """
    try:
        if not ocr_service.is_available():
            return jsonify({
                'success': False,
                'error': 'OCR service is not available'
            }), 503

        # Check if file was uploaded
        if 'file' not in request.files:
            return jsonify({
                'success': False,
                'error': 'No file uploaded'
            }), 400

        file = request.files['file']
        if file.filename == '':
            return jsonify({
                'success': False,
                'error': 'No file selected'
            }), 400

        # SECURITY: Validate file type BEFORE reading content
        allowed_extensions = {'.pdf', '.png', '.jpg', '.jpeg', '.tiff', '.bmp', '.gif'}
        file_ext = os.path.splitext(secure_filename(file.filename))[1].lower()

        if file_ext not in allowed_extensions:
            return jsonify({
                'success': False,
                'error': f'Unsupported file type: {file_ext}. Supported: {", ".join(allowed_extensions)}'
            }), 400

        # Get parameters with validation
        # SECURITY: Validate language parameter against whitelist
        ALLOWED_LANGUAGES = frozenset({
            'eng', 'kor', 'jpn', 'chi_sim', 'chi_tra',
            'fra', 'deu', 'spa', 'ita', 'por', 'rus'
        })
        language = request.form.get('language', 'eng')
        if language not in ALLOWED_LANGUAGES:
            language = 'eng'  # Default to English if invalid

        title = request.form.get('title', '')
        author = request.form.get('author', '')
        is_public = request.form.get('is_public', 'true').lower() == 'true'

        # Read file data
        file_data = file.read()

        # SECURITY: Check file size AFTER reading (limit to 10MB)
        MAX_FILE_SIZE = 10 * 1024 * 1024  # 10MB
        if len(file_data) > MAX_FILE_SIZE:
            return jsonify({
                'success': False,
                'error': 'File too large. Maximum size is 10MB.'
            }), 400

        # Process file with OCR
        ocr_result = ocr_service.process_uploaded_file(
            file_data=file_data,
            filename=file.filename,
            language=language
        )
        
        if not ocr_result['success']:
            return jsonify({
                'success': False,
                'error': f"OCR failed: {ocr_result.get('error', 'Unknown error')}",
                'ocr_result': ocr_result
            }), 400
        
        # Generate title if not provided
        if not title:
            title = f"OCR Extract from {file.filename}"
        
        # Create markdown content with OCR metadata
        markdown_content = f"""# {title}

*Extracted from: {file.filename}*  
*OCR Method: {ocr_result.get('method', 'unknown')}*  
*Confidence: {ocr_result.get('confidence', 0)}%*  
*Language: {language}*  
*Extracted: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S')} UTC*

---

{ocr_result['text']}
"""
        
        # Create document
        user_id = get_current_user_id()
        document = Document(
            title=title,
            markdown_content=markdown_content,
            author=author or None,
            user_id=user_id,
            is_public=is_public,
            document_metadata={
                'ocr_result': ocr_result,
                'source_file': file.filename,
                'extraction_date': datetime.now(timezone.utc).isoformat()
            }
        )
        
        db.session.add(document)
        db.session.commit()
        
        logger.info(f"Created document from OCR: user={user_id}, document_id={document.id}, "
                   f"file={file.filename}, confidence={ocr_result.get('confidence')}")
        
        return jsonify({
            'success': True,
            'document': document.to_dict(),
            'ocr_result': ocr_result,
            'message': 'Document created successfully from OCR extraction'
        }), 201
        
    except Exception as e:
        logger.error(f"Error in extract-and-create: {e}")
        db.session.rollback()
        return jsonify({
            'success': False,
            'error': 'Failed to create document from OCR'
        }), 500

@ocr_bp.route('/ocr/extract-attachment/<int:attachment_id>', methods=['POST'])
@limiter.limit("20 per hour")
@jwt_required()
def extract_from_attachment(attachment_id: int) -> Response | tuple[Response, int]:
    """
    Extract text from an existing attachment
    """
    try:
        if not ocr_service.is_available():
            return jsonify({
                'success': False,
                'error': 'OCR service is not available'
            }), 503
        
        # Find attachment
        attachment = Attachment.query.get_or_404(attachment_id)

        # Check permission - user must own the attachment or be able to edit the document
        user_id = get_current_user_id()
        has_permission = False

        # Check if user owns the attachment directly
        if attachment.uploaded_by == user_id:
            has_permission = True
        # Check if user can edit the associated document
        elif attachment.document_id:
            document = db.session.get(Document, attachment.document_id)
            if document and document.can_edit(user_id):
                has_permission = True

        if not has_permission:
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # SECURITY: Validate file path is within allowed upload directory
        upload_dir = current_app.config.get('UPLOAD_FOLDER', './uploads')
        real_path = os.path.realpath(attachment.file_path)
        real_upload_dir = os.path.realpath(upload_dir)

        if not real_path.startswith(real_upload_dir + os.sep):
            logger.warning(f"Path traversal attempt: {attachment.file_path}")
            return jsonify({
                'success': False,
                'error': 'Invalid file path'
            }), 400

        # Check if file exists
        if not os.path.exists(real_path):
            return jsonify({
                'success': False,
                'error': 'Attachment file not found'
            }), 404

        # Get language parameter with validation
        ALLOWED_LANGUAGES = {'eng', 'kor', 'jpn', 'chi_sim', 'chi_tra', 'fra', 'deu', 'spa'}
        language = request.json.get('language', 'eng') if request.json else 'eng'
        if language not in ALLOWED_LANGUAGES:
            language = 'eng'  # Default to English if invalid

        # Read file and process with OCR
        with open(real_path, 'rb') as f:
            file_data = f.read()
        
        ocr_result = ocr_service.process_uploaded_file(
            file_data=file_data,
            filename=attachment.filename,
            language=language
        )
        
        # SECURITY: Update attachment metadata with OCR result using immutable pattern
        # Use dict copy to avoid mutation tracking issues
        from sqlalchemy.orm.attributes import flag_modified
        if attachment.metadata:
            new_metadata = dict(attachment.metadata)
            new_metadata['ocr_result'] = ocr_result
            attachment.metadata = new_metadata
        else:
            attachment.metadata = {'ocr_result': ocr_result}
        flag_modified(attachment, 'metadata')

        db.session.commit()
        
        logger.info(f"OCR on attachment: user={user_id}, attachment_id={attachment_id}, "
                   f"success={ocr_result.get('success')}")
        
        return jsonify({
            'success': True,
            'ocr_result': ocr_result,
            'attachment': attachment.to_dict()
        })
        
    except Exception as e:
        logger.error(f"Error extracting from attachment: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to extract text from attachment'
        }), 500