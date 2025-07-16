import re
import yaml
from typing import List, Dict, Optional, Tuple
import logging
import requests
import os
from urllib.parse import urlparse
import hashlib

logger = logging.getLogger(__name__)

class ObsidianParser:
    """옵시디언 스타일 마크다운 파서 - 내부 링크, 태그, 프론트매터 지원"""
    
    def __init__(self):
        # 내부 링크 패턴
        self.internal_link_pattern = re.compile(r'\[\[([^\|\]]+)(?:\|([^\]]+))?\]\]')  # [[link|display]] or [[link]]
        
        # 태그 패턴 - 워드 경계를 고려한 해시태그
        self.hashtag_pattern = re.compile(r'(?:^|\s)#([a-zA-Z가-힣][a-zA-Z0-9가-힣_-]*)')
        
        # 프론트매터 패턴
        self.frontmatter_pattern = re.compile(r'^---\s*\n(.*?)\n---\s*\n', re.DOTALL)
    
    def parse_markdown(self, content: str, backup_dir: str = None) -> Dict:
        """마크다운 파싱 메인 함수"""
        result = {
            'frontmatter': {},
            'internal_links': [],
            'hashtags': [],
            'clean_content': content
        }
        
        # 프론트매터 추출 및 제거
        frontmatter_data, clean_content = self._extract_frontmatter(content)
        result['frontmatter'] = frontmatter_data
        result['clean_content'] = clean_content
        
        # 이미지 다운로드 및 경로 변환
        if backup_dir:
            result['clean_content'] = self._process_images(result['clean_content'], backup_dir)
        
        # 내부 링크 추출
        result['internal_links'] = self._extract_internal_links(result['clean_content'])
        
        # 해시태그 추출
        result['hashtags'] = self._extract_hashtags(result['clean_content'])
        
        return result
    
    def _extract_frontmatter(self, content: str) -> Tuple[Dict, str]:
        """YAML 프론트매터 추출"""
        frontmatter_data = {}
        clean_content = content
        
        match = self.frontmatter_pattern.match(content)
        if match:
            try:
                yaml_content = match.group(1)
                frontmatter_data = yaml.safe_load(yaml_content) or {}
                clean_content = content[match.end():]
                
                # Convert date objects to ISO format strings for JSON serialization
                frontmatter_data = self._convert_dates_to_strings(frontmatter_data)
                
                logger.info(f"Extracted frontmatter: {frontmatter_data}")
            except (yaml.YAMLError, ImportError, AttributeError) as e:
                logger.warning(f"Failed to parse YAML frontmatter: {e}")
                frontmatter_data = {}
            except Exception as e:
                logger.error(f"Unexpected error parsing frontmatter: {e}")
                frontmatter_data = {}
        
        return frontmatter_data, clean_content
    
    def _convert_dates_to_strings(self, data: Dict) -> Dict:
        """Convert date/datetime objects to ISO format strings for JSON serialization"""
        from datetime import date, datetime
        
        if not isinstance(data, dict):
            return data
        
        converted_data = {}
        for key, value in data.items():
            if isinstance(value, datetime):
                # Convert datetime to ISO format string
                converted_data[key] = value.isoformat()
            elif isinstance(value, date):
                # Convert date to ISO format string
                converted_data[key] = value.isoformat()
            elif isinstance(value, dict):
                # Recursively convert nested dictionaries
                converted_data[key] = self._convert_dates_to_strings(value)
            elif isinstance(value, list):
                # Convert dates in lists
                converted_data[key] = [
                    item.isoformat() if isinstance(item, (date, datetime)) else
                    self._convert_dates_to_strings(item) if isinstance(item, dict) else
                    item
                    for item in value
                ]
            else:
                # Keep other types as-is
                converted_data[key] = value
        
        return converted_data
    
    def _process_images(self, content: str, backup_dir: str) -> str:
        """이미지 URL을 찾아서 다운로드하고 로컬 경로로 변환"""
        # 링크된 이미지 패턴: [![image.png](url)](url)
        linked_image_pattern = re.compile(r'\[!\[([^\]]*)\]\(([^)]+)\)\]\(([^)]+)\)')
        # 일반 이미지 패턴: ![image.png](url)
        image_pattern = re.compile(r'!\[([^\]]*)\]\(([^)]+)\)')
        
        # img 폴더 생성
        img_dir = os.path.join(backup_dir, 'img')
        os.makedirs(img_dir, exist_ok=True)
        
        processed_content = content
        
        # 링크된 이미지 처리
        for match in linked_image_pattern.finditer(content):
            alt_text = match.group(1)
            image_url = match.group(2)
            original_match = match.group(0)
            
            if self._is_external_url(image_url):
                local_filename = self._download_image(image_url, img_dir, alt_text)
                if local_filename:
                    # 프록시를 통한 백엔드 이미지 경로로 변환
                    new_image_markdown = f'![{alt_text}](/img/{local_filename})'
                    processed_content = processed_content.replace(original_match, new_image_markdown)
                    logger.info(f"Converted linked image: {image_url} -> {local_filename}")
        
        # 일반 이미지 처리 (링크된 이미지로 이미 처리되지 않은 것만)
        for match in image_pattern.finditer(processed_content):
            alt_text = match.group(1)
            image_url = match.group(2)
            original_match = match.group(0)
            
            if self._is_external_url(image_url):
                local_filename = self._download_image(image_url, img_dir, alt_text)
                if local_filename:
                    # 프록시를 통한 백엔드 이미지 경로로 변환
                    new_image_markdown = f'![{alt_text}](/img/{local_filename})'
                    processed_content = processed_content.replace(original_match, new_image_markdown)
                    logger.info(f"Converted image: {image_url} -> {local_filename}")
        
        return processed_content
    
    def _is_external_url(self, url: str) -> bool:
        """외부 URL인지 확인"""
        return url.startswith(('http://', 'https://'))
    
    def _download_image(self, url: str, img_dir: str, alt_text: str = '') -> str:
        """이미지를 다운로드하고 로컬 파일명 반환"""
        try:
            response = requests.get(url, stream=True, timeout=30)
            response.raise_for_status()
            
            # 파일 확장자 추출
            parsed_url = urlparse(url)
            url_path = parsed_url.path
            file_extension = os.path.splitext(url_path)[1] or '.png'
            
            # 파일명 생성 (alt_text가 있으면 사용, 없으면 URL 해시 사용)
            if alt_text and alt_text.strip():
                # alt_text에서 파일명으로 사용할 수 없는 문자 제거
                safe_filename = re.sub(r'[^\w\-_.]', '_', alt_text.strip())
                filename = f"{safe_filename}{file_extension}"
            else:
                # URL 해시로 고유 파일명 생성
                url_hash = hashlib.md5(url.encode()).hexdigest()[:8]
                filename = f"image_{url_hash}{file_extension}"
            
            file_path = os.path.join(img_dir, filename)
            
            # 이미 존재하는 파일이면 다운로드 건너뛰기
            if os.path.exists(file_path):
                logger.info(f"Image already exists: {filename}")
                return filename
            
            # 이미지 다운로드
            with open(file_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    f.write(chunk)
            
            logger.info(f"Downloaded image: {url} -> {filename}")
            return filename
            
        except Exception as e:
            logger.error(f"Failed to download image {url}: {e}")
            return None
    
    def _extract_internal_links(self, content: str) -> List[Dict]:
        """내부 링크 추출"""
        links = []
        
        for match in self.internal_link_pattern.finditer(content):
            target = match.group(1).strip()
            display_text = match.group(2).strip() if match.group(2) else target
            
            links.append({
                'target': target,
                'display_text': display_text,
                'position': match.start(),
                'raw_match': match.group(0)
            })
        
        return links
    
    def _extract_hashtags(self, content: str) -> List[Dict]:
        """해시태그 추출"""
        hashtags = []
        seen_tags = set()
        
        for match in self.hashtag_pattern.finditer(content):
            tag = match.group(1).lower()
            
            if tag not in seen_tags:
                hashtags.append({
                    'tag': tag,
                    'position': match.start(),
                    'raw_match': match.group(0).strip()
                })
                seen_tags.add(tag)
        
        return hashtags
    
    def render_internal_links(self, content: str, document_lookup_func=None) -> str:
        """내부 링크를 HTML 링크로 변환"""
        def replace_link(match):
            target = match.group(1).strip()
            display_text = match.group(2).strip() if match.group(2) else target
            
            if document_lookup_func:
                doc_id = document_lookup_func(target)
                if doc_id:
                    return f'<a href="/documents/{doc_id}" class="internal-link">{display_text}</a>'
                else:
                    return f'<a href="#" class="internal-link broken" data-target="{target}">{display_text}</a>'
            else:
                return f'<span class="internal-link-placeholder" data-target="{target}">{display_text}</span>'
        
        return self.internal_link_pattern.sub(replace_link, content)
    
    def render_hashtags(self, content: str) -> str:
        """해시태그를 HTML 링크로 변환"""
        def replace_hashtag(match):
            full_match = match.group(0)
            tag = match.group(1)
            prefix = full_match[:full_match.index('#')]
            
            return f'{prefix}<a href="/tags/{tag}" class="hashtag">#{tag}</a>'
        
        return self.hashtag_pattern.sub(replace_hashtag, content)
    
    def add_frontmatter(self, content: str, metadata: Dict) -> str:
        """기존 콘텐츠에 프론트매터 추가/업데이트"""
        # 기존 프론트매터 제거
        _, clean_content = self._extract_frontmatter(content)
        
        if metadata:
            yaml_content = yaml.dump(metadata, allow_unicode=True, default_flow_style=False)
            return f"---\n{yaml_content}---\n{clean_content}"
        else:
            return clean_content