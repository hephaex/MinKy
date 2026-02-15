"""Sync operations for documents with backup files."""
from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from app import limiter
from app.utils.auth import get_current_user_id
import logging

logger = logging.getLogger(__name__)

documents_sync_bp = Blueprint('documents_sync', __name__)


@documents_sync_bp.route('/documents/sync', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required()
def sync_backup_files():
    """Sync backup files with database"""
    try:
        # SECURITY: Require admin privileges for full sync operations
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required for sync operations'}), 403

        from app.utils.backup_sync import sync_manager

        current_user_id = get_current_user_id()

        data = request.get_json() or {}
        dry_run = data.get('dry_run', False)

        # SECURITY: Validate dry_run is boolean type
        if not isinstance(dry_run, bool):
            return jsonify({'error': 'dry_run must be a boolean'}), 400

        # Perform full sync
        sync_results = sync_manager.perform_full_sync(
            user_id=current_user_id,
            dry_run=dry_run
        )

        return jsonify({
            'message': 'Sync completed' if not dry_run else 'Sync preview completed',
            'dry_run': dry_run,
            'results': sync_results
        })

    except Exception as e:
        logger.error("Error syncing backup files: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_sync_bp.route('/documents/sync/preview', methods=['GET'])
@limiter.limit("10 per hour")
@jwt_required()
def preview_backup_sync():
    """Preview backup file synchronization"""
    try:
        # SECURITY: Require admin privileges for sync preview (scans all files)
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required for sync preview'}), 403

        from app.utils.backup_sync import sync_manager

        current_user_id = get_current_user_id()

        # Dry run for sync preview
        sync_results = sync_manager.perform_full_sync(
            user_id=current_user_id,
            dry_run=True
        )

        return jsonify({
            'message': 'Sync preview completed',
            'preview': True,
            'results': sync_results
        })

    except Exception as e:
        logger.error("Error previewing sync: %s", e)
        return jsonify({'error': 'Internal server error'}), 500


@documents_sync_bp.route('/documents/sync/files', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required()
def list_backup_files_for_sync():
    """List backup files available for synchronization"""
    try:
        # SECURITY: Require admin privileges to list server files
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required to list sync files'}), 403

        from app.utils.backup_sync import sync_manager

        current_user_id = get_current_user_id()

        backup_files = sync_manager.scan_backup_files()

        # Add sync status info for each file
        file_info = []
        for backup_info in backup_files:
            # SECURITY: Pass user_id to filter by authorization
            existing_doc = sync_manager.find_matching_document(backup_info, user_id=current_user_id)

            if existing_doc:
                comparison = sync_manager.compare_document_versions(existing_doc, backup_info)
                status_info = {
                    'has_matching_document': True,
                    'document_id': existing_doc.id,
                    'comparison': comparison
                }
            else:
                status_info = {
                    'has_matching_document': False,
                    'will_create_new': True
                }

            file_info.append({
                'filename': backup_info['filename'],
                'title': backup_info['title'],
                'file_mtime': backup_info['file_mtime'].isoformat(),
                'tags': backup_info['tags'],
                'status': status_info
            })

        return jsonify({
            'backup_files': file_info,
            'total_files': len(file_info)
        })

    except Exception as e:
        logger.error("Error listing backup files: %s", e)
        return jsonify({'error': 'Internal server error'}), 500
