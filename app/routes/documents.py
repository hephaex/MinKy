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

documents_bp = Blueprint('documents', __name__)

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
        
        # Sanitize input
        title = bleach.clean(data.get('title', '').strip()) if data.get('title') else None
        author = bleach.clean(data.get('author', '').strip()) if data.get('author') else None
        
        document.update_content(
            title=title,
            markdown_content=data.get('markdown_content'),
            author=author,
            change_summary=data.get('change_summary'),
            updated_by=current_user_id
        )
        
        # Update visibility if provided and user owns the document
        if 'is_public' in data and document.user_id == current_user_id:
            document.is_public = bool(data['is_public'])
        
        # 옵시디언 스타일 콘텐츠 처리 및 메타데이터 업데이트
        obsidian_data = None
        if data.get('markdown_content'):
            try:
                obsidian_data = process_obsidian_content(data['markdown_content'])
                
                # 문서 메타데이터 업데이트
                document.document_metadata = {
                    'frontmatter': obsidian_data['frontmatter'],
                    'internal_links': obsidian_data['internal_links'],
                    'hashtags': obsidian_data['hashtags']
                }
            except Exception as e:
                print(f"Error processing Obsidian content: {e}")
                # 옵시디언 처리 실패시 기본값 설정
                obsidian_data = {
                    'frontmatter': {},
                    'internal_links': [],
                    'hashtags': [],
                    'all_tags': []
                }
        
        # Update tags if provided
        if 'tags' in data:
            # 옵시디언 태그 + 자동 감지 태그 + 사용자 제공 태그 결합
            auto_tags = []
            obsidian_tags = []
            if data.get('markdown_content'):
                auto_tags = detect_auto_tags(data['markdown_content'])
                if obsidian_data:
                    obsidian_tags = obsidian_data.get('all_tags', [])
            
            all_tags = merge_tags(merge_tags(data['tags'], auto_tags), obsidian_tags)
            
            # Clear existing tags and add new ones
            document.tags.clear()
            if all_tags:
                document.add_tags(all_tags)
        elif data.get('markdown_content'):
            # If content is updated but tags aren't specified, still process obsidian tags
            auto_tags = detect_auto_tags(data['markdown_content'])
            obsidian_tags = []
            if obsidian_data:
                obsidian_tags = obsidian_data.get('all_tags', [])
            
            # Get existing tag names
            existing_tag_names = [tag.name for tag in document.tags]
            all_tags = merge_tags(merge_tags(existing_tag_names, auto_tags), obsidian_tags)
            document.tags.clear()
            document.add_tags(all_tags)
        
        db.session.commit()
        
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
        
        db.session.delete(document)
        db.session.commit()
        
        return jsonify({'message': 'Document deleted successfully'})
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': str(e)}), 500

def extract_markdown_metadata(content):
    """Extract metadata from markdown frontmatter (legacy function)"""
    frontmatter_pattern = r'^---\s*\n(.*?)\n---\s*\n'
    match = re.match(frontmatter_pattern, content, re.DOTALL)
    
    if match:
        yaml_content = match.group(1)
        # Remove frontmatter from content
        content = re.sub(frontmatter_pattern, '', content, flags=re.DOTALL)
        
        # Parse simple YAML-like metadata
        metadata = {}
        for line in yaml_content.split('\n'):
            if ':' in line:
                key, value = line.split(':', 1)
                metadata[key.strip()] = value.strip().strip('"\'')
        
        return metadata, content
    
    return {}, content

def process_obsidian_content(content, document=None):
    """옵시디언 스타일 콘텐츠 처리"""
    parser = ObsidianParser()
    parsed = parser.parse_markdown(content)
    
    result = {
        'frontmatter': parsed['frontmatter'],
        'internal_links': parsed['internal_links'],
        'hashtags': parsed['hashtags'],
        'clean_content': parsed['clean_content']
    }
    
    # 프론트매터에서 태그 추출
    frontmatter_tags = []
    if 'tags' in parsed['frontmatter']:
        tags_value = parsed['frontmatter']['tags']
        if isinstance(tags_value, list):
            frontmatter_tags = tags_value
        elif isinstance(tags_value, str):
            frontmatter_tags = [tag.strip() for tag in tags_value.split(',')]
    
    # 해시태그와 프론트매터 태그 결합
    all_tag_names = set()
    all_tag_names.update(tag['tag'] for tag in parsed['hashtags'])
    all_tag_names.update(frontmatter_tags)
    
    result['all_tags'] = list(all_tag_names)
    
    return result

def document_title_lookup(title):
    """문서 제목으로 문서 ID 조회 (내부 링크용)"""
    try:
        doc = Document.query.filter_by(title=title).first()
        return doc.id if doc else None
    except:
        return None

