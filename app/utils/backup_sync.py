import re
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Optional, Any
import logging
from app.models.document import Document
from app.utils.backup_manager import backup_manager
from app.utils.obsidian_parser import ObsidianParser
from app.utils.auto_tag import detect_auto_tags
from app import db

logger = logging.getLogger(__name__)

def extract_author_from_frontmatter(frontmatter):
    """Extract author from frontmatter, handling various formats"""
    if not frontmatter:
        return None
    
    author = frontmatter.get('author')
    if not author:
        return None
    
    # Handle different author formats
    if isinstance(author, list):
        # If author is a list, take the first item
        if len(author) > 0:
            author = author[0]
        else:
            return None
    
    # If it's a string, clean it up
    if isinstance(author, str):
        # Remove Obsidian-style wiki links: [[name]] -> name
        author = author.strip()
        if author.startswith('[[') and author.endswith(']]'):
            author = author[2:-2]
        # Remove quotes if present
        author = author.strip('"\'')
        return author if author else None
    
    return None

class BackupSyncManager:
    """백업 파일과 DB 동기화 관리 클래스"""
    
    def __init__(self):
        self.obsidian_parser = ObsidianParser()
        self.backup_dir = backup_manager.backup_root_dir
    
    def _try_pattern_with_time(self, filename: str) -> Optional[Dict]:
        """Try parsing YYYYMMDD_title_HHMMSS.md pattern"""
        pattern = r'^(\d{8})_(.+)_(\d{6})\.md$'
        match = re.match(pattern, filename)

        if match:
            date_str, title_part, time_str = match.groups()
            try:
                date_obj = datetime.strptime(date_str + time_str, '%Y%m%d%H%M%S')
                return {
                    'filename': filename,
                    'date': date_obj,
                    'title_part': title_part,
                    'original_date_str': date_str,
                    'original_time_str': time_str
                }
            except ValueError:
                pass
        return None

    def _try_pattern_date_only(self, filename: str) -> Optional[Dict]:
        """Try parsing YYYYMMDD_title.md pattern"""
        pattern = r'^(\d{8})_(.+)\.md$'
        match = re.match(pattern, filename)

        if match:
            date_str, title_part = match.groups()
            try:
                date_obj = datetime.strptime(date_str, '%Y%m%d')
                return {
                    'filename': filename,
                    'date': date_obj,
                    'title_part': title_part,
                    'original_date_str': date_str,
                    'original_time_str': '000000'
                }
            except ValueError:
                pass
        return None

    def _try_pattern_hyphenated(self, filename: str) -> Optional[Dict]:
        """Try parsing YYYY-MM-DD_title.md pattern"""
        pattern = r'^(\d{4}-\d{2}-\d{2})_(.+)\.md$'
        match = re.match(pattern, filename)

        if match:
            date_str, title_part = match.groups()
            try:
                date_obj = datetime.strptime(date_str, '%Y-%m-%d')
                return {
                    'filename': filename,
                    'date': date_obj,
                    'title_part': title_part,
                    'original_date_str': date_str.replace('-', ''),
                    'original_time_str': '000000'
                }
            except ValueError:
                pass
        return None

    def _try_pattern_generic(self, filename: str) -> Optional[Dict]:
        """Try parsing generic markdown file using file modification time"""
        title_part = filename[:-3]
        file_path = self.backup_dir / filename

        if file_path.exists():
            file_mtime = datetime.fromtimestamp(file_path.stat().st_mtime)
            return {
                'filename': filename,
                'date': file_mtime,
                'title_part': title_part,
                'original_date_str': file_mtime.strftime('%Y%m%d'),
                'original_time_str': file_mtime.strftime('%H%M%S')
            }
        return None

    def parse_backup_filename(self, filename: str) -> Optional[Dict]:
        """백업 파일명에서 정보 추출: 다양한 패턴 지원"""
        try:
            if not filename.lower().endswith('.md'):
                return None

            result = (
                self._try_pattern_with_time(filename) or
                self._try_pattern_date_only(filename) or
                self._try_pattern_hyphenated(filename) or
                self._try_pattern_generic(filename)
            )

            if not result:
                logger.warning(f"Could not parse filename: {filename}")

            return result

        except Exception as e:
            logger.error(f"Failed to parse backup filename {filename}: {e}")
            return None
    
    def extract_document_info_from_backup(self, file_path: Path) -> Optional[Dict]:
        """백업 파일에서 문서 정보 추출"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            # 백업 파일 헤더에서 메타데이터 추출
            header_info = self._parse_backup_header(content)
            
            # 실제 마크다운 콘텐츠 추출 (헤더 제거)
            markdown_content = self._extract_markdown_content(content)
            
            # 옵시디언 파싱
            parsed_content = self.obsidian_parser.parse_markdown(markdown_content)
            
            # 파일 수정 시간
            file_mtime = datetime.fromtimestamp(file_path.stat().st_mtime)
            
            # Extract author from header or frontmatter
            author = header_info.get('author')
            if not author:
                author = extract_author_from_frontmatter(parsed_content.get('frontmatter', {}))
            
            return {
                'file_path': str(file_path),
                'file_mtime': file_mtime,
                'header_info': header_info,
                'markdown_content': markdown_content,
                'parsed_content': parsed_content,
                'title': header_info.get('title') or parsed_content.get('frontmatter', {}).get('title') or 'Untitled',
                'author': author,
                'original_doc_id': header_info.get('document_id'),
                'tags': self._extract_all_tags(parsed_content)
            }
            
        except Exception as e:
            logger.error(f"Failed to extract info from backup file {file_path}: {e}")
            return None
    
    def _parse_backup_header(self, content: str) -> Dict[str, Any]:
        """백업 파일 헤더에서 메타데이터 파싱"""
        header_info: Dict[str, Any] = {}

        try:
            # 헤더 부분 추출 (첫 번째 --- 블록)
            header_pattern = r'^---\s*\n(.*?)\n---\s*\n'
            header_match = re.match(header_pattern, content, re.DOTALL)

            if not header_match:
                return header_info

            header_content = header_match.group(1)

            # 각 라인에서 정보 추출
            for line in header_content.split('\n'):
                self._parse_header_line(line, header_info)

        except Exception as e:
            logger.warning(f"Failed to parse backup header: {e}")

        return header_info

    def _parse_header_line(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse a single header line and update header_info"""
        if line.startswith('# Document ID:'):
            self._parse_document_id(line, header_info)
        elif line.startswith('# Title:'):
            header_info['title'] = line.split(':', 1)[1].strip()
        elif line.startswith('# Author:'):
            self._parse_author(line, header_info)
        elif line.startswith('# Created:'):
            self._parse_created_date(line, header_info)
        elif line.startswith('# Tags:'):
            self._parse_tags(line, header_info)
        elif line.startswith('# Public:'):
            self._parse_is_public(line, header_info)

    def _parse_document_id(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse document ID from header line"""
        try:
            header_info['document_id'] = int(line.split(':', 1)[1].strip())
        except ValueError:
            pass

    def _parse_author(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse author from header line"""
        author = line.split(':', 1)[1].strip()
        if author != 'Unknown':
            header_info['author'] = author

    def _parse_created_date(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse created date from header line"""
        try:
            created_str = line.split(':', 1)[1].strip()
            header_info['created'] = datetime.fromisoformat(created_str.replace('Z', '+00:00'))
        except ValueError:
            pass

    def _parse_tags(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse tags from header line"""
        tags_str = line.split(':', 1)[1].strip()
        if tags_str:
            header_info['tags'] = [tag.strip() for tag in tags_str.split(',')]

    def _parse_is_public(self, line: str, header_info: Dict[str, Any]) -> None:
        """Parse is_public flag from header line"""
        is_public_str = line.split(':', 1)[1].strip().lower()
        header_info['is_public'] = is_public_str == 'true'
    
    def _extract_markdown_content(self, content: str) -> str:
        """백업 파일에서 실제 마크다운 콘텐츠 추출 (헤더 제거)"""
        try:
            # 첫 번째 --- 블록 이후의 내용 추출
            header_pattern = r'^---\s*\n.*?\n---\s*\n'
            clean_content = re.sub(header_pattern, '', content, flags=re.DOTALL)
            return clean_content.strip()
        except Exception as e:
            logger.warning(f"Failed to extract markdown content: {e}")
            return content
    
    def _extract_all_tags(self, parsed_content: Dict) -> List[str]:
        """파싱된 콘텐츠에서 모든 태그 추출"""
        all_tags = set()
        
        # 프론트매터 태그
        frontmatter = parsed_content.get('frontmatter', {})
        if 'tags' in frontmatter:
            tags_value = frontmatter['tags']
            if isinstance(tags_value, list):
                all_tags.update(tags_value)
            elif isinstance(tags_value, str):
                all_tags.update(tag.strip() for tag in tags_value.split(','))
        
        # 해시태그
        hashtags = parsed_content.get('hashtags', [])
        for hashtag in hashtags:
            all_tags.add(hashtag.get('tag', ''))
        
        # 자동 감지 태그
        content = parsed_content.get('clean_content', '')
        if content:
            auto_tags = detect_auto_tags(content)
            all_tags.update(auto_tags)
        
        # Filter out unwanted automatic tags
        filtered_tags = [tag for tag in all_tags if tag and tag.lower() != 'clippings']
        
        return filtered_tags
    
    def find_matching_document(self, backup_info: Dict) -> Optional[Document]:
        """백업 파일과 매칭되는 DB 문서 찾기"""
        try:
            # 1. 원본 문서 ID로 찾기 (백업 헤더에 기록된 경우)
            original_doc_id = backup_info['header_info'].get('document_id')
            if original_doc_id:
                doc: Optional[Document] = db.session.get(Document, original_doc_id)
                if doc:
                    return doc

            # 2. 제목으로 찾기
            title = backup_info['title']
            docs: List[Document] = Document.query.filter_by(title=title).all()
            if len(docs) == 1:
                return docs[0]
            elif len(docs) > 1:
                # 여러 개 있으면 가장 최근 것
                result: Document = max(docs, key=lambda d: d.updated_at or d.created_at)
                return result

            # 3. 콘텐츠 유사성으로 찾기 (간단한 해시 비교)
            content_hash = hash(backup_info['markdown_content'][:500])  # 첫 500자로 해시
            all_docs: List[Document] = Document.query.all()
            for doc in all_docs:
                if doc.markdown_content:
                    doc_hash = hash(doc.markdown_content[:500])
                    if doc_hash == content_hash:
                        return doc

            return None
            
        except Exception as e:
            logger.error(f"Failed to find matching document: {e}")
            return None
    
    def compare_document_versions(self, document: Document, backup_info: Dict) -> Dict:
        """문서와 백업 파일 비교"""
        try:
            db_updated = document.updated_at or document.created_at
            file_updated = backup_info['file_mtime']
            
            # 콘텐츠 비교
            db_content = document.markdown_content or ''
            backup_content = backup_info['markdown_content']
            content_different = db_content.strip() != backup_content.strip()
            
            # 제목 비교
            title_different = document.title != backup_info['title']
            
            return {
                'db_newer': db_updated > file_updated,
                'file_newer': file_updated > db_updated,
                'content_different': content_different,
                'title_different': title_different,
                'db_updated': db_updated,
                'file_updated': file_updated,
                'recommendation': self._get_sync_recommendation(db_updated, file_updated, content_different)
            }
            
        except Exception as e:
            logger.error(f"Failed to compare document versions: {e}")
            return {'recommendation': 'skip'}
    
    def _get_sync_recommendation(self, db_time: datetime, file_time: datetime, content_different: bool) -> str:
        """동기화 권장 사항 결정"""
        if not content_different:
            return 'no_change'
        
        time_diff = abs((db_time - file_time).total_seconds())
        
        # 시간 차이가 1분 이내면 충돌로 간주
        if time_diff < 60:
            return 'conflict'
        elif file_time > db_time:
            return 'update_db'
        else:
            return 'update_file'
    
    def _generate_ai_tags(self, content: str, title: str) -> List[str]:
        """Generate AI tags for content"""
        try:
            from app.services.ai_service import ai_service
            ai_tags = ai_service.suggest_tags(content, title)
            logger.info(f"AI generated tags for document '{title}': {ai_tags}")
            return ai_tags
        except Exception as e:
            logger.warning(f"Failed to generate AI tags: {e}")
            return []

    def _update_document_from_backup(self, document: Document, backup_info: Dict) -> None:
        """Update document with backup content and metadata"""
        document.title = backup_info['title']
        document.markdown_content = backup_info['markdown_content']
        if backup_info['author']:
            document.author = backup_info['author']

        if backup_info['parsed_content']:
            document.document_metadata = {
                'frontmatter': backup_info['parsed_content'].get('frontmatter', {}),
                'internal_links': backup_info['parsed_content'].get('internal_links', []),
                'hashtags': backup_info['parsed_content'].get('hashtags', [])
            }

        existing_tags = backup_info['tags'] or []
        ai_tags = self._generate_ai_tags(backup_info['markdown_content'], backup_info['title'])
        all_tags = list(set(existing_tags + ai_tags))

        for tag in document.tags.all():
            document.tags.remove(tag)

        if all_tags:
            document.add_tags(all_tags)

    def _create_new_backup(self, document: Document) -> str:
        """Create new backup file from document"""
        from app.utils.backup_manager import create_document_backup
        return create_document_backup(document, force=True)

    def _prepare_sync_result(self, action: str, document: Document, backup_info: Dict) -> Dict:
        """Prepare base sync result structure"""
        return {
            'action': action,
            'document_id': document.id,
            'backup_file': backup_info['file_path'],
            'success': False
        }

    def sync_document_from_backup(self, document: Document, backup_info: Dict, force_direction: str = 'auto') -> Dict:
        """백업 파일과 문서 동기화"""
        try:
            comparison = self.compare_document_versions(document, backup_info)
            action = comparison['recommendation'] if force_direction == 'auto' else force_direction
            result = self._prepare_sync_result(action, document, backup_info)

            if action == 'no_change':
                result['success'] = True
                result['message'] = 'No changes needed'

            elif action == 'update_db':
                self._update_document_from_backup(document, backup_info)
                db.session.commit()
                result['success'] = True
                result['message'] = 'Database updated from backup file'

            elif action == 'update_file':
                new_backup_path = self._create_new_backup(document)
                result['success'] = True
                result['message'] = f'New backup created: {new_backup_path}'
                result['new_backup'] = new_backup_path

            elif action == 'conflict':
                result['message'] = 'Conflict detected - manual resolution required'
                result['conflict_info'] = comparison

            else:
                result['message'] = 'Sync skipped'

            return result

        except Exception as e:
            db.session.rollback()
            logger.error(f"Failed to sync document {document.id}: {e}")
            return {
                'action': 'error',
                'success': False,
                'message': f'Sync failed: {str(e)}',
                'document_id': document.id,
                'backup_file': backup_info['file_path']
            }
    
    def create_document_from_backup(self, backup_info: Dict, user_id: Optional[int] = None) -> Dict:
        """백업 파일에서 새 문서 생성"""
        try:
            document = Document(
                title=backup_info['title'],
                markdown_content=backup_info['markdown_content'],
                author=backup_info['author'],
                user_id=user_id,
                is_public=backup_info['header_info'].get('is_public', True),
                document_metadata={
                    'frontmatter': backup_info['parsed_content'].get('frontmatter', {}),
                    'internal_links': backup_info['parsed_content'].get('internal_links', []),
                    'hashtags': backup_info['parsed_content'].get('hashtags', [])
                }
            )
            
            db.session.add(document)
            db.session.flush()  # ID 할당을 위해
            
            # 태그 추가 (기존 태그 + AI 생성 태그)
            existing_tags = backup_info['tags'] or []
            
            # AI 태그 자동 생성
            ai_tags = []
            try:
                from app.services.ai_service import ai_service
                ai_tags = ai_service.suggest_tags(backup_info['markdown_content'], backup_info['title'])
                logger.info(f"AI generated tags for imported document '{backup_info['title']}': {ai_tags}")
            except Exception as e:
                logger.warning(f"Failed to generate AI tags for imported document: {e}")
            
            # 기존 태그와 AI 태그 병합 (중복 제거)
            all_tags = list(set(existing_tags + ai_tags))
            
            if all_tags:
                document.add_tags(all_tags)
            
            db.session.commit()
            
            return {
                'action': 'create',
                'success': True,
                'document_id': document.id,
                'backup_file': backup_info['file_path'],
                'message': f'New document created from backup: {document.title}'
            }
            
        except Exception as e:
            db.session.rollback()
            logger.error(f"Failed to create document from backup: {e}")
            return {
                'action': 'create',
                'success': False,
                'backup_file': backup_info['file_path'],
                'message': f'Failed to create document: {str(e)}'
            }
    
    def scan_backup_files(self) -> List[Dict[str, Any]]:
        """백업 디렉토리 스캔하여 파일 정보 수집"""
        backup_files: List[Dict[str, Any]] = []
        
        try:
            if not self.backup_dir.exists():
                logger.warning("Backup directory does not exist")
                return backup_files
            
            # Use recursive glob to scan all subdirectories
            for file_path in self.backup_dir.rglob("*.md"):
                filename_info = self.parse_backup_filename(file_path.name)
                if filename_info:
                    backup_info = self.extract_document_info_from_backup(file_path)
                    if backup_info:
                        backup_info.update(filename_info)
                        backup_files.append(backup_info)
            
            # 파일 수정 시간 기준으로 정렬 (최신순)
            backup_files.sort(key=lambda x: x['file_mtime'], reverse=True)
            
        except Exception as e:
            logger.error(f"Failed to scan backup files: {e}")
        
        return backup_files
    
    def _initialize_sync_results(self, backup_files: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Initialize sync results structure"""
        return {
            'total_files': len(backup_files),
            'processed': 0,
            'created': 0,
            'updated': 0,
            'conflicts': 0,
            'errors': 0,
            'skipped': 0,
            'details': []
        }

    def _process_existing_document(self, existing_doc: Document, backup_info: Dict, dry_run: bool) -> Dict:
        """Process sync for existing document"""
        if not dry_run:
            return self.sync_document_from_backup(existing_doc, backup_info)

        comparison = self.compare_document_versions(existing_doc, backup_info)
        return {
            'action': comparison['recommendation'],
            'document_id': existing_doc.id,
            'backup_file': backup_info['file_path'],
            'success': True,
            'message': f'Would {comparison["recommendation"]} (dry run)'
        }

    def _process_new_document(self, backup_info: Dict, user_id: Optional[int], dry_run: bool) -> Dict:
        """Process creation of new document from backup"""
        if not dry_run:
            return self.create_document_from_backup(backup_info, user_id)

        return {
            'action': 'create',
            'success': True,
            'backup_file': backup_info['file_path'],
            'message': f'Would create new document: {backup_info["title"]} (dry run)'
        }

    def _update_result_counters(self, results: Dict[str, Any], sync_result: Dict) -> None:
        """Update result counters based on sync result"""
        if sync_result['success']:
            action = sync_result['action']
            if action == 'update_db':
                results['updated'] += 1
            elif action == 'conflict':
                results['conflicts'] += 1
            elif action == 'create':
                results['created'] += 1
            else:
                results['skipped'] += 1
        else:
            results['errors'] += 1

    def perform_full_sync(self, user_id: Optional[int] = None, dry_run: bool = False) -> Dict[str, Any]:
        """전체 백업 파일 동기화 수행"""
        backup_files = self.scan_backup_files()
        results = self._initialize_sync_results(backup_files)

        for backup_info in backup_files:
            try:
                results['processed'] += 1
                existing_doc = self.find_matching_document(backup_info)

                if existing_doc:
                    sync_result = self._process_existing_document(existing_doc, backup_info, dry_run)
                else:
                    sync_result = self._process_new_document(backup_info, user_id, dry_run)

                self._update_result_counters(results, sync_result)
                results['details'].append(sync_result)

            except Exception as e:
                results['errors'] += 1
                results['details'].append({
                    'action': 'error',
                    'success': False,
                    'backup_file': backup_info.get('file_path', 'unknown'),
                    'message': f'Processing error: {str(e)}'
                })
                logger.error(f"Error processing backup file: {e}")

        return results

# 전역 동기화 매니저 인스턴스
sync_manager = BackupSyncManager()