import json
from pathlib import Path
from typing import Dict, Any
import logging

logger = logging.getLogger(__name__)

class BackupConfig:
    """백업 시스템 설정 관리 클래스"""
    
    DEFAULT_CONFIG = {
        "auto_cleanup_enabled": False,  # 기본적으로 자동 정리 비활성화
        "auto_cleanup_days": 30,        # 자동 정리시 보관 기간
        "backup_enabled": True,         # 백업 기능 활성화 여부
        "backup_on_create": True,       # 문서 생성시 백업
        "backup_on_update": True,       # 문서 수정시 백업
        "backup_on_upload": True,       # 파일 업로드시 백업
        "max_backup_size_mb": 10,       # 개별 백업 파일 최대 크기 (MB)
        "max_total_backups": 1000,      # 최대 백업 파일 개수
        "compression_enabled": False,   # 백업 파일 압축 (향후 기능)
    }
    
    def __init__(self, config_file: str = "backup_config.json"):
        self.config_file = Path(config_file)
        self.config = self.load_config()
    
    def load_config(self) -> Dict[str, Any]:
        """설정 파일 로드"""
        try:
            if self.config_file.exists():
                with open(self.config_file, 'r', encoding='utf-8') as f:
                    loaded_config = json.load(f)
                
                # 기본 설정과 병합 (새로운 설정 키가 추가된 경우 대비)
                config = self.DEFAULT_CONFIG.copy()
                config.update(loaded_config)
                
                logger.info(f"Backup config loaded from {self.config_file}")
                return config
            else:
                logger.info("Backup config file not found, using defaults")
                return self.DEFAULT_CONFIG.copy()
                
        except Exception as e:
            logger.error(f"Failed to load backup config: {e}")
            return self.DEFAULT_CONFIG.copy()
    
    def save_config(self) -> bool:
        """설정 파일 저장"""
        try:
            with open(self.config_file, 'w', encoding='utf-8') as f:
                json.dump(self.config, f, indent=2, ensure_ascii=False)
            
            logger.info(f"Backup config saved to {self.config_file}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to save backup config: {e}")
            return False
    
    def get(self, key: str, default=None):
        """설정 값 조회"""
        return self.config.get(key, default)
    
    def set(self, key: str, value: Any) -> bool:
        """설정 값 변경"""
        try:
            self.config[key] = value
            return self.save_config()
        except Exception as e:
            logger.error(f"Failed to set config {key}={value}: {e}")
            return False
    
    def update(self, updates: Dict[str, Any]) -> bool:
        """여러 설정 값 한번에 업데이트"""
        try:
            self.config.update(updates)
            return self.save_config()
        except Exception as e:
            logger.error(f"Failed to update config: {e}")
            return False
    
    def reset_to_defaults(self) -> bool:
        """기본 설정으로 초기화"""
        try:
            self.config = self.DEFAULT_CONFIG.copy()
            return self.save_config()
        except Exception as e:
            logger.error(f"Failed to reset config: {e}")
            return False
    
    def is_auto_cleanup_enabled(self) -> bool:
        """자동 정리 기능 활성화 여부"""
        return bool(self.get("auto_cleanup_enabled", False))

    def get_auto_cleanup_days(self) -> int:
        """자동 정리 보관 기간"""
        return int(self.get("auto_cleanup_days", 30))

    def is_backup_enabled(self) -> bool:
        """백업 기능 활성화 여부"""
        return bool(self.get("backup_enabled", True))
    
    def should_backup_on_create(self) -> bool:
        """문서 생성시 백업 여부"""
        return self.get("backup_on_create", True) and self.is_backup_enabled()
    
    def should_backup_on_update(self) -> bool:
        """문서 수정시 백업 여부"""
        return self.get("backup_on_update", True) and self.is_backup_enabled()
    
    def should_backup_on_upload(self) -> bool:
        """파일 업로드시 백업 여부"""
        return self.get("backup_on_upload", True) and self.is_backup_enabled()
    
    def get_max_backup_size_mb(self) -> int:
        """개별 백업 파일 최대 크기 (MB)"""
        return int(self.get("max_backup_size_mb", 10))

    def get_max_total_backups(self) -> int:
        """최대 백업 파일 개수"""
        return int(self.get("max_total_backups", 1000))
    
    def to_dict(self) -> Dict[str, Any]:
        """설정을 딕셔너리로 반환"""
        return self.config.copy()

# 전역 백업 설정 인스턴스
backup_config = BackupConfig()