import os
import re
from datetime import datetime
from pathlib import Path
from typing import Optional
import logging
from .backup_config import backup_config

logger = logging.getLogger(__name__)

class DocumentBackupManager:
    """문서 백업 관리 클래스"""
    
    def __init__(self, backup_root_dir: str = "backup"):
        self.backup_root_dir = Path(backup_root_dir)
        self.ensure_backup_directory()
    
    def ensure_backup_directory(self):
        """백업 디렉토리가 존재하는지 확인하고 생성"""
        try:
            self.backup_root_dir.mkdir(parents=True, exist_ok=True)
            logger.info(f"Backup directory ensured: {self.backup_root_dir}")
        except Exception as e:
            logger.error(f"Failed to create backup directory: {e}")
            raise
    
    def sanitize_filename(self, title: str, max_length: int = 100) -> str:
        """파일명에 사용할 수 없는 문자를 제거하고 정리"""
        # 파일명에 사용할 수 없는 문자 제거
        sanitized = re.sub(r'[<>:"/\\|?*]', '_', title)
        
        # 연속된 공백이나 언더스코어를 하나로 변경
        sanitized = re.sub(r'[\s_]+', '_', sanitized)
        
        # 앞뒤 공백 및 언더스코어 제거
        sanitized = sanitized.strip('_').strip()
        
        # 길이 제한
        if len(sanitized) > max_length:
            sanitized = sanitized[:max_length]
        
        # 빈 제목인 경우 기본값 설정
        if not sanitized:
            sanitized = "untitled"
        
        return sanitized
    
    def generate_short_filename(self, document_title: str, created_at: Optional[datetime] = None) -> str:
        """짧은 백업 파일명 생성: YYYYMMDD_제목.md (12자 제한)"""
        if created_at is None:
            created_at = datetime.now()
        
        # 날짜 부분 생성 (8자: YYYYMMDD)
        date_str = created_at.strftime("%Y%m%d")
        
        # 제목 부분은 12자 - 8자(날짜) - 1자(_) - 3자(.md) = 0자
        # 실제로는 확장자를 제외하고 계산: 12자 - 8자(날짜) - 1자(_) = 3자
        max_title_length = 12 - len(date_str) - 1 - 3  # .md 확장자 제외
        if max_title_length < 1:
            max_title_length = 1
        
        # 제목을 짧게 만들기
        short_title = self.create_short_title(document_title, max_title_length)
        
        return f"{date_str}_{short_title}.md"
    
    def generate_medium_filename(self, document_title: str, created_at: Optional[datetime] = None) -> str:
        """중간 길이 백업 파일명 생성: YYYYMMDD_제목.md (64자 제한)"""
        if created_at is None:
            created_at = datetime.now()
        
        # 날짜 부분 생성 (8자: YYYYMMDD)
        date_str = created_at.strftime("%Y%m%d")
        
        # 제목 부분은 64자 - 8자(날짜) - 1자(_) - 3자(.md) = 52자
        max_title_length = 64 - len(date_str) - 1 - 3  # .md 확장자 제외
        
        # 제목을 적절한 길이로 만들기
        medium_title = self.create_medium_title(document_title, max_title_length)
        
        return f"{date_str}_{medium_title}.md"
    
    def create_short_title(self, title: str, max_length: int) -> str:
        """제목을 지정된 길이로 줄이기"""
        # 먼저 sanitize
        sanitized = self.sanitize_filename(title, max_length * 3)  # 여유분 두고 처리
        
        # 한글과 영문을 구분해서 처리
        if max_length <= 0:
            return "x"
        elif max_length == 1:
            return sanitized[0] if sanitized else "x"
        elif max_length == 2:
            return sanitized[:2] if len(sanitized) >= 2 else sanitized
        else:
            # 3자 이상인 경우
            if len(sanitized) <= max_length:
                return sanitized
            else:
                # 너무 긴 경우 앞부분만 취하기
                return sanitized[:max_length]
    
    def create_medium_title(self, title: str, max_length: int) -> str:
        """제목을 중간 길이로 만들기 (64자 제한용)"""
        # 먼저 sanitize
        sanitized = self.sanitize_filename(title, max_length * 2)  # 여유분 두고 처리
        
        # 길이가 제한보다 작거나 같으면 그대로 반환
        if len(sanitized) <= max_length:
            return sanitized
        else:
            # 너무 긴 경우 앞부분만 취하기
            return sanitized[:max_length]
    
    def generate_backup_filename(self, document_title: str, created_at: Optional[datetime] = None) -> str:
        """백업 파일명 생성: 제목.md"""
        # 제목 부분 생성
        sanitized_title = self.sanitize_filename(document_title)
        
        # 단순히 제목만 사용
        filename = f"{sanitized_title}.md"
        return filename
    
    def create_backup(self, document) -> Optional[str]:
        """문서 백업 생성"""
        # 백업 기능이 비활성화된 경우 건너뛰기
        if not backup_config.is_backup_enabled():
            logger.info("Backup is disabled in configuration")
            return None
            
        try:
            # 백업 파일명 생성
            filename = self.generate_backup_filename(
                document.title, 
                document.created_at
            )
            
            backup_path = self.backup_root_dir / filename
            
            # 백업 내용 생성
            backup_content = self.generate_backup_content(document)
            
            # 파일 크기 확인
            content_size_mb = len(backup_content.encode('utf-8')) / (1024 * 1024)
            max_size_mb = backup_config.get_max_backup_size_mb()
            
            if content_size_mb > max_size_mb:
                logger.warning(f"Backup content too large ({content_size_mb:.2f}MB > {max_size_mb}MB), skipping backup for document {document.id}")
                return None
            
            # 파일 저장
            with open(backup_path, 'w', encoding='utf-8') as f:
                f.write(backup_content)
            
            logger.info(f"Document backup created: {backup_path}")
            
            # 자동 정리 실행 (설정에 따라)
            if backup_config.is_auto_cleanup_enabled():
                self.auto_cleanup_if_needed()
            
            return str(backup_path)
            
        except Exception as e:
            logger.error(f"Failed to create backup for document {document.id}: {e}")
            return None
    
    def generate_backup_content(self, document) -> str:
        """백업 파일 내용 생성"""
        content_parts = []
        
        # 헤더 정보
        content_parts.append("---")
        content_parts.append("# Document Backup")
        content_parts.append(f"# Generated: {datetime.now().isoformat()}")
        content_parts.append(f"# Document ID: {document.id}")
        content_parts.append(f"# Title: {document.title}")
        content_parts.append(f"# Author: {document.author or 'Unknown'}")
        content_parts.append(f"# Created: {document.created_at.isoformat() if document.created_at else 'Unknown'}")
        content_parts.append(f"# Updated: {document.updated_at.isoformat() if document.updated_at else 'Unknown'}")
        content_parts.append(f"# Public: {document.is_public}")
        
        # 태그 정보
        if hasattr(document, 'tags') and document.tags:
            tag_names = [tag.name for tag in document.tags]
            content_parts.append(f"# Tags: {', '.join(tag_names)}")
        
        # 메타데이터 정보 (옵시디언 기능)
        if hasattr(document, 'document_metadata') and document.document_metadata:
            metadata = document.document_metadata
            if isinstance(metadata, dict):
                if metadata.get('frontmatter'):
                    content_parts.append(f"# Frontmatter: {metadata['frontmatter']}")
                if metadata.get('internal_links'):
                    links = [link.get('target', '') for link in metadata['internal_links']]
                    content_parts.append(f"# Internal Links: {', '.join(links)}")
                if metadata.get('hashtags'):
                    hashtags = [tag.get('tag', '') for tag in metadata['hashtags']]
                    content_parts.append(f"# Hashtags: {', '.join(hashtags)}")
        
        content_parts.append("---")
        content_parts.append("")
        
        # 실제 마크다운 내용
        if document.markdown_content:
            content_parts.append(document.markdown_content)
        else:
            content_parts.append("# " + document.title)
            content_parts.append("")
            content_parts.append("*No content available*")
        
        return "\n".join(content_parts)
    
    def generate_backup_content_for_comparison(self, document) -> str:
        """중복 비교용 백업 파일 내용 생성 (타임스탬프 제외)"""
        content_parts = []
        
        # 헤더 정보 (Generate 타임스탬프 제외)
        content_parts.append("---")
        content_parts.append("# Document Backup")
        content_parts.append(f"# Document ID: {document.id}")
        content_parts.append(f"# Title: {document.title}")
        content_parts.append(f"# Author: {document.author or 'Unknown'}")
        content_parts.append(f"# Created: {document.created_at.isoformat() if document.created_at else 'Unknown'}")
        content_parts.append(f"# Updated: {document.updated_at.isoformat() if document.updated_at else 'Unknown'}")
        content_parts.append(f"# Public: {document.is_public}")
        
        # 태그 정보
        if hasattr(document, 'tags') and document.tags:
            tag_names = [tag.name for tag in document.tags]
            content_parts.append(f"# Tags: {', '.join(tag_names)}")
        
        # 메타데이터 정보 (옵시디언 기능)
        if hasattr(document, 'document_metadata') and document.document_metadata:
            metadata = document.document_metadata
            if isinstance(metadata, dict):
                if metadata.get('frontmatter'):
                    content_parts.append(f"# Frontmatter: {metadata['frontmatter']}")
                if metadata.get('internal_links'):
                    links = [link.get('target', '') for link in metadata['internal_links']]
                    content_parts.append(f"# Internal Links: {', '.join(links)}")
                if metadata.get('hashtags'):
                    hashtags = [tag.get('tag', '') for tag in metadata['hashtags']]
                    content_parts.append(f"# Hashtags: {', '.join(hashtags)}")
        
        content_parts.append("---")
        content_parts.append("")
        
        # 실제 마크다운 내용
        if document.markdown_content:
            content_parts.append(document.markdown_content)
        else:
            content_parts.append("# " + document.title)
            content_parts.append("")
            content_parts.append("*No content available*")
        
        return "\n".join(content_parts)
    
    def update_backup(self, document) -> Optional[str]:
        """문서 업데이트시 새로운 백업 생성 (기존 백업은 유지)"""
        # 업데이트시에도 새로운 백업 파일 생성
        # 파일명에 현재 시간을 포함하여 이전 백업과 구분
        return self.create_backup(document)
    
    def delete_backup(self, backup_path: str) -> bool:
        """백업 파일 삭제"""
        try:
            Path(backup_path).unlink()
            logger.info(f"Backup deleted: {backup_path}")
            return True
        except Exception as e:
            logger.error(f"Failed to delete backup {backup_path}: {e}")
            return False
    
    def list_backups(self) -> list:
        """백업 파일 목록 조회"""
        try:
            backup_files = []
            for file_path in self.backup_root_dir.glob("*.md"):
                backup_files.append({
                    'filename': file_path.name,
                    'path': str(file_path),
                    'size': file_path.stat().st_size,
                    'created': datetime.fromtimestamp(file_path.stat().st_ctime).isoformat(),
                    'modified': datetime.fromtimestamp(file_path.stat().st_mtime).isoformat()
                })
            
            # 생성일 기준으로 정렬 (최신순)
            backup_files.sort(key=lambda x: x['created'], reverse=True)
            return backup_files
            
        except Exception as e:
            logger.error(f"Failed to list backups: {e}")
            return []
    
    def auto_cleanup_if_needed(self) -> int:
        """자동 정리가 필요한 경우 실행"""
        if not backup_config.is_auto_cleanup_enabled():
            return 0
        
        days_to_keep = backup_config.get_auto_cleanup_days()
        return self.cleanup_old_backups(days_to_keep)
    
    def cleanup_old_backups(self, days_to_keep: Optional[int] = None) -> int:
        """오래된 백업 파일 정리"""
        if days_to_keep is None:
            days_to_keep = backup_config.get_auto_cleanup_days()
            
        try:
            cutoff_date = datetime.now().timestamp() - (days_to_keep * 24 * 60 * 60)
            deleted_count = 0
            
            for file_path in self.backup_root_dir.glob("*.md"):
                if file_path.stat().st_ctime < cutoff_date:
                    file_path.unlink()
                    deleted_count += 1
                    logger.info(f"Old backup deleted: {file_path}")
            
            logger.info(f"Cleanup completed: {deleted_count} old backups deleted (keeping {days_to_keep} days)")
            return deleted_count
            
        except Exception as e:
            logger.error(f"Failed to cleanup old backups: {e}")
            return 0
    
    def cleanup_excess_backups(self) -> int:
        """백업 파일 개수 제한에 따른 정리"""
        try:
            max_backups = backup_config.get_max_total_backups()
            backup_files = list(self.backup_root_dir.glob("*.md"))
            
            if len(backup_files) <= max_backups:
                return 0
            
            # 생성일 기준으로 정렬 (오래된 것부터)
            backup_files.sort(key=lambda f: f.stat().st_ctime)
            
            # 초과된 개수만큼 삭제
            excess_count = len(backup_files) - max_backups
            deleted_count = 0
            
            for file_path in backup_files[:excess_count]:
                file_path.unlink()
                deleted_count += 1
                logger.info(f"Excess backup deleted: {file_path}")
            
            logger.info(f"Excess cleanup completed: {deleted_count} old backups deleted (max: {max_backups})")
            return deleted_count
            
        except Exception as e:
            logger.error(f"Failed to cleanup excess backups: {e}")
            return 0

