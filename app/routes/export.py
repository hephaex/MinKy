from flask import Blueprint, request, jsonify, send_file, current_app, Response
from flask_jwt_extended import jwt_required
from app import db, limiter
from app.models.document import Document
from app.utils.auth import get_current_user_id, get_current_user
from app.utils.responses import get_or_404
from app.utils.exporters import DocumentExporter
from app.services.notification_service import NotificationService
import os
import tempfile
import logging
from datetime import datetime, timezone

logger = logging.getLogger(__name__)

export_bp = Blueprint('export', __name__)

def _export_html_format(exporter: DocumentExporter, document: Document) -> Response:
    """Export document to HTML format"""
    include_styles = request.args.get('include_styles', 'true').lower() == 'true'
    content = exporter.export_to_html(include_styles=include_styles)
    exporter.cleanup()

    return current_app.response_class(
        content,
        mimetype='text/html',
        headers={
            'Content-Disposition': f'attachment; filename="{document.title[:50]}.html"'
        }
    )


def _export_json_format(exporter: DocumentExporter, document: Document) -> Response:
    """Export document to JSON format"""
    file_path = exporter.export_to_json()

    with open(file_path, 'r', encoding='utf-8') as f:
        json_content = f.read()

    exporter.cleanup()

    return current_app.response_class(
        json_content,
        mimetype='application/json',
        headers={
            'Content-Disposition': f'attachment; filename="{document.title[:50]}.json"'
        }
    )


def _export_file_format(exporter: DocumentExporter, document: Document, format_type: str, user) -> Response:
    """Export document to file-based formats (PDF, DOCX, Markdown)"""
    format_configs = {
        'pdf': ('application/pdf', exporter.export_to_pdf),
        'docx': ('application/vnd.openxmlformats-officedocument.wordprocessingml.document', exporter.export_to_docx),
        'markdown': ('text/markdown', exporter.export_to_markdown)
    }

    mimetype, export_func = format_configs[format_type]
    file_path = export_func()
    filename = f"{document.title[:50]}.{format_type}"

    def cleanup_after_send():
        try:
            exporter.cleanup()
        except Exception as e:
            logger.debug("Export cleanup error (non-critical): %s", e)

    response = send_file(
        file_path,
        mimetype=mimetype,
        as_attachment=True,
        download_name=filename
    )

    response.call_on_close(cleanup_after_send)

    try:
        NotificationService.create_document_export_notification(document, user, format_type)
    except Exception as notification_error:
        current_app.logger.warning(f"Failed to create export notification: {str(notification_error)}")

    return response


@export_bp.route('/documents/<int:document_id>/export/<format_type>', methods=['GET'])
@limiter.limit("30 per hour")
@jwt_required()
def export_document(document_id: int, format_type: str) -> Response | tuple[Response, int]:
    """Export a single document in the specified format"""
    current_user_id = get_current_user_id()
    user = get_current_user()

    if not user:
        return jsonify({'error': 'User not found'}), 404

    document = get_or_404(Document, document_id)

    if not document.is_public and document.user_id != current_user_id:
        return jsonify({'error': 'Access denied'}), 403

    valid_formats = ['html', 'pdf', 'docx', 'markdown', 'json']
    if format_type not in valid_formats:
        return jsonify({'error': f'Invalid format. Supported formats: {", ".join(valid_formats)}'}), 400

    try:
        exporter = DocumentExporter(document)

        if format_type == 'html':
            return _export_html_format(exporter, document)

        if format_type == 'json':
            return _export_json_format(exporter, document)

        return _export_file_format(exporter, document, format_type, user)

    except Exception as e:
        current_app.logger.error(f"Export error for document {document_id}: {str(e)}")
        return jsonify({'error': 'Export failed', 'details': str(e)}), 500

def _get_accessible_documents(document_ids: list[int], current_user_id: int) -> list[Document]:
    """Get documents user has access to"""
    accessible_documents = []
    for doc_id in document_ids:
        document = db.session.get(Document, doc_id)
        if document and (document.is_public or document.user_id == current_user_id):
            accessible_documents.append(document)
    return accessible_documents


def _export_document_format(document: Document, format_type: str, temp_dir: str, exporter: DocumentExporter) -> str:
    """Export a single document in the specified format and return the file path"""
    base_filename = f"{document.id}_{document.title[:30]}"

    if format_type == 'html':
        content = exporter.export_to_html()
        file_path = os.path.join(temp_dir, f"{base_filename}.html")
        with open(file_path, 'w', encoding='utf-8') as f:
            f.write(content)
        return file_path

    format_exporters = {
        'json': exporter.export_to_json,
        'markdown': exporter.export_to_markdown,
        'pdf': exporter.export_to_pdf,
        'docx': exporter.export_to_docx
    }

    export_func = format_exporters[format_type]
    temp_file_path = export_func()
    final_path = os.path.join(temp_dir, f"{base_filename}.{format_type}")
    os.rename(temp_file_path, final_path)
    return final_path


def _export_all_documents(documents: list[Document], formats: list[str], temp_dir: str) -> list[str]:
    """Export all documents in all requested formats"""
    export_files = []

    for document in documents:
        exporter = DocumentExporter(document)
        try:
            for format_type in formats:
                file_path = _export_document_format(document, format_type, temp_dir, exporter)
                export_files.append(file_path)
        finally:
            exporter.cleanup()

    return export_files


