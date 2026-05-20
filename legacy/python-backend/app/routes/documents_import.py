"""File upload, export, and import endpoints for documents."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from werkzeug.utils import secure_filename
from app import db, limiter
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.obsidian_parser import ObsidianParser
from app.utils.backup_manager import upload_document_backup, export_all_documents
from app.services.document_import_service import document_import_service
import logging

logger = logging.getLogger(__name__)

documents_import_bp = Blueprint('documents_import', __name__)


def process_obsidian_content(markdown_content, backup_dir=None):
    """Process Obsidian-style content and extract metadata"""
    parser = ObsidianParser()
    parsed = parser.parse_markdown(markdown_content, backup_dir=backup_dir)

    all_tags = set()

    if 'tags' in parsed.get('frontmatter', {}):
        frontmatter_tags = parsed['frontmatter']['tags']
        if isinstance(frontmatter_tags, list):
            all_tags.update(frontmatter_tags)
        elif isinstance(frontmatter_tags, str):
            all_tags.update(tag.strip() for tag in frontmatter_tags.split(','))

    for hashtag in parsed.get('hashtags', []):
        all_tags.add(hashtag.get('tag', ''))

    filtered_tags = [tag for tag in all_tags if tag and tag.lower() != 'clippings']

    return {
        'frontmatter': parsed.get('frontmatter', {}),
        'internal_links': parsed.get('internal_links', []),
        'hashtags': parsed.get('hashtags', []),
        'all_tags': filtered_tags,
        'processed_content': parsed.get('clean_content', markdown_content)
    }


def extract_author_from_frontmatter(frontmatter):
    """Extract author from frontmatter, handling various formats"""
    if not frontmatter:
        return None

    author = frontmatter.get('author')
    if not author:
        return None

    if isinstance(author, list):
        if len(author) > 0:
            author = author[0]
        else:
            return None

    if isinstance(author, str):
        author = author.strip()
        if author.startswith('[[') and author.endswith(']]'):
            author = author[2:-2]
        author = author.strip('"\'')
        return author if author else None

    return None


@documents_import_bp.route('/documents/upload', methods=['POST'])
@limiter.limit("10 per hour")
@jwt_required()
def upload_markdown_file():
    """Upload a markdown file and create a document"""
    try:
        current_user_id = get_current_user_id()

        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400

        file = request.files['file']
        if file.filename == '':
            return jsonify({'error': 'No file selected'}), 400

        if not file.filename.lower().endswith('.md'):
            return jsonify({'error': 'Only markdown files (.md) are allowed'}), 400

        # SECURITY: Check file size before reading into memory to prevent DoS
        MAX_MARKDOWN_SIZE = 10 * 1024 * 1024  # 10MB
        file.seek(0, 2)  # Seek to end
        file_size = file.tell()
        file.seek(0)  # Reset position

        if file_size > MAX_MARKDOWN_SIZE:
            return jsonify({'error': f'File too large. Maximum size is {MAX_MARKDOWN_SIZE // (1024*1024)}MB'}), 413

        content = file.read().decode('utf-8')
        # SECURITY: Use secure_filename to prevent path traversal in title extraction
        safe_filename = secure_filename(file.filename)
        title = safe_filename[:-3] if safe_filename.endswith('.md') else safe_filename

        try:
            obsidian_data = process_obsidian_content(content, backup_dir="backup")
        except Exception as e:
            logger.warning("Error processing Obsidian content during upload: %s", e)
            obsidian_data = {
                'frontmatter': {},
                'internal_links': [],
                'hashtags': [],
                'all_tags': [],
                'processed_content': content
            }

        if 'title' in obsidian_data['frontmatter']:
            frontmatter_title = obsidian_data['frontmatter']['title']
            if frontmatter_title and isinstance(frontmatter_title, str) and frontmatter_title.strip():
                title = frontmatter_title.strip()

        if not title or not title.strip():
            title = "Untitled Document"

        if title is None:
            title = "Untitled Document"

        document = Document(
            title=title,
            markdown_content=obsidian_data.get('processed_content', content),
            author=extract_author_from_frontmatter(obsidian_data['frontmatter']),
            user_id=current_user_id,
            is_public=obsidian_data['frontmatter'].get('public', True),
            document_metadata={
                'frontmatter': obsidian_data['frontmatter'],
                'internal_links': obsidian_data['internal_links'],
                'hashtags': obsidian_data['hashtags']
            }
        )

        obsidian_tags = obsidian_data.get('all_tags', [])

        ai_tags = []
        try:
            from app.services.ai_service import ai_service
            ai_tags = ai_service.suggest_tags(content, title)
            logger.info(f"AI generated tags for uploaded document '{title}': {ai_tags}")
        except Exception as e:
            logger.warning(f"Failed to generate AI tags for uploaded document: {e}")

        all_tags = list(set(obsidian_tags + ai_tags))

        if all_tags:
            document.add_tags(all_tags)

        db.session.add(document)
        db.session.commit()

        try:
            backup_path = upload_document_backup(document)
            if backup_path:
                logger.info("Document backup created: %s", backup_path)
        except Exception as backup_error:
            logger.error("Backup creation error for uploaded document %s: %s", document.id, backup_error)

        return jsonify({
            'message': 'File uploaded successfully',
            'document': document.to_dict()
        }), 201

    except Exception as e:
        db.session.rollback()
        logger.error("Error uploading document: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_import_bp.route('/documents/export', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required()
def export_all_documents_to_backup():
    """Export all documents to backup folder (admin only)"""
    try:
        # SECURITY: Require admin privileges for bulk export of ALL documents
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required'}), 403

        data = request.get_json() or {}
        use_short_filename = data.get('short_filename', False)

        results = export_all_documents(use_short_filename=use_short_filename)

        logger.info(f"Admin {user.id} exported all documents: {results['exported']} documents")

        return jsonify({
            'message': f'Export completed: {results["exported"]} documents exported',
            'results': results
        })

    except Exception as e:
        logger.error("Error exporting documents: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_import_bp.route('/documents/import', methods=['POST'])
@limiter.limit("10 per hour")
@jwt_required()
def import_document():
    """Import various document formats and convert to Markdown"""
    try:
        # SECURITY: Get current user for document ownership
        current_user_id = get_current_user_id()

        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400

        file = request.files['file']
        if file.filename == '':
            return jsonify({'error': 'No file selected'}), 400

        if not document_import_service.is_supported_file(file):
            # SECURITY: Don't expose MIME type in error message to prevent information disclosure
            return jsonify({
                'error': 'Unsupported file type',
                'supported_extensions': ['.docx', '.pptx', '.xlsx', '.pdf', '.html', '.txt', '.csv', '.json', '.xml']
            }), 400

        auto_tag = request.form.get('auto_tag', 'true').lower() == 'true'

        # SECURITY: Pass user_id to ensure document ownership
        result = document_import_service.import_file(file, user_id=current_user_id, auto_tag=auto_tag)

        if result['success']:
            return jsonify(result), 201
        else:
            return jsonify(result), 400

    except Exception as e:
        logger.error("Error importing document: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error',
            'message': 'Import failed due to an unexpected error'
        }), 500


@documents_import_bp.route('/documents/import/supported-types', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
@jwt_required()  # SECURITY: Require authentication for API endpoint
def get_supported_import_types():
    """Get list of supported file types for import (authenticated)"""
    return jsonify({
        'supported_types': document_import_service.supported_types,
        'extensions': ['.docx', '.pptx', '.xlsx', '.pdf', '.html', '.htm', '.txt',
                      '.csv', '.json', '.xml', '.png', '.jpg', '.jpeg', '.zip']
    })