# 전역 백업 매니저 인스턴스
backup_manager = DocumentBackupManager()

def create_document_backup(document, force: bool = False) -> Optional[str]:
    """문서 백업 생성 (편의 함수)"""
    if not force and not backup_config.should_backup_on_create():
        return None
    return backup_manager.create_backup(document)

def update_document_backup(document, force: bool = False) -> Optional[str]:
    """문서 업데이트 백업 생성 (편의 함수)"""
    if not force and not backup_config.should_backup_on_update():
        return None
    return backup_manager.update_backup(document)

def upload_document_backup(document, force: bool = False) -> Optional[str]:
    """문서 업로드 백업 생성 (편의 함수)"""
    if not force and not backup_config.should_backup_on_upload():
        return None
    return backup_manager.create_backup(document)

def export_all_documents(use_short_filename: bool = False) -> dict:
    """DB의 모든 문서를 백업 폴더로 내보내기 (중복 제거 포함)"""
    from app.models.document import Document
    import hashlib
    
    try:
        # 모든 문서 가져오기
        documents = Document.query.all()
        
        # 기존 백업 파일들 스캔 (파일명 비교용)
        existing_files = set()
        existing_base_files = set()  # 기본 파일명 (숫자 접미사 제외)
        existing_file_contents = {}
        
        import re
        
        for file_path in backup_manager.backup_root_dir.glob("*.md"):
            existing_files.add(file_path.name)
            
            # 기본 파일명 추출 (숫자 접미사 제거)
            base_name = file_path.stem  # 확장자 제거
            # _숫자 패턴 제거 (예: _1, _2, _3 등)
            base_name_clean = re.sub(r'_\d+$', '', base_name)
            existing_base_files.add(base_name_clean + '.md')
            
            try:
                with open(file_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                    # Generated 라인 제거하여 비교용 콘텐츠 만들기
                    import re
                    comparison_content = re.sub(r'# Generated:.*\n', '', content)
                    # 내용 해시 생성 (내용 비교용)
                    content_hash = hashlib.md5(comparison_content.strip().encode('utf-8')).hexdigest()
                    existing_file_contents[content_hash] = file_path.name
            except Exception as e:
                logger.warning(f"Error reading existing file {file_path}: {e}")
        
        results = {
            'total_documents': len(documents),
            'exported': 0,
            'skipped_filename': 0,
            'skipped_content': 0,
            'errors': 0,
            'details': []
        }
        
        for document in documents:
            try:
                # 파일명 생성
                if use_short_filename:
                    filename = backup_manager.generate_short_filename(
                        document.title, 
                        document.created_at
                    )
                else:
                    # 기본적으로 64자 제한 파일명 사용
                    filename = backup_manager.generate_medium_filename(
                        document.title, 
                        document.created_at
                    )
                
                # 백업 콘텐츠 생성
                backup_content = backup_manager.generate_backup_content(document)
                # 비교용 콘텐츠 생성 (타임스탬프 제외)
                comparison_content = backup_manager.generate_backup_content_for_comparison(document)
                content_hash = hashlib.md5(comparison_content.strip().encode('utf-8')).hexdigest()
                
                # 1차: 파일명 비교 (Primary comparison by filename)
                # 정확한 파일명 또는 기본 파일명 확인
                if filename in existing_files or filename in existing_base_files:
                    results['skipped_filename'] += 1
                    results['details'].append({
                        'document_id': document.id,
                        'title': document.title,
                        'filename': filename,
                        'status': 'skipped_filename_exists',
                        'reason': 'Same filename already exists in backup folder'
                    })
                    logger.info(f"Skipped document {document.id}: {filename} (filename already exists)")
                    continue
                
                # 2차: 파일 내용 비교 (Secondary comparison by content)
                if content_hash in existing_file_contents:
                    existing_filename = existing_file_contents[content_hash]
                    results['skipped_content'] += 1
                    results['details'].append({
                        'document_id': document.id,
                        'title': document.title,
                        'filename': filename,
                        'existing_file': existing_filename,
                        'status': 'skipped_content_duplicate',
                        'reason': 'Identical content already exists in backup folder'
                    })
                    logger.info(f"Skipped document {document.id}: {filename} (identical content exists as {existing_filename})")
                    continue
                
                # 파일명과 내용 모두 중복되지 않은 경우 내보내기
                backup_path = backup_manager.backup_root_dir / filename
                
                # 파일 쓰기
                with open(backup_path, 'w', encoding='utf-8') as f:
                    f.write(backup_content)
                
                # 내보낸 파일 추적에 추가
                existing_files.add(filename)
                existing_file_contents[content_hash] = filename
                
                results['exported'] += 1
                results['details'].append({
                    'document_id': document.id,
                    'title': document.title,
                    'filename': filename,
                    'status': 'exported',
                    'reason': 'New file with unique filename and content'
                })
                
                logger.info(f"Exported document {document.id}: {filename}")
                
            except Exception as e:
                results['errors'] += 1
                results['details'].append({
                    'document_id': document.id,
                    'title': document.title,
                    'error': str(e),
                    'status': 'error'
                })
                logger.error(f"Failed to export document {document.id}: {e}")
        
        total_skipped = results['skipped_filename'] + results['skipped_content']
        logger.info(f"Export completed: {results['exported']} exported, {total_skipped} skipped ({results['skipped_filename']} by filename, {results['skipped_content']} by content), {results['errors']} errors")
        return results
        
    except Exception as e:
        logger.error(f"Failed to export documents: {e}")
        return {
            'total_documents': 0,
            'exported': 0,
            'skipped_filename': 0,
            'skipped_content': 0,
            'errors': 1,
            'details': [{'error': str(e), 'status': 'fatal_error'}]
        }