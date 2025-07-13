from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required, get_jwt_identity, jwt_required
from app import db
from app.models.document import Document
from app.models.user import User
from app.models.tag import Tag
from sqlalchemy import or_
import bleach
import os
import re
from datetime import datetime
from app.utils.auto_tag import detect_auto_tags, merge_tags
from app.utils.obsidian_parser import ObsidianParser
from app.utils.backup_manager import create_document_backup, update_document_backup, upload_document_backup, export_all_documents

documents_bp = Blueprint('documents', __name__)

def process_obsidian_content(markdown_content):
    """Process Obsidian-style content and extract metadata"""
    parser = ObsidianParser()
    parsed = parser.parse_markdown(markdown_content)
    
    # Extract all tags (frontmatter + hashtags)
    all_tags = set()
    
    # Tags from frontmatter
    if 'tags' in parsed.get('frontmatter', {}):
        frontmatter_tags = parsed['frontmatter']['tags']
        if isinstance(frontmatter_tags, list):
            all_tags.update(frontmatter_tags)
        elif isinstance(frontmatter_tags, str):
            all_tags.update(tag.strip() for tag in frontmatter_tags.split(','))
    
    # Tags from hashtags
    for hashtag in parsed.get('hashtags', []):
        all_tags.add(hashtag.get('tag', ''))
    
    return {
        'frontmatter': parsed.get('frontmatter', {}),
        'internal_links': parsed.get('internal_links', []),
        'hashtags': parsed.get('hashtags', []),
        'all_tags': list(filter(None, all_tags))
    }

def get_current_user_id():
    try:
        return get_jwt_identity()
    except:
        return None

