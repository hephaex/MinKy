import os
import re
from datetime import datetime
from pathlib import Path
from typing import Optional
import logging

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
        try:
            # 백업 파일명 생성
            filename = self.generate_backup_filename(
                document.title, 
                document.created_at
            )
            
            backup_path = self.backup_root_dir / filename
            
            # 백업 내용 생성
            backup_content = self.generate_backup_content(document)
            
            # 파일 저장
            with open(backup_path, 'w', encoding='utf-8') as f:
                f.write(backup_content)
            
            logger.info(f"Document backup created: {backup_path}")
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
    
    def cleanup_old_backups(self, days_to_keep: int = 30) -> int:
        """오래된 백업 파일 정리"""
        try:
            cutoff_date = datetime.now().timestamp() - (days_to_keep * 24 * 60 * 60)
            deleted_count = 0
            
            for file_path in self.backup_root_dir.glob("*.md"):
                if file_path.stat().st_ctime < cutoff_date:
                    file_path.unlink()
                    deleted_count += 1
                    logger.info(f"Old backup deleted: {file_path}")
            
            logger.info(f"Cleanup completed: {deleted_count} old backups deleted")
            return deleted_count
            
        except Exception as e:
            logger.error(f"Failed to cleanup old backups: {e}")
            return 0

# 전역 백업 매니저 인스턴스
backup_manager = DocumentBackupManager()

def create_document_backup(document) -> Optional[str]:
    """문서 백업 생성 (편의 함수)"""
    return backup_manager.create_backup(document)

def update_document_backup(document) -> Optional[str]:
    """문서 업데이트 백업 생성 (편의 함수)"""
    return backup_manager.update_backup(document)