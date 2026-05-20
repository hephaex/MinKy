"""
Backup Filename Parser
Handles parsing of various backup filename formats
"""

import re
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional
import logging

logger = logging.getLogger(__name__)


class BackupFilenameParser:
    """Parser for backup filename formats"""

    def __init__(self, backup_dir: Path):
        self.backup_dir = backup_dir

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

    def parse(self, filename: str) -> Optional[Dict]:
        """
        Parse backup filename and extract information.

        Supported patterns:
        - YYYYMMDD_title_HHMMSS.md
        - YYYYMMDD_title.md
        - YYYY-MM-DD_title.md
        - Generic .md files (uses file mtime)
        """
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
