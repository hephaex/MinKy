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
    
    def sanitize_filename(self, title: str) -> str:
        """파일명에 사용할 수 없는 문자를 제거하고 정리"""
        # 파일명에 사용할 수 없는 문자 제거
        sanitized = re.sub(r'[<>:"/\\|?*]', '_', title)
        
        # 연속된 공백이나 언더스코어를 하나로 변경
        sanitized = re.sub(r'[\s_]+', '_', sanitized)
        
        # 앞뒤 공백 및 언더스코어 제거
        sanitized = sanitized.strip('_').strip()
        
        # 너무 긴 제목은 잘라내기 (100자 제한)
        if len(sanitized) > 100:
            sanitized = sanitized[:100]
        
        # 빈 제목인 경우 기본값 설정
        if not sanitized:
            sanitized = "untitled"
        
        return sanitized
    
    def generate_backup_filename(self, document_title: str, created_at: Optional[datetime] = None) -> str:
        """백업 파일명 생성: YYYYMMDD_제목.md"""
        if created_at is None:
            created_at = datetime.now()
        
        # 날짜 부분 생성
        date_str = created_at.strftime("%Y%m%d")
        
        # 제목 부분 생성
        sanitized_title = self.sanitize_filename(document_title)
        
        # 시간 부분 추가 (동일 날짜에 같은 제목인 경우를 위해)
        time_str = created_at.strftime("%H%M%S")
        
        filename = f"{date_str}_{sanitized_title}_{time_str}.md"
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
    
    def cleanup_old_backups(self, days_to_keep: int = None) -> int:
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