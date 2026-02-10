"""File upload, export, and import endpoints for documents."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from app import db
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.auto_tag import detect_auto_tags, merge_tags
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
@jwt_required(optional=True)
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

        content = file.read().decode('utf-8')
        title = file.filename[:-3] if file.filename.endswith('.md') else file.filename

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
@jwt_required(optional=True)
def export_all_documents_to_backup():
    """Export all documents to backup folder"""
    try:
        data = request.get_json() or {}
        use_short_filename = data.get('short_filename', False)

        results = export_all_documents(use_short_filename=use_short_filename)

        return jsonify({
            'message': f'Export completed: {results["exported"]} documents exported',
            'results': results
        })

    except Exception as e:
        logger.error("Error exporting documents: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_import_bp.route('/documents/import', methods=['POST'])
@jwt_required(optional=True)
def import_document():
    """Import various document formats and convert to Markdown"""
    try:
        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400

        file = request.files['file']
        if file.filename == '':
            return jsonify({'error': 'No file selected'}), 400

        if not document_import_service.is_supported_file(file):
            return jsonify({
                'error': f'Unsupported file type: {file.mimetype}',
                'supported_types': list(document_import_service.supported_types.keys())
            }), 400

        auto_tag = request.form.get('auto_tag', 'true').lower() == 'true'

        result = document_import_service.import_file(file, auto_tag=auto_tag)

        if result['success']:
            return jsonify(result), 201
        else:
            return jsonify(result), 400

    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e),
            'message': 'Import failed due to an unexpected error'
        }), 500


@documents_import_bp.route('/documents/import/supported-types', methods=['GET'])
def get_supported_import_types():
    """Get list of supported file types for import"""
    return jsonify({
        'supported_types': document_import_service.supported_types,
        'extensions': ['.docx', '.pptx', '.xlsx', '.pdf', '.html', '.htm', '.txt',
                      '.csv', '.json', '.xml', '.png', '.jpg', '.jpeg', '.zip']
    })