def validate_markdown_file(file):
    """Validate uploaded markdown file"""
    if not file or not file.filename:
        return False, "No file provided"
    
    # Check file extension
    if not file.filename.lower().endswith('.md'):
        return False, "File must be a markdown (.md) file"
    
    # Check file size (max 10MB)
    file.seek(0, os.SEEK_END)
    file_size = file.tell()
    file.seek(0)
    
    if file_size > 10 * 1024 * 1024:  # 10MB
        return False, "File size must be less than 10MB"
    
    if file_size == 0:
        return False, "File cannot be empty"
    
    return True, None

@documents_bp.route('/documents/timeline', methods=['GET'])
def get_documents_timeline():
    """Get documents grouped by date hierarchy (year/month/week/day)"""
    try:
        from sqlalchemy import func, extract, text
        from datetime import datetime, timedelta
        
        user_id = get_current_user_id()
        group_by = request.args.get('group_by', 'month')  # year, month, week, day
        include_private = request.args.get('include_private', 'false').lower() == 'true'
        
        # Base query
        query = Document.query
        
        # Filter by access permissions
        if user_id and include_private:
            query = query.filter(
                (Document.is_public == True) | (Document.user_id == user_id)
            )
        else:
            query = query.filter(Document.is_public == True)
        
        timeline = {}
        
        if group_by == 'year':
            # Group by year
            results = query.with_entities(
                extract('year', Document.created_at).label('year'),
                func.count(Document.id).label('count')
            ).group_by('year').order_by('year').all()
            
            for year, count in results:
                timeline[str(int(year))] = {
                    'count': count,
                    'type': 'year',
                    'label': f"{int(year)}년",
                    'key': str(int(year))
                }
        
        elif group_by == 'month':
            # Group by year and month
            results = query.with_entities(
                extract('year', Document.created_at).label('year'),
                extract('month', Document.created_at).label('month'),
                func.count(Document.id).label('count')
            ).group_by('year', 'month').order_by('year', 'month').all()
            
            for year, month, count in results:
                year_key = str(int(year))
                month_key = f"{int(year)}-{int(month):02d}"
                
                if year_key not in timeline:
                    timeline[year_key] = {
                        'count': 0,
                        'type': 'year',
                        'label': f"{int(year)}년",
                        'key': year_key,
                        'children': {}
                    }
                
                timeline[year_key]['count'] += count
                timeline[year_key]['children'][month_key] = {
                    'count': count,
                    'type': 'month',
                    'label': f"{int(month)}월",
                    'key': month_key
                }
        
        elif group_by == 'week':
            # Group by year, month and week
            results = query.with_entities(
                extract('year', Document.created_at).label('year'),
                extract('month', Document.created_at).label('month'),
                extract('week', Document.created_at).label('week'),
                func.count(Document.id).label('count')
            ).group_by('year', 'month', 'week').order_by('year', 'month', 'week').all()
            
            for year, month, week, count in results:
                year_key = str(int(year))
                month_key = f"{int(year)}-{int(month):02d}"
                week_key = f"{int(year)}-W{int(week):02d}"
                
                if year_key not in timeline:
                    timeline[year_key] = {
                        'count': 0, 'type': 'year', 'label': f"{int(year)}년",
                        'key': year_key, 'children': {}
                    }
                
                if month_key not in timeline[year_key]['children']:
                    timeline[year_key]['children'][month_key] = {
                        'count': 0, 'type': 'month', 'label': f"{int(month)}월",
                        'key': month_key, 'children': {}
                    }
                
                timeline[year_key]['count'] += count
                timeline[year_key]['children'][month_key]['count'] += count
                timeline[year_key]['children'][month_key]['children'][week_key] = {
                    'count': count,
                    'type': 'week',
                    'label': f"주 {int(week)}",
                    'key': week_key
                }
        
        elif group_by == 'day':
            # Group by date
            results = query.with_entities(
                func.date(Document.created_at).label('date'),
                func.count(Document.id).label('count')
            ).group_by('date').order_by('date').all()
            
            for date, count in results:
                date_str = date.strftime('%Y-%m-%d')
                year = date.year
                month = date.month
                day = date.day
                
                year_key = str(year)
                month_key = f"{year}-{month:02d}"
                day_key = date_str
                
                if year_key not in timeline:
                    timeline[year_key] = {
                        'count': 0, 'type': 'year', 'label': f"{year}년",
                        'key': year_key, 'children': {}
                    }
                
                if month_key not in timeline[year_key]['children']:
                    timeline[year_key]['children'][month_key] = {
                        'count': 0, 'type': 'month', 'label': f"{month}월",
                        'key': month_key, 'children': {}
                    }
                
                timeline[year_key]['count'] += count
                timeline[year_key]['children'][month_key]['count'] += count
                timeline[year_key]['children'][month_key]['children'][day_key] = {
                    'count': count,
                    'type': 'day',
                    'label': f"{day}일",
                    'key': day_key
                }
        
        return jsonify({
            'timeline': timeline,
            'group_by': group_by
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/by-date', methods=['GET'])
def get_documents_by_date():
    """Get documents for a specific date/period"""
    try:
        user_id = get_current_user_id()
        date_key = request.args.get('date_key')  # e.g., "2025", "2025-07", "2025-07-12"
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        include_private = request.args.get('include_private', 'false').lower() == 'true'
        
        if not date_key:
            return jsonify({'error': 'date_key parameter required'}), 400
        
        # Base query
        query = Document.query
        
        # Filter by access permissions
        if user_id and include_private:
            query = query.filter(
                (Document.is_public == True) | (Document.user_id == user_id)
            )
        else:
            query = query.filter(Document.is_public == True)
        
        # Parse date_key and apply date filters
        from datetime import datetime, timedelta
        
        if len(date_key) == 4:  # Year: "2025"
            year = int(date_key)
            start_date = datetime(year, 1, 1)
            end_date = datetime(year + 1, 1, 1)
            
        elif len(date_key) == 7:  # Month: "2025-07"
            year, month = map(int, date_key.split('-'))
            start_date = datetime(year, month, 1)
            if month == 12:
                end_date = datetime(year + 1, 1, 1)
            else:
                end_date = datetime(year, month + 1, 1)
                
        elif len(date_key) == 10:  # Day: "2025-07-12"
            date_obj = datetime.strptime(date_key, '%Y-%m-%d')
            start_date = date_obj
            end_date = date_obj + timedelta(days=1)
            
        elif 'W' in date_key:  # Week: "2025-W28"
            year, week = date_key.split('-W')
            year = int(year)
            week = int(week)
            # Calculate week start date
            start_date = datetime.strptime(f'{year}-W{week:02d}-1', "%Y-W%W-%w")
            end_date = start_date + timedelta(days=7)
        else:
            return jsonify({'error': 'Invalid date_key format'}), 400
        
        # Apply date filter
        query = query.filter(
            Document.created_at >= start_date,
            Document.created_at < end_date
        )
        
        # Paginate results
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
            'date_range': {
                'start': start_date.isoformat(),
                'end': end_date.isoformat(),
                'key': date_key
            }
        })
    
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@documents_bp.route('/documents/upload', methods=['POST'])
@jwt_required(optional=True)
def upload_markdown_file():
    """Upload a markdown file and create a document"""
    try:
        if 'file' not in request.files:
            return jsonify({'error': 'No file provided'}), 400
        
        file = request.files['file']
        current_user_id = get_current_user_id()
        
        # Validate file
        is_valid, error_message = validate_markdown_file(file)
        if not is_valid:
            return jsonify({'error': error_message}), 400
        
        # Read file content
        try:
            content = file.read().decode('utf-8')
        except UnicodeDecodeError:
            return jsonify({'error': 'File must be UTF-8 encoded'}), 400
        
        # Extract metadata from frontmatter
        metadata, markdown_content = extract_markdown_metadata(content)
        
        # Get title from metadata or filename
        title = metadata.get('title', '')
        if not title:
            # Use filename without extension as title
            title = os.path.splitext(file.filename)[0]
        
        # Clean and validate title
        title = bleach.clean(title.strip())
        if not title:
            return jsonify({'error': 'Document title cannot be empty'}), 400
        
        # Get other metadata
        author = metadata.get('author', '')
        if author:
            author = bleach.clean(author.strip())
        
        # Get tags from metadata
        tags = []
        if 'tags' in metadata:
            tags_str = metadata['tags']
            if tags_str:
                # Parse tags (assume comma-separated or array-like format)
                tags_str = tags_str.strip('[]')
                tags = [tag.strip().strip('"\',') for tag in tags_str.split(',') if tag.strip()]
        
        # Get visibility setting
        is_public = True
        if 'public' in metadata:
            is_public = metadata['public'].lower() in ['true', 'yes', '1']
        elif 'private' in metadata:
            is_public = metadata['private'].lower() not in ['true', 'yes', '1']
        
        # Create document
        document = Document(
            title=title,
            markdown_content=markdown_content,
            author=author if author else None,
            user_id=current_user_id,
            is_public=is_public
        )
        
        # Detect automatic tags from content
        auto_tags = detect_auto_tags(markdown_content)
        
        # Merge provided tags with auto-detected tags
        all_tags = merge_tags(tags, auto_tags)
        
        # Add tags if any exist
        if all_tags:
            document.add_tags(all_tags)
        
        # Store additional metadata
        if metadata:
            # Remove processed metadata
            stored_metadata = {k: v for k, v in metadata.items() 
                             if k not in ['title', 'author', 'tags', 'public', 'private']}
            if stored_metadata:
                document.document_metadata = stored_metadata
        
        db.session.add(document)
        db.session.commit()
        
        return jsonify({
            'message': 'File uploaded successfully',
            'document': document.to_dict(),
            'metadata_found': bool(metadata)
        }), 201
    
    except Exception as e:
        db.session.rollback()
        return jsonify({'error': f'Upload failed: {str(e)}'}), 500