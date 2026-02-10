"""Core CRUD operations for documents."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from pydantic import ValidationError
from app import db
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import get_or_404
from app.schemas.document import DocumentCreate, DocumentUpdate
from sqlalchemy import or_
import bleach
import logging
from datetime import datetime, timezone
from app.utils.auto_tag import detect_auto_tags, merge_tags
from app.utils.obsidian_parser import ObsidianParser
from app.utils.backup_manager import create_document_backup, update_document_backup

logger = logging.getLogger(__name__)

documents_bp = Blueprint('documents', __name__)


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


@documents_bp.route('/documents', methods=['POST'])
@jwt_required(optional=True)
def create_document():
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'Request body is required'}), 400

        try:
            validated = DocumentCreate.model_validate(data)
        except ValidationError as e:
            return jsonify({
                'error': 'Validation failed',
                'details': e.errors()
            }), 400

        current_user_id = get_current_user_id()

        title = bleach.clean(validated.title)
        author = bleach.clean(validated.author) if validated.author else None
        is_public = validated.is_public
        tags = validated.tags
        category_id = validated.category_id

        try:
            obsidian_data = process_obsidian_content(validated.markdown_content, backup_dir="backup")
        except Exception as e:
            logger.warning("Error processing Obsidian content during creation: %s", e)
            obsidian_data = {
                'frontmatter': {},
                'internal_links': [],
                'hashtags': [],
                'all_tags': [],
                'processed_content': validated.markdown_content
            }

        if not title and 'title' in obsidian_data['frontmatter']:
            frontmatter_title = obsidian_data['frontmatter']['title']
            if isinstance(frontmatter_title, str) and frontmatter_title.strip():
                title = frontmatter_title.strip()

        if not author:
            author = extract_author_from_frontmatter(obsidian_data['frontmatter'])

        if not title or not title.strip():
            title = "Untitled Document"

        if category_id:
            from app.models.category import Category
            category = db.session.get(Category, category_id)
            if not category:
                return jsonify({'error': 'Category not found'}), 404

        document = Document(
            title=title,
            markdown_content=obsidian_data.get('processed_content', validated.markdown_content),
            author=author,
            user_id=current_user_id,
            is_public=is_public,
            document_metadata={
                'frontmatter': obsidian_data['frontmatter'],
                'internal_links': obsidian_data['internal_links'],
                'hashtags': obsidian_data['hashtags']
            }
        )

        if category_id:
            document.category_id = category_id

        auto_tags = detect_auto_tags(validated.markdown_content)
        obsidian_tags = obsidian_data.get('all_tags', [])
        all_tags = merge_tags(merge_tags(tags, auto_tags), obsidian_tags)

        if all_tags:
            document.add_tags(all_tags)

        db.session.add(document)
        db.session.commit()

        try:
            backup_path = create_document_backup(document)
            if backup_path:
                logger.info("Document backup created: %s", backup_path)
            else:
                logger.warning("Failed to create backup for document %s", document.id)
        except Exception as backup_error:
            logger.error("Backup creation error for document %s: %s", document.id, backup_error)

        return jsonify(document.to_dict()), 201

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating document: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_bp.route('/documents', methods=['GET'])
@jwt_required(optional=True)
def list_documents():
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        search = request.args.get('search', '')
        include_private = request.args.get('include_private', 'false').lower() == 'true'

        current_user_id = get_current_user_id()

        tags_filter = request.args.getlist('tags')

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
        logger.error("Error listing documents: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_bp.route('/documents/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def get_document(document_id):
    from werkzeug.exceptions import HTTPException
    try:
        document = get_or_404(Document, document_id)
        current_user_id = get_current_user_id()

        if not document.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403

        return jsonify(document.to_dict())

    except HTTPException:
        raise
    except Exception as e:
        logger.error("Error getting document %s: %s", document_id, e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_bp.route('/documents/<int:document_id>', methods=['PUT'])
@jwt_required(optional=True)
def update_document(document_id):
    try:
        document = get_or_404(Document, document_id)
        current_user_id = get_current_user_id()

        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403

        data = request.get_json()
        if not data:
            return jsonify({'error': 'No data provided'}), 400

        try:
            validated = DocumentUpdate.model_validate(data)
        except ValidationError as e:
            return jsonify({
                'error': 'Validation failed',
                'details': e.errors()
            }), 400

        if validated.title is not None:
            document.title = bleach.clean(validated.title)

        if validated.markdown_content is not None:
            document.markdown_content = validated.markdown_content

            try:
                obsidian_data = process_obsidian_content(validated.markdown_content)
                document.document_metadata = {
                    'frontmatter': obsidian_data['frontmatter'],
                    'internal_links': obsidian_data['internal_links'],
                    'hashtags': obsidian_data['hashtags']
                }

                auto_tags = detect_auto_tags(validated.markdown_content)
                obsidian_tags = obsidian_data.get('all_tags', [])
                existing_user_tags = [tag.name for tag in document.tags if not tag.is_auto_tag]
                all_tags = merge_tags(merge_tags(existing_user_tags, auto_tags), obsidian_tags)

                document.tags = []
                if all_tags:
                    document.add_tags(all_tags)

            except Exception as e:
                logger.warning("Error processing Obsidian content during update: %s", e)

        if validated.author is not None:
            document.author = bleach.clean(validated.author) if validated.author else None

        if validated.is_public is not None:
            document.is_public = validated.is_public

        if validated.category_id is not None:
            if validated.category_id == 0:
                document.category_id = None
            else:
                from app.models.category import Category
                category = db.session.get(Category, validated.category_id)
                if not category:
                    return jsonify({'error': 'Category not found'}), 404
                document.category_id = validated.category_id

        if validated.tags is not None:
            document.tags = []
            if validated.tags:
                document.add_tags(validated.tags)

        document.updated_at = datetime.now(timezone.utc)
        db.session.commit()

        try:
            backup_path = update_document_backup(document)
            if backup_path:
                logger.info("Document backup updated: %s", backup_path)
        except Exception as backup_error:
            logger.error("Backup update error for document %s: %s", document.id, backup_error)

        return jsonify(document.to_dict())

    except Exception as e:
        db.session.rollback()
        logger.error("Error updating document %s: %s", document_id, e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_bp.route('/documents/<int:document_id>', methods=['DELETE'])
@jwt_required(optional=True)
def delete_document(document_id):
    try:
        document = get_or_404(Document, document_id)
        current_user_id = get_current_user_id()

        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403

        from app.models.document import document_tags
        db.session.execute(
            document_tags.delete().where(document_tags.c.document_id == document.id)
        )

        db.session.delete(document)
        db.session.commit()

        return jsonify({'message': 'Document deleted successfully'}), 200

    except Exception as e:
        db.session.rollback()
        logger.error("Error deleting document %s: %s", document_id, e)
        return jsonify({'error': 'Internal server error'}), 500