@documents_bp.route('/documents', methods=['POST'])
@jwt_required(optional=True)
def create_document():
    try:
        data = request.get_json()
        
        if not data or 'title' not in data or 'markdown_content' not in data:
            return jsonify({'error': 'Title and markdown_content are required'}), 400
        
        current_user_id = get_current_user_id()
        
        # Sanitize input to prevent XSS
        title = bleach.clean(data['title'].strip())
        author = bleach.clean(data.get('author', '').strip()) if data.get('author') else None
        is_public = data.get('is_public', True)
        tags = data.get('tags', [])
        
        if not title:
            return jsonify({'error': 'Title cannot be empty'}), 400
        
        # 옵시디언 스타일 콘텐츠 처리
        try:
            obsidian_data = process_obsidian_content(data['markdown_content'])
        except Exception as e:
            print(f"Error processing Obsidian content during creation: {e}")
            # 옵시디언 처리 실패시 기본값 설정
            obsidian_data = {
                'frontmatter': {},
                'internal_links': [],
                'hashtags': [],
                'all_tags': []
            }
        
        # 프론트매터에서 제목 오버라이드 (옵션)
        if 'title' in obsidian_data['frontmatter'] and not title:
            title = obsidian_data['frontmatter']['title']
        
        document = Document(
            title=title,
            markdown_content=data['markdown_content'],  # 원본 마크다운 유지
            author=author,
            user_id=current_user_id,
            is_public=is_public,
            document_metadata={
                'frontmatter': obsidian_data['frontmatter'],
                'internal_links': obsidian_data['internal_links'],
                'hashtags': obsidian_data['hashtags']
            }
        )
        
        # 옵시디언 태그 + 자동 감지 태그 + 사용자 제공 태그 결합
        auto_tags = detect_auto_tags(data['markdown_content'])
        obsidian_tags = obsidian_data.get('all_tags', [])
        all_tags = merge_tags(merge_tags(tags, auto_tags), obsidian_tags)
        
        # Add tags if any exist
        if all_tags:
            document.add_tags(all_tags)
        
        db.session.add(document)
        db.session.commit()
        
        # 문서 백업 생성
        try:
            backup_path = create_document_backup(document)
            if backup_path:
                print(f"Document backup created: {backup_path}")
            else:
                print(f"Failed to create backup for document {document.id}")
        except Exception as backup_error:
            print(f"Backup creation error for document {document.id}: {backup_error}")
            # 백업 실패가 문서 생성을 막지 않도록 함
        
        return jsonify(document.to_dict()), 201
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents', methods=['GET'])
@jwt_required(optional=True)
def list_documents():
    try:
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)
        search = request.args.get('search', '')
        include_private = request.args.get('include_private', 'false').lower() == 'true'
        
        current_user_id = get_current_user_id()
        
        tags_filter = request.args.getlist('tags')  # Support multiple tags
        
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
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/sync', methods=['POST'])
@jwt_required(optional=True)
def sync_backup_files():
    """백업 파일과 DB 동기화"""
    try:
        from app.utils.backup_sync import sync_manager
        
        current_user_id = get_current_user_id()
        
        data = request.get_json() or {}
        dry_run = data.get('dry_run', False)
        
        # 전체 동기화 수행
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
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/sync/preview', methods=['GET'])
@jwt_required(optional=True)
def preview_backup_sync():
    """백업 파일 동기화 미리보기"""
    try:
        from app.utils.backup_sync import sync_manager
        
        current_user_id = get_current_user_id()
        
        # 드라이런으로 동기화 미리보기
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
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/sync/files', methods=['GET'])
@jwt_required(optional=True)
def list_backup_files_for_sync():
    """동기화 가능한 백업 파일 목록"""
    try:
        from app.utils.backup_sync import sync_manager
        
        current_user_id = get_current_user_id()
        
        backup_files = sync_manager.scan_backup_files()
        
        # 각 파일에 대한 동기화 상태 정보 추가
        file_info = []
        for backup_info in backup_files:
            existing_doc = sync_manager.find_matching_document(backup_info)
            
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
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['GET'])
@jwt_required(optional=True)
def get_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_view(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        return jsonify(document.to_dict())
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['PUT'])
@jwt_required(optional=True)
def update_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        data = request.get_json()
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
        # Update fields if provided
        if 'title' in data:
            title = bleach.clean(data['title'].strip())
            if not title:
                return jsonify({'error': 'Title cannot be empty'}), 400
            document.title = title
        
        if 'markdown_content' in data:
            document.markdown_content = data['markdown_content']
            
            # Process Obsidian content for updated content
            try:
                obsidian_data = process_obsidian_content(data['markdown_content'])
                document.document_metadata = {
                    'frontmatter': obsidian_data['frontmatter'],
                    'internal_links': obsidian_data['internal_links'],
                    'hashtags': obsidian_data['hashtags']
                }
                
                # Update tags if content changed
                auto_tags = detect_auto_tags(data['markdown_content'])
                obsidian_tags = obsidian_data.get('all_tags', [])
                existing_user_tags = [tag.name for tag in document.tags if not tag.is_auto_tag]
                all_tags = merge_tags(merge_tags(existing_user_tags, auto_tags), obsidian_tags)
                
                # Clear existing tags and add updated ones
                for tag in document.tags.all():
                    document.tags.remove(tag)
                if all_tags:
                    document.add_tags(all_tags)
                    
            except Exception as e:
                print(f"Error processing Obsidian content during update: {e}")
        
        if 'author' in data:
            author = bleach.clean(data['author'].strip()) if data['author'] else None
            document.author = author
        
        if 'is_public' in data:
            document.is_public = data['is_public']
        
        if 'tags' in data:
            # Handle manual tag updates
            for tag in document.tags.all():
                document.tags.remove(tag)
            if data['tags']:
                document.add_tags(data['tags'])
        
        document.updated_at = datetime.utcnow()
        db.session.commit()
        
        # Create backup for updated document
        try:
            backup_path = update_document_backup(document)
            if backup_path:
                print(f"Document backup updated: {backup_path}")
        except Exception as backup_error:
            print(f"Backup update error for document {document.id}: {backup_error}")
        
        return jsonify(document.to_dict())
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/<int:document_id>', methods=['DELETE'])
@jwt_required(optional=True)
def delete_document(document_id):
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_current_user_id()
        
        if not document.can_edit(current_user_id):
            return jsonify({'error': 'Access denied'}), 403
        
        # Clear tags relationship before deletion
        # Since tags is a dynamic relationship, we need to handle it properly
        from app.models.document import document_tags
        db.session.execute(
            document_tags.delete().where(document_tags.c.document_id == document.id)
        )
        
        db.session.delete(document)
        db.session.commit()
        
        return jsonify({'message': 'Document deleted successfully'}), 200
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/upload', methods=['POST'])
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
        
        # Read file content
        content = file.read().decode('utf-8')
        
        # Extract title from filename (remove .md extension)
        title = file.filename[:-3] if file.filename.endswith('.md') else file.filename
        
        # Process Obsidian content
        try:
            obsidian_data = process_obsidian_content(content)
        except Exception as e:
            print(f"Error processing Obsidian content during upload: {e}")
            obsidian_data = {
                'frontmatter': {},
                'internal_links': [],
                'hashtags': [],
                'all_tags': []
            }
        
        # Override title if specified in frontmatter
        if 'title' in obsidian_data['frontmatter']:
            title = obsidian_data['frontmatter']['title']
        
        # Create document
        document = Document(
            title=title,
            markdown_content=content,
            author=obsidian_data['frontmatter'].get('author'),
            user_id=current_user_id,
            is_public=obsidian_data['frontmatter'].get('public', True),
            document_metadata={
                'frontmatter': obsidian_data['frontmatter'],
                'internal_links': obsidian_data['internal_links'],
                'hashtags': obsidian_data['hashtags']
            }
        )
        
        # Add tags
        auto_tags = detect_auto_tags(content)
        obsidian_tags = obsidian_data.get('all_tags', [])
        all_tags = merge_tags(auto_tags, obsidian_tags)
        
        if all_tags:
            document.add_tags(all_tags)
        
        db.session.add(document)
        db.session.commit()
        
        # Create backup
        try:
            backup_path = upload_document_backup(document)
            if backup_path:
                print(f"Document backup created: {backup_path}")
        except Exception as backup_error:
            print(f"Backup creation error for uploaded document {document.id}: {backup_error}")
        
        return jsonify({
            'message': 'File uploaded successfully',
            'document': document.to_dict()
        }), 201
        
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/export', methods=['POST'])
@jwt_required(optional=True)
def export_all_documents_to_backup():
    """모든 문서를 백업 폴더로 내보내기"""
    try:
        data = request.get_json() or {}
        use_short_filename = data.get('short_filename', False)
        
        # 전체 문서 내보내기 실행
        results = export_all_documents(use_short_filename=use_short_filename)
        
        return jsonify({
            'message': f'Export completed: {results["exported"]} documents exported',
            'results': results
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/by-date', methods=['GET'])
@jwt_required(optional=True)
def get_documents_by_date():
    """날짜별 문서 조회"""
    try:
        current_user_id = get_current_user_id()
        date_key = request.args.get('date_key')
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 50, type=int)
        
        if not date_key:
            return jsonify({'error': 'date_key parameter is required'}), 400
        
        # date_key 파싱 (YYYY, YYYY-MM, YYYY-MM-DD 형식 지원)
        date_parts = date_key.split('-')
        if len(date_parts) < 1:
            return jsonify({'error': 'Invalid date_key format'}), 400
        
        try:
            year = int(date_parts[0])
            month = int(date_parts[1]) if len(date_parts) > 1 else None
            day = int(date_parts[2]) if len(date_parts) > 2 else None
        except ValueError:
            return jsonify({'error': 'Invalid date_key format'}), 400
        
        # 쿼리 조건 구성
        if current_user_id:
            base_query = Document.query.filter(
                or_(Document.user_id == current_user_id, Document.is_public == True)
            )
        else:
            base_query = Document.query.filter_by(is_public=True)
        
        # 날짜 필터링
        from sqlalchemy import extract, and_
        filters = [extract('year', Document.created_at) == year]
        
        if month is not None:
            filters.append(extract('month', Document.created_at) == month)
        
        if day is not None:
            filters.append(extract('day', Document.created_at) == day)
        
        query = base_query.filter(and_(*filters))
        
        # 페이지네이션 적용
        pagination = query.order_by(Document.created_at.desc()).paginate(
            page=page, per_page=per_page, error_out=False
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
            'date_key': date_key
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/timeline', methods=['GET'])
@jwt_required(optional=True)
def get_documents_timeline():
    """문서 타임라인 데이터 가져오기"""
    try:
        current_user_id = get_current_user_id()
        group_by = request.args.get('group_by', 'month')  # 'month', 'year', 'day'
        
        # 사용자가 볼 수 있는 문서들 가져오기
        if current_user_id:
            # 로그인한 사용자: 자신의 문서 + 공개 문서
            documents = Document.query.filter(
                or_(Document.user_id == current_user_id, Document.is_public == True)
            ).order_by(Document.created_at.desc()).all()
        else:
            # 비로그인 사용자: 공개 문서만
            documents = Document.query.filter_by(is_public=True).order_by(Document.created_at.desc()).all()
        
        # 타임라인 데이터 구성
        timeline = {}
        
        for doc in documents:
            created_at = doc.created_at
            if not created_at:
                continue
                
            if group_by == 'month':
                # 연도 레벨
                year_key = str(created_at.year)
                year_label = f"{created_at.year}년"
                
                # 월 레벨
                month_key = f"{created_at.year}-{created_at.month:02d}"
                month_label = f"{created_at.month}월"
                
                # 연도 항목 초기화
                if year_key not in timeline:
                    timeline[year_key] = {
                        'key': year_key,
                        'label': year_label,
                        'count': 0,
                        'children': {}
                    }
                
                # 월 항목 초기화
                if month_key not in timeline[year_key]['children']:
                    timeline[year_key]['children'][month_key] = {
                        'key': month_key,
                        'label': month_label,
                        'count': 0
                    }
                
                # 카운트 증가
                timeline[year_key]['count'] += 1
                timeline[year_key]['children'][month_key]['count'] += 1
                
            elif group_by == 'year':
                # 연도별만
                year_key = str(created_at.year)
                year_label = f"{created_at.year}년"
                
                if year_key not in timeline:
                    timeline[year_key] = {
                        'key': year_key,
                        'label': year_label,
                        'count': 0
                    }
                
                timeline[year_key]['count'] += 1
                
            elif group_by == 'day':
                # 연도 > 월 > 일
                year_key = str(created_at.year)
                year_label = f"{created_at.year}년"
                
                month_key = f"{created_at.year}-{created_at.month:02d}"
                month_label = f"{created_at.month}월"
                
                day_key = f"{created_at.year}-{created_at.month:02d}-{created_at.day:02d}"
                day_label = f"{created_at.day}일"
                
                # 연도 초기화
                if year_key not in timeline:
                    timeline[year_key] = {
                        'key': year_key,
                        'label': year_label,
                        'count': 0,
                        'children': {}
                    }
                
                # 월 초기화
                if month_key not in timeline[year_key]['children']:
                    timeline[year_key]['children'][month_key] = {
                        'key': month_key,
                        'label': month_label,
                        'count': 0,
                        'children': {}
                    }
                
                # 일 초기화
                if day_key not in timeline[year_key]['children'][month_key]['children']:
                    timeline[year_key]['children'][month_key]['children'][day_key] = {
                        'key': day_key,
                        'label': day_label,
                        'count': 0
                    }
                
                # 카운트 증가
                timeline[year_key]['count'] += 1
                timeline[year_key]['children'][month_key]['count'] += 1
                timeline[year_key]['children'][month_key]['children'][day_key]['count'] += 1
        
        return jsonify({
            'timeline': timeline,
            'group_by': group_by,
            'total_documents': len(documents)
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

