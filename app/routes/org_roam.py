from typing import Dict, Any
from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import jwt_required
from app.models.document import Document
from app.models.user import User
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query
from app.utils.org_roam_parser import OrgRoamParser, OrgRoamImporter
from app.middleware.security import rate_limit_api, rate_limit_upload, validate_request_security, audit_log
from marshmallow import Schema, fields, ValidationError
from werkzeug.utils import secure_filename
import os
import tempfile
import zipfile
from datetime import datetime, timezone
from app import db

org_roam_bp = Blueprint('org_roam', __name__)

class OrgRoamImportSchema(Schema):
    import_as_private = fields.Bool(load_default=True)
    preserve_links = fields.Bool(load_default=True)
    auto_tag = fields.Bool(load_default=True)
    overwrite_existing = fields.Bool(load_default=False)

@org_roam_bp.route('/org-roam/upload', methods=['POST'])
@jwt_required()
@rate_limit_upload("5 per hour")
@validate_request_security
@audit_log("org_roam_file_upload")
def upload_org_roam_files():
    """org-roam 파일 업로드 (단일 파일 또는 ZIP)"""
    current_user_id = get_current_user_id()
    user = db.session.get(User, current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    if 'files' not in request.files:
        return jsonify({'error': 'No files provided'}), 400
    
    files = request.files.getlist('files')
    if not files or files[0].filename == '':
        return jsonify({'error': 'No files selected'}), 400
    
    # 설정 파라미터
    import_as_private = request.form.get('import_as_private', 'true').lower() == 'true'
    preserve_links = request.form.get('preserve_links', 'true').lower() == 'true'
    auto_tag = request.form.get('auto_tag', 'true').lower() == 'true'
    overwrite_existing = request.form.get('overwrite_existing', 'false').lower() == 'true'
    
    try:
        upload_results = {
            'uploaded_files': [],
            'processed_files': 0,
            'total_files': len(files),
            'errors': []
        }
        
        # 임시 디렉토리 생성
        with tempfile.TemporaryDirectory() as temp_dir:
            org_files = []
            
            for file in files:
                if not file.filename:
                    continue
                
                filename = secure_filename(file.filename)
                file_path = os.path.join(temp_dir, filename)
                
                try:
                    file.save(file_path)
                    
                    if filename.endswith('.zip'):
                        # ZIP 파일 압축 해제
                        extracted_files = _extract_zip_file(file_path, temp_dir)
                        org_files.extend(extracted_files)
                    elif filename.endswith('.org'):
                        # 단일 org 파일
                        org_files.append(file_path)
                    else:
                        upload_results['errors'].append(f"Unsupported file type: {filename}")
                    
                    upload_results['uploaded_files'].append({
                        'filename': filename,
                        'size': os.path.getsize(file_path),
                        'type': 'zip' if filename.endswith('.zip') else 'org'
                    })
                    
                except Exception as e:
                    upload_results['errors'].append(f"Failed to process {filename}: {str(e)}")
            
            # org 파일 파싱 및 임포트
            if org_files:
                import_results = _import_org_files(
                    org_files, current_user_id,
                    import_as_private, preserve_links, auto_tag, overwrite_existing
                )
                upload_results.update(import_results)
            
            upload_results['processed_files'] = len(org_files)
        
        return jsonify({
            'message': 'File upload and processing completed',
            'results': upload_results
        })
        
    except Exception as e:
        current_app.logger.error(f"Org-roam upload failed: {str(e)}")
        return jsonify({'error': 'Upload failed', 'details': str(e)}), 500

def _extract_zip_file(zip_path: str, extract_dir: str) -> list:
    """ZIP 파일에서 org 파일들 추출"""
    org_files = []
    
    try:
        with zipfile.ZipFile(zip_path, 'r') as zip_ref:
            for file_info in zip_ref.filelist:
                if file_info.filename.endswith('.org') and not file_info.is_dir():
                    # 안전한 파일명으로 변경
                    safe_filename = secure_filename(os.path.basename(file_info.filename))
                    extract_path = os.path.join(extract_dir, safe_filename)
                    
                    # 파일 추출
                    with zip_ref.open(file_info) as source, open(extract_path, 'wb') as target:
                        target.write(source.read())
                    
                    org_files.append(extract_path)
    
    except Exception as e:
        current_app.logger.error(f"Failed to extract ZIP file: {e}")
        raise
    
    return org_files

def _import_org_files(org_files: list, user_id: int, import_as_private: bool,
                     preserve_links: bool, auto_tag: bool, overwrite_existing: bool) -> Dict[str, Any]:
    """org 파일들을 데이터베이스로 임포트"""
    parser = OrgRoamParser()
    results: Dict[str, Any] = {
        'imported': 0,
        'updated': 0,
        'skipped': 0,
        'failed': 0,
        'errors': [],
        'documents': []
    }
    
    try:
        for org_file in org_files:
            try:
                # org 파일 파싱
                org_doc = parser.parse_org_file(org_file)
                if not org_doc:
                    results['failed'] += 1
                    results['errors'].append(f"Failed to parse {os.path.basename(org_file)}")
                    continue
                
                # 기존 문서 확인
                existing_doc = None
                if org_doc.get('id'):
                    # org-roam ID로 검색
                    existing_doc = Document.query.filter(
                        Document.document_metadata['org_roam_id'].astext == org_doc['id']
                    ).first()
                
                if not existing_doc:
                    # 제목으로 검색
                    existing_doc = Document.query.filter_by(
                        title=org_doc['title'],
                        user_id=user_id
                    ).first()
                
                if existing_doc and not overwrite_existing:
                    results['skipped'] += 1
                    continue
                
                # 마크다운 변환
                from app.utils.org_roam_parser import OrgRoamImporter
                importer = OrgRoamImporter(db.session)
                markdown_content = importer._convert_org_to_markdown(org_doc)
                
                # 한국어 처리
                from app.utils.korean_text import process_korean_document
                korean_processing = process_korean_document(org_doc['title'], markdown_content)
                
                if existing_doc:
                    # 기존 문서 업데이트
                    existing_doc.markdown_content = markdown_content
                    existing_doc.html_content = existing_doc.convert_markdown_to_html()
                    existing_doc.updated_at = datetime.now(timezone.utc)
                    
                    # 메타데이터 업데이트
                    existing_doc.document_metadata.update({
                        'org_roam_id': org_doc.get('id'),
                        'org_filename': org_doc['filename'],
                        'roam_tags': org_doc.get('roam_tags', []),
                        'roam_aliases': org_doc.get('roam_aliases', []),
                        'language': org_doc['language'],
                        'last_import_date': datetime.now(timezone.utc).isoformat()
                    })
                    
                    results['updated'] += 1
                    action = 'updated'
                else:
                    # 새 문서 생성
                    document = Document(
                        title=org_doc['title'],
                        markdown_content=markdown_content,
                        author=f"Imported from {org_doc['filename']}",
                        user_id=user_id,
                        is_public=not import_as_private
                    )
                    
                    # 메타데이터 설정
                    document.document_metadata = {
                        'org_roam_id': org_doc.get('id'),
                        'org_filename': org_doc['filename'],
                        'org_file_path': org_doc.get('file_path', ''),
                        'roam_tags': org_doc.get('roam_tags', []),
                        'roam_aliases': org_doc.get('roam_aliases', []),
                        'language': org_doc['language'],
                        'import_date': datetime.now(timezone.utc).isoformat(),
                        'preserve_links': preserve_links
                    }
                    
                    db.session.add(document)
                    existing_doc = document
                    results['imported'] += 1
                    action = 'imported'
                
                # 태그 처리
                if auto_tag:
                    all_tags = []
                    all_tags.extend(org_doc.get('roam_tags', []))
                    all_tags.extend(org_doc.get('tags', []))
                    if korean_processing.get('auto_tags'):
                        all_tags.extend(korean_processing['auto_tags'])
                    
                    if all_tags:
                        existing_doc.add_tags(list(set(all_tags)))
                
                # 결과에 문서 정보 추가
                results['documents'].append({
                    'title': org_doc['title'],
                    'filename': org_doc['filename'],
                    'action': action,
                    'language': org_doc['language'],
                    'tags_count': len(org_doc.get('roam_tags', []) + org_doc.get('tags', [])),
                    'links_count': len(org_doc.get('links', [])),
                    'word_count': len(korean_processing.get('content_tokens', []))
                })
                
            except Exception as e:
                results['failed'] += 1
                results['errors'].append(f"Failed to import {os.path.basename(org_file)}: {str(e)}")
                current_app.logger.error(f"Import error for {org_file}: {e}")
        
        db.session.commit()
        
    except Exception as e:
        db.session.rollback()
        results['errors'].append(f"Database error: {str(e)}")
        raise
    
    return results

@org_roam_bp.route('/org-roam/import-directory', methods=['POST'])
@jwt_required()
@rate_limit_api("2 per hour")
@validate_request_security
@audit_log("org_roam_directory_import")
def import_org_roam_directory():
    """org-roam 디렉토리 경로를 통한 임포트 (서버 파일 시스템)"""
    current_user_id = get_current_user_id()
    user = db.session.get(User, current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    # 관리자만 서버 디렉토리 접근 허용
    if not user.is_admin:
        return jsonify({'error': 'Admin privileges required for directory import'}), 403
    
    data = request.get_json()
    if not data:
        return jsonify({'error': 'Request body required'}), 400
    
    directory_path = data.get('directory_path')
    if not directory_path:
        return jsonify({'error': 'directory_path required'}), 400
    
    # 데이터 검증
    schema = OrgRoamImportSchema()
    try:
        import_settings = schema.load(data)
    except ValidationError as e:
        return jsonify({'error': 'Invalid settings', 'details': e.messages}), 400
    
    try:
        # 디렉토리 존재 확인
        if not os.path.exists(directory_path) or not os.path.isdir(directory_path):
            return jsonify({'error': 'Directory not found or not accessible'}), 400
        
        # OrgRoamImporter 사용
        importer = OrgRoamImporter(db.session)
        results = importer.import_from_directory(
            directory_path=directory_path,
            user_id=current_user_id,
            import_as_private=import_settings['import_as_private']
        )
        
        return jsonify({
            'message': 'Directory import completed',
            'directory_path': directory_path,
            'results': results
        })
        
    except Exception as e:
        current_app.logger.error(f"Directory import failed: {str(e)}")
        return jsonify({'error': 'Directory import failed', 'details': str(e)}), 500

@org_roam_bp.route('/org-roam/documents', methods=['GET'])
@jwt_required()
@rate_limit_api("30 per minute")
@validate_request_security
def get_org_roam_documents():
    """org-roam에서 임포트된 문서 목록"""
    current_user_id = get_current_user_id()
    user = db.session.get(User, current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    page = int(request.args.get('page', 1))
    per_page = min(int(request.args.get('per_page', 20)), 100)
    
    try:
        # org-roam 메타데이터가 있는 문서들만 검색
        base_query = Document.query.filter(
            Document.user_id == current_user_id,
            Document.document_metadata.has_key('org_roam_id')  # org-roam ID가 있는 문서
        )
        
        def serialize_org_roam_doc(doc):
            doc_dict = doc.to_dict()
            metadata = doc.document_metadata or {}
            doc_dict['org_roam_info'] = {
                'org_roam_id': metadata.get('org_roam_id'),
                'org_filename': metadata.get('org_filename'),
                'roam_tags': metadata.get('roam_tags', []),
                'roam_aliases': metadata.get('roam_aliases', []),
                'import_date': metadata.get('import_date'),
                'language': metadata.get('language'),
                'backlinks_count': len(metadata.get('backlinks', [])),
                'outbound_links_count': len(metadata.get('outbound_links', []))
            }
            return doc_dict

        query = base_query.order_by(Document.updated_at.desc())
        return paginate_query(
            query, page, per_page,
            serializer_func=serialize_org_roam_doc,
            items_key='documents'
        )
        
    except Exception as e:
        current_app.logger.error(f"Failed to get org-roam documents: {str(e)}")
        return jsonify({'error': 'Failed to get documents', 'details': str(e)}), 500

@org_roam_bp.route('/org-roam/documents/<int:document_id>/links', methods=['GET'])
@jwt_required()
@rate_limit_api("60 per minute")
@validate_request_security
def get_document_links(document_id):
    """문서의 링크 관계 정보 (백링크 + 아웃바운드 링크)"""
    current_user_id = get_current_user_id()
    user = db.session.get(User, current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    document = Document.query.get_or_404(document_id)
    
    # 접근 권한 확인
    if not document.can_view(current_user_id):
        return jsonify({'error': 'Access denied'}), 403
    
    try:
        metadata = document.document_metadata or {}
        
        # 백링크 정보
        backlinks = metadata.get('backlinks', [])
        outbound_links = metadata.get('outbound_links', [])
        
        # 실제 문서 존재 여부 확인하여 링크 정보 보강
        enhanced_backlinks = []
        for backlink in backlinks:
            source_doc = None
            if backlink.get('source_id'):
                source_doc = Document.query.filter(
                    Document.document_metadata['org_roam_id'].astext == backlink['source_id']
                ).first()
            
            if not source_doc and backlink.get('source_filename'):
                source_doc = Document.query.filter_by(
                    user_id=current_user_id
                ).filter(
                    Document.document_metadata['org_filename'].astext == backlink['source_filename']
                ).first()
            
            enhanced_backlink = {
                'link_info': backlink,
                'document_exists': source_doc is not None,
                'document_id': source_doc.id if source_doc else None,
                'document_title': source_doc.title if source_doc else backlink.get('source_title'),
                'accessible': source_doc.can_view(current_user_id) if source_doc else False
            }
            enhanced_backlinks.append(enhanced_backlink)
        
        # 아웃바운드 링크 정보 보강
        enhanced_outbound_links = []
        for outbound_link in outbound_links:
            target_doc = None
            if outbound_link.get('target_id'):
                target_doc = Document.query.filter(
                    Document.document_metadata['org_roam_id'].astext == outbound_link['target_id']
                ).first()
            
            if not target_doc and outbound_link.get('target_filename'):
                target_doc = Document.query.filter_by(
                    user_id=current_user_id
                ).filter(
                    Document.document_metadata['org_filename'].astext == outbound_link['target_filename']
                ).first()
            
            enhanced_outbound_link = {
                'link_info': outbound_link,
                'document_exists': target_doc is not None,
                'document_id': target_doc.id if target_doc else None,
                'document_title': target_doc.title if target_doc else outbound_link.get('target_title'),
                'accessible': target_doc.can_view(current_user_id) if target_doc else False
            }
            enhanced_outbound_links.append(enhanced_outbound_link)
        
        return jsonify({
            'document_id': document_id,
            'document_title': document.title,
            'backlinks': enhanced_backlinks,
            'outbound_links': enhanced_outbound_links,
            'statistics': {
                'backlinks_count': len(enhanced_backlinks),
                'outbound_links_count': len(enhanced_outbound_links),
                'existing_backlinks': len([bl for bl in enhanced_backlinks if bl['document_exists']]),
                'existing_outbound_links': len([ol for ol in enhanced_outbound_links if ol['document_exists']]),
                'broken_links': len([ol for ol in enhanced_outbound_links if not ol['document_exists']])
            }
        })
        
    except Exception as e:
        current_app.logger.error(f"Failed to get document links: {str(e)}")
        return jsonify({'error': 'Failed to get document links', 'details': str(e)}), 500

@org_roam_bp.route('/org-roam/statistics', methods=['GET'])
@jwt_required()
@rate_limit_api("10 per minute")
@validate_request_security
def get_org_roam_statistics():
    """org-roam 임포트 통계"""
    current_user_id = get_current_user_id()
    user = db.session.get(User, current_user_id)
    
    if not user:
        return jsonify({'error': 'User not found'}), 404
    
    try:
        from sqlalchemy import func
        
        # 기본 통계
        total_org_docs = Document.query.filter(
            Document.user_id == current_user_id,
            Document.document_metadata.has_key('org_roam_id')
        ).count()
        
        # 언어별 분포
        language_stats = db.session.query(
            Document.document_metadata['language'].astext.label('language'),
            func.count(Document.id).label('count')
        ).filter(
            Document.user_id == current_user_id,
            Document.document_metadata.has_key('org_roam_id')
        ).group_by('language').all()
        
        # 최근 임포트 문서들
        recent_imports = Document.query.filter(
            Document.user_id == current_user_id,
            Document.document_metadata.has_key('org_roam_id')
        ).order_by(Document.created_at.desc()).limit(5).all()
        
        # roam_tags 통계
        roam_tags_stats = {}
        org_docs = Document.query.filter(
            Document.user_id == current_user_id,
            Document.document_metadata.has_key('org_roam_id')
        ).all()
        
        for doc in org_docs:
            roam_tags = doc.document_metadata.get('roam_tags', [])
            for tag in roam_tags:
                roam_tags_stats[tag] = roam_tags_stats.get(tag, 0) + 1
        
        # 링크 통계
        total_backlinks = 0
        total_outbound_links = 0
        for doc in org_docs:
            total_backlinks += len(doc.document_metadata.get('backlinks', []))
            total_outbound_links += len(doc.document_metadata.get('outbound_links', []))
        
        statistics = {
            'total_org_roam_documents': total_org_docs,
            'language_distribution': {lang: count for lang, count in language_stats},
            'total_backlinks': total_backlinks,
            'total_outbound_links': total_outbound_links,
            'avg_links_per_document': (total_backlinks + total_outbound_links) / max(total_org_docs, 1),
            'popular_roam_tags': sorted(roam_tags_stats.items(), key=lambda x: x[1], reverse=True)[:10],
            'recent_imports': [
                {
                    'id': doc.id,
                    'title': doc.title,
                    'filename': doc.document_metadata.get('org_filename'),
                    'imported_at': doc.created_at.isoformat(),
                    'language': doc.document_metadata.get('language')
                }
                for doc in recent_imports
            ]
        }
        
        return jsonify({'statistics': statistics})
        
    except Exception as e:
        current_app.logger.error(f"Failed to get org-roam statistics: {str(e)}")
        return jsonify({'error': 'Failed to get statistics', 'details': str(e)}), 500