def _create_zip_archive(export_files: list[str], temp_dir: str) -> str:
    """Create a ZIP archive with all exported files"""
    import zipfile
    zip_path = os.path.join(temp_dir, f"bulk_export_{datetime.now(timezone.utc).strftime('%Y%m%d_%H%M%S')}.zip")

    with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        for file_path in export_files:
            arcname = os.path.basename(file_path)
            zf.write(file_path, arcname)

    return zip_path


@export_bp.route('/documents/bulk-export', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required()
def bulk_export_documents() -> Response | tuple[Response, int]:
    """Export multiple documents in specified formats"""
    current_user_id = get_current_user_id()
    user = get_current_user()

    if not user:
        return jsonify({'error': 'User not found'}), 404

    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400

    document_ids = data.get('document_ids', [])
    formats = data.get('formats', ['html'])

    if not document_ids:
        return jsonify({'error': 'document_ids required'}), 400

    if not isinstance(document_ids, list):
        return jsonify({'error': 'document_ids must be a list'}), 400

    valid_formats = ['html', 'pdf', 'docx', 'markdown', 'json']
    for fmt in formats:
        if fmt not in valid_formats:
            return jsonify({'error': f'Invalid format "{fmt}". Supported formats: {", ".join(valid_formats)}'}), 400

    accessible_documents = _get_accessible_documents(document_ids, current_user_id)

    if not accessible_documents:
        return jsonify({'error': 'No accessible documents found'}), 404

    try:
        temp_dir = tempfile.mkdtemp()
        export_files = _export_all_documents(accessible_documents, formats, temp_dir)
        zip_path = _create_zip_archive(export_files, temp_dir)

        def cleanup_bulk_export():
            try:
                import shutil
                shutil.rmtree(temp_dir)
            except Exception as e:
                logger.debug("Bulk export cleanup error (non-critical): %s", e)

        response = send_file(
            zip_path,
            mimetype='application/zip',
            as_attachment=True,
            download_name=f"bulk_export_{len(accessible_documents)}_documents.zip"
        )

        response.call_on_close(cleanup_bulk_export)
        return response

    except Exception as e:
        current_app.logger.error(f"Bulk export error: {str(e)}")
        return jsonify({'error': 'Bulk export failed', 'details': str(e)}), 500

@export_bp.route('/documents/<int:document_id>/export/bundle', methods=['GET'])
@limiter.limit("10 per hour")
@jwt_required()
def export_document_bundle(document_id: int) -> Response | tuple[Response, int]:
    """Export a document in multiple formats as a ZIP bundle"""
    current_user_id = get_current_user_id()
    user = get_current_user()
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = get_or_404(Document, document_id)

    # Check if user has access to this document
    if not document.is_public and document.user_id != current_user_id:
        return jsonify({'error': 'Access denied'}), 403

    # Get formats from query parameters
    formats_param = request.args.get('formats', 'html,pdf,docx,markdown,json')
    formats = [fmt.strip() for fmt in formats_param.split(',')]
    
    # Validate formats
    valid_formats = ['html', 'pdf', 'docx', 'markdown', 'json']
    formats = [fmt for fmt in formats if fmt in valid_formats]
    
    if not formats:
        formats = ['html', 'markdown', 'json']  # Default formats
    
    try:
        exporter = DocumentExporter(document)
        bundle_path = exporter.export_bundle(formats=formats)
        
        def cleanup_bundle():
            try:
                exporter.cleanup()
            except Exception as e:
                logger.debug("Bundle export cleanup error (non-critical): %s", e)
        
        response = send_file(
            bundle_path,
            mimetype='application/zip',
            as_attachment=True,
            download_name=f"{document.title[:50]}_bundle.zip"
        )
        
        response.call_on_close(cleanup_bundle)
        return response
        
    except Exception as e:
        current_app.logger.error(f"Bundle export error for document {document_id}: {str(e)}")
        return jsonify({'error': 'Bundle export failed', 'details': str(e)}), 500

@export_bp.route('/export/formats', methods=['GET'])
def get_export_formats() -> Response:
    """Get list of supported export formats"""
    formats = {
        'html': {
            'name': 'HTML',
            'description': 'Web page format with styling',
            'extension': 'html',
            'mimetype': 'text/html'
        },
        'pdf': {
            'name': 'PDF',
            'description': 'Portable Document Format',
            'extension': 'pdf',
            'mimetype': 'application/pdf'
        },
        'docx': {
            'name': 'Word Document',
            'description': 'Microsoft Word document',
            'extension': 'docx',
            'mimetype': 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
        },
        'markdown': {
            'name': 'Markdown',
            'description': 'Original markdown with metadata',
            'extension': 'md',
            'mimetype': 'text/markdown'
        },
        'json': {
            'name': 'JSON',
            'description': 'Structured data format',
            'extension': 'json',
            'mimetype': 'application/json'
        }
    }
    
    return jsonify({
        'formats': formats,
        'default_bundle_formats': ['html', 'pdf', 'markdown']
    })