from flask import Blueprint, request, jsonify, send_file, current_app
from flask_jwt_extended import jwt_required, get_jwt_identity
from app.models.document import Document
from app.models.user import User
from app.utils.exporters import DocumentExporter
from app.services.notification_service import NotificationService
import os
import tempfile
from datetime import datetime

export_bp = Blueprint('export', __name__)

@export_bp.route('/documents/<int:document_id>/export/<format_type>', methods=['GET'])
@jwt_required()
def export_document(document_id, format_type):
    """Export a single document in the specified format"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    # Check if user has access to this document
    if document.is_private and document.author_id != current_user_id:
        return jsonify({'error': 'Access denied'}), 403
    
    # Validate format type
    valid_formats = ['html', 'pdf', 'docx', 'markdown', 'json']
    if format_type not in valid_formats:
        return jsonify({'error': f'Invalid format. Supported formats: {", ".join(valid_formats)}'}), 400
    
    try:
        exporter = DocumentExporter(document)
        
        if format_type == 'html':
            include_styles = request.args.get('include_styles', 'true').lower() == 'true'
            content = exporter.export_to_html(include_styles=include_styles)
            exporter.cleanup()
            
            response = current_app.response_class(
                content,
                mimetype='text/html',
                headers={
                    'Content-Disposition': f'attachment; filename="{document.title[:50]}.html"'
                }
            )
            return response
            
        elif format_type == 'json':
            content = exporter.export_to_json()
            exporter.cleanup()
            
            with open(content, 'r', encoding='utf-8') as f:
                json_content = f.read()
            
            response = current_app.response_class(
                json_content,
                mimetype='application/json',
                headers={
                    'Content-Disposition': f'attachment; filename="{document.title[:50]}.json"'
                }
            )
            return response
            
        else:  # pdf, docx, markdown
            if format_type == 'pdf':
                file_path = exporter.export_to_pdf()
                mimetype = 'application/pdf'
            elif format_type == 'docx':
                file_path = exporter.export_to_docx()
                mimetype = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
            elif format_type == 'markdown':
                file_path = exporter.export_to_markdown()
                mimetype = 'text/markdown'
            
            filename = f"{document.title[:50]}.{format_type}"
            
            def cleanup_after_send():
                try:
                    exporter.cleanup()
                except Exception:
                    pass
            
            response = send_file(
                file_path,
                mimetype=mimetype,
                as_attachment=True,
                download_name=filename
            )
            
            # Schedule cleanup after response is sent
            response.call_on_close(cleanup_after_send)
            
            # Create notification for document export
            try:
                NotificationService.create_document_export_notification(document, user, format_type)
            except Exception as notification_error:
                current_app.logger.warning(f"Failed to create export notification: {str(notification_error)}")
            
            return response
            
    except Exception as e:
        current_app.logger.error(f"Export error for document {document_id}: {str(e)}")
        return jsonify({'error': 'Export failed', 'details': str(e)}), 500

@export_bp.route('/documents/bulk-export', methods=['POST'])
@jwt_required()
def bulk_export_documents():
    """Export multiple documents in specified formats"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
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
    
    # Validate formats
    valid_formats = ['html', 'pdf', 'docx', 'markdown', 'json']
    for fmt in formats:
        if fmt not in valid_formats:
            return jsonify({'error': f'Invalid format "{fmt}". Supported formats: {", ".join(valid_formats)}'}), 400
    
    # Get documents user has access to
    accessible_documents = []
    for doc_id in document_ids:
        document = Document.query.get(doc_id)
        if document and (not document.is_private or document.author_id == current_user_id):
            accessible_documents.append(document)
    
    if not accessible_documents:
        return jsonify({'error': 'No accessible documents found'}), 404
    
    try:
        # Create a temporary directory for the bulk export
        temp_dir = tempfile.mkdtemp()
        export_files = []
        
        for document in accessible_documents:
            exporter = DocumentExporter(document)
            try:
                for format_type in formats:
                    if format_type == 'html':
                        content = exporter.export_to_html()
                        file_path = os.path.join(temp_dir, f"{document.id}_{document.title[:30]}.html")
                        with open(file_path, 'w', encoding='utf-8') as f:
                            f.write(content)
                        export_files.append(file_path)
                    elif format_type == 'json':
                        file_path = exporter.export_to_json()
                        new_path = os.path.join(temp_dir, f"{document.id}_{document.title[:30]}.json")
                        os.rename(file_path, new_path)
                        export_files.append(new_path)
                    elif format_type == 'markdown':
                        file_path = exporter.export_to_markdown()
                        new_path = os.path.join(temp_dir, f"{document.id}_{document.title[:30]}.md")
                        os.rename(file_path, new_path)
                        export_files.append(new_path)
                    elif format_type == 'pdf':
                        file_path = exporter.export_to_pdf()
                        new_path = os.path.join(temp_dir, f"{document.id}_{document.title[:30]}.pdf")
                        os.rename(file_path, new_path)
                        export_files.append(new_path)
                    elif format_type == 'docx':
                        file_path = exporter.export_to_docx()
                        new_path = os.path.join(temp_dir, f"{document.id}_{document.title[:30]}.docx")
                        os.rename(file_path, new_path)
                        export_files.append(new_path)
            finally:
                exporter.cleanup()
        
        # Create ZIP file with all exports
        import zipfile
        zip_path = os.path.join(temp_dir, f"bulk_export_{datetime.utcnow().strftime('%Y%m%d_%H%M%S')}.zip")
        
        with zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
            for file_path in export_files:
                arcname = os.path.basename(file_path)
                zf.write(file_path, arcname)
        
        def cleanup_bulk_export():
            try:
                import shutil
                shutil.rmtree(temp_dir)
            except Exception:
                pass
        
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
@jwt_required()
def export_document_bundle(document_id):
    """Export a document in multiple formats as a ZIP bundle"""
    current_user_id = get_jwt_identity()
    user = User.query.get(current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    # Check if user has access to this document
    if document.is_private and document.author_id != current_user_id:
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
            except Exception:
                pass
        
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
def get_export_formats():
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