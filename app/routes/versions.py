from flask import Blueprint, request
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db
from app.models.document import Document
from app.models.version import DocumentVersion, DocumentSnapshot
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, success_response, error_response
import logging

logger = logging.getLogger(__name__)

versions_bp = Blueprint('versions', __name__)

@versions_bp.route('/documents/<int:document_id>/versions', methods=['GET'])
def get_document_versions(document_id):
    """Get version history for a document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        include_content = request.args.get('include_content', 'false').lower() == 'true'
        
        query = DocumentVersion.query.filter_by(document_id=document_id)\
            .order_by(DocumentVersion.version_number.desc())

        return paginate_query(
            query, page, per_page,
            serializer_func=lambda v: v.to_dict(include_content=include_content),
            items_key='versions',
            extra_fields={
                'document': {
                    'id': document.id,
                    'title': document.title,
                    'current_version': document.get_latest_version_number()
                }
            }
        )
        
    except Exception as e:
        logger.error("Error getting versions for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@versions_bp.route('/documents/<int:document_id>/versions/<int:version_number>', methods=['GET'])
def get_document_version(document_id, version_number):
    """Get a specific version of a document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        version = DocumentVersion.query.filter_by(
            document_id=document_id,
            version_number=version_number
        ).first_or_404()

        include_diff = request.args.get('include_diff', 'false').lower() == 'true'

        result = version.to_dict()

        if include_diff:
            diff = version.get_diff_from_previous()
            result['diff'] = diff

        return success_response(result)

    except Exception as e:
        logger.error("Error getting version %s for document %s: %s", version_number, document_id, e)
        return error_response('Internal server error', 500)

@versions_bp.route('/documents/<int:document_id>/versions/<int:version_number>/restore', methods=['POST'])
@jwt_required()
def restore_document_version(document_id, version_number):
    """Restore a document to a specific version"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_jwt_identity()
        
        # Check if user can edit document
        if not document.can_edit(current_user_id):
            return error_response('Access denied', 403)

        version = DocumentVersion.query.filter_by(
            document_id=document_id,
            version_number=version_number
        ).first_or_404()

        data = request.get_json() or {}
        change_summary = data.get('change_summary', f'Restored to version {version_number}')

        # Create a version of current state before restoring
        document.create_version(
            change_summary=f'Pre-restore backup (before restoring to v{version_number})',
            created_by=current_user_id
        )

        # Restore to the specified version
        version.restore_to_document()

        # Create another version for the restore action
        document.create_version(
            change_summary=change_summary,
            created_by=current_user_id
        )

        db.session.commit()

        return success_response({
            'message': f'Document restored to version {version_number}',
            'document': document.to_dict(),
            'restored_from_version': version_number,
            'new_version': document.get_latest_version_number()
        })

    except Exception as e:
        db.session.rollback()
        logger.error("Error restoring version %s for document %s: %s", version_number, document_id, e)
        return error_response('Internal server error', 500)

@versions_bp.route('/documents/<int:document_id>/versions/<int:version_number>/diff', methods=['GET'])
def get_version_diff(document_id, version_number):
    """Get diff between a version and its previous version"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()

        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        version = DocumentVersion.query.filter_by(
            document_id=document_id,
            version_number=version_number
        ).first_or_404()

        diff = version.get_diff_from_previous()

        if not diff:
            return success_response({
                'message': 'This is the first version, no diff available',
                'version_number': version_number
            })

        return success_response({
            'diff': diff,
            'version_number': version_number
        })

    except Exception as e:
        logger.error("Error getting diff for version %s of document %s: %s", version_number, document_id, e)
        return error_response('Internal server error', 500)

@versions_bp.route('/documents/<int:document_id>/versions/compare', methods=['GET'])
def compare_versions(document_id):
    """Compare two specific versions"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()

        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        version1_num = request.args.get('version1', type=int)
        version2_num = request.args.get('version2', type=int)

        if not version1_num or not version2_num:
            return error_response('Both version1 and version2 parameters are required', 400)

        version1 = DocumentVersion.query.filter_by(
            document_id=document_id,
            version_number=version1_num
        ).first_or_404()

        version2 = DocumentVersion.query.filter_by(
            document_id=document_id,
            version_number=version2_num
        ).first_or_404()

        # Generate diff between the two versions
        import difflib

        title_diff = list(difflib.unified_diff(
            version1.title.splitlines(keepends=True),
            version2.title.splitlines(keepends=True),
            fromfile=f"v{version1_num}/title",
            tofile=f"v{version2_num}/title",
            lineterm=''
        ))

        content_diff = list(difflib.unified_diff(
            version1.markdown_content.splitlines(keepends=True),
            version2.markdown_content.splitlines(keepends=True),
            fromfile=f"v{version1_num}/content",
            tofile=f"v{version2_num}/content",
            lineterm=''
        ))

        return success_response({
            'version1': version1.to_dict(),
            'version2': version2.to_dict(),
            'diff': {
                'title_diff': title_diff,
                'content_diff': content_diff,
                'has_changes': bool(title_diff or content_diff)
            }
        })

    except Exception as e:
        logger.error("Error comparing versions for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@versions_bp.route('/documents/<int:document_id>/snapshots', methods=['GET'])
def get_document_snapshots(document_id):
    """Get snapshots for a document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()

        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        snapshots = DocumentSnapshot.query.filter_by(document_id=document_id)\
            .order_by(DocumentSnapshot.version_number.desc()).all()

        return success_response({
            'snapshots': [snapshot.to_dict() for snapshot in snapshots],
            'document': {
                'id': document.id,
                'title': document.title
            }
        })

    except Exception as e:
        logger.error("Error getting snapshots for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)