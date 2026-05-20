"""Tag auto-generation endpoints."""
from flask import Blueprint, request, Response
from flask_jwt_extended import jwt_required
from app import db
from app.models.tag import Tag
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, get_or_404, success_response, error_response
from app.utils.auto_tag import detect_auto_tags, merge_tags
import logging

logger = logging.getLogger(__name__)

tags_auto_bp = Blueprint('tags_auto', __name__)


def _get_documents_for_auto_tagging(document_id, limit, current_user_id):
    """Get documents to process for auto tagging."""
    if document_id:
        document = db.session.get(Document, document_id)
        if not document:
            return None, error_response('Document not found', 404)
        if not document.can_view(current_user_id):
            return None, error_response('Access denied', 403)
        return [document], None

    query = Document.query.filter(~Document.tags.any())
    if current_user_id:
        query = query.filter(
            (Document.is_public == True) | (Document.user_id == current_user_id)
        )
    else:
        query = query.filter(Document.is_public == True)

    return query.limit(limit).all(), None


def _process_document_tags(doc, dry_run, results):
    """Process tags for a single document."""
    results['processed'] += 1

    if doc.tags:
        results['documents'].append({
            'id': doc.id,
            'title': doc.title,
            'status': 'skipped',
            'reason': 'already_has_tags',
            'existing_tags': [tag.name for tag in doc.tags]
        })
        return

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


def _format_auto_tag_response(dry_run, results):
    """Format auto tag generation response."""
    return success_response({
        'dry_run': dry_run,
        'results': results,
        'summary': {
            'total_processed': results['processed'],
            'documents_tagged': results['tagged'],
            'errors': results['errors']
        }
    })


@tags_auto_bp.route('/tags/auto-generate', methods=['POST'])
@jwt_required(optional=True)
def generate_auto_tags() -> Response | tuple[Response, int]:
    """Generate automatic tags for documents without tags."""
    dry_run = False
    try:
        current_user_id = get_current_user_id()
        data = request.get_json() or {}

        document_id = data.get('document_id')
        limit = data.get('limit', 100)
        dry_run = data.get('dry_run', False)

        results = {
            'processed': 0,
            'tagged': 0,
            'errors': 0,
            'documents': []
        }

        documents, error = _get_documents_for_auto_tagging(document_id, limit, current_user_id)
        if error:
            return error

        logger.info("AUTO_TAG_GENERATION: Processing %d documents", len(documents))

        for doc in documents:
            try:
                _process_document_tags(doc, dry_run, results)
            except Exception as e:
                results['errors'] += 1
                results['documents'].append({
                    'id': doc.id,
                    'title': doc.title,
                    'status': 'error',
                    'error': str(e)
                })
                logger.error("AUTO_TAG_GENERATION: Error processing document %s: %s", doc.id, e)

        if not dry_run:
            db.session.commit()
            logger.info("AUTO_TAG_GENERATION: Committed changes to database")

        return _format_auto_tag_response(dry_run, results)

    except Exception as e:
        if not dry_run:
            db.session.rollback()
        logger.error("AUTO_TAG_GENERATION: Fatal error: %s", e)
        return error_response('Internal server error', 500)


@tags_auto_bp.route('/tags/tagless-documents', methods=['GET'])
@jwt_required(optional=True)
def get_tagless_documents() -> Response | tuple[Response, int]:
    """Get documents that don't have any tags."""
    try:
        current_user_id = get_current_user_id()
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        include_private = request.args.get('include_private', 'false').lower() == 'true'

        query = Document.query.filter(~Document.tags.any())

        if current_user_id and include_private:
            query = query.filter(
                (Document.is_public == True) | (Document.user_id == current_user_id)
            )
        else:
            query = query.filter(Document.is_public == True)

        def serialize_with_preview(doc):
            doc_dict = doc.to_dict_lite()
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
        logger.error("Error getting tagless documents: %s", e)
        return error_response('Internal server error', 500)


@tags_auto_bp.route('/tags/preview-auto-tags/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def preview_auto_tags(document_id: int) -> Response | tuple[Response, int]:
    """Preview what auto tags would be generated for a specific document."""
    try:
        current_user_id = get_current_user_id()

        document = get_or_404(Document, document_id)

        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        content = document.markdown_content or document.content or ''
        auto_tags = detect_auto_tags(content)

        existing_tags = [tag.name for tag in document.tags]

        merged_tags = merge_tags(existing_tags, auto_tags)

        return success_response({
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
        logger.error("Error previewing auto tags for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)
