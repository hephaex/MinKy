import re
import os
import sqlite3
import json
from datetime import datetime, timezone
from typing import List, Dict, Optional, Tuple, Any
import orgparse
from pathlib import Path
import logging

logger = logging.getLogger(__name__)

class OrgRoamParser:
    """Emacs org-roam 문서 파서"""
    
    def __init__(self):
        self.link_pattern = re.compile(r'\[\[([^\]]+)\]\[([^\]]*)\]\]')  # [[link][description]]
        self.simple_link_pattern = re.compile(r'\[\[([^\]]+)\]\]')  # [[link]]
        self.tag_pattern = re.compile(r'#\+ROAM_TAGS:\s*(.+)')
        self.alias_pattern = re.compile(r'#\+ROAM_ALIAS:\s*(.+)')
        self.title_pattern = re.compile(r'#\+TITLE:\s*(.+)')
        self.id_pattern = re.compile(r':ID:\s+([a-f0-9\-]+)')
        
    def parse_org_file(self, file_path: str) -> Optional[Dict]:
        """단일 org 파일 파싱"""
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            # orgparse를 사용한 구조화된 파싱
            try:
                org_doc = orgparse.load(file_path)
            except Exception as e:
                logger.warning(f"orgparse failed for {file_path}: {e}, using manual parsing")
                org_doc = None
            
            # 메타데이터 추출
            metadata = self._extract_metadata(content)
            
            # 링크 추출
            links = self._extract_links(content)
            
            # 태그 추출
            tags = self._extract_tags(content)
            
            # 내용 정리 (메타데이터 제거)
            clean_content = self._clean_content(content)
            
            # 파일 정보
            file_info = os.stat(file_path)
            
            result = {
                'file_path': file_path,
                'filename': os.path.basename(file_path),
                'title': metadata.get('title', os.path.splitext(os.path.basename(file_path))[0]),
                'id': metadata.get('id'),
                'roam_tags': metadata.get('roam_tags', []),
                'roam_aliases': metadata.get('roam_aliases', []),
                'tags': tags,
                'links': links,
                'content': clean_content,
                'raw_content': content,
                'created_at': datetime.fromtimestamp(file_info.st_ctime),
                'modified_at': datetime.fromtimestamp(file_info.st_mtime),
                'size': file_info.st_size,
                'language': self._detect_language(clean_content)
            }
            
            # orgparse 결과가 있으면 구조 정보 추가
            if org_doc:
                result['structure'] = self._extract_structure(org_doc)
            
            return result
            
        except Exception as e:
            logger.error(f"Failed to parse org file {file_path}: {e}")
            return None
    
    def _extract_metadata(self, content: str) -> Dict:
        """메타데이터 추출"""
        metadata = {}
        
        # TITLE 추출
        title_match = self.title_pattern.search(content)
        if title_match:
            metadata['title'] = title_match.group(1).strip()
        
        # ID 추출
        id_match = self.id_pattern.search(content)
        if id_match:
            metadata['id'] = id_match.group(1).strip()
        
        # ROAM_TAGS 추출
        tag_match = self.tag_pattern.search(content)
        if tag_match:
            tags = [tag.strip().strip('"') for tag in tag_match.group(1).split()]
            metadata['roam_tags'] = tags
        
        # ROAM_ALIAS 추출
        alias_match = self.alias_pattern.search(content)
        if alias_match:
            aliases = [alias.strip().strip('"') for alias in alias_match.group(1).split()]
            metadata['roam_aliases'] = aliases
        
        return metadata
    
    def _extract_links(self, content: str) -> List[Dict]:
        """링크 추출"""
        links = []
        
        # [[link][description]] 형태의 링크
        for match in self.link_pattern.finditer(content):
            link_target = match.group(1).strip()
            link_desc = match.group(2).strip()
            
            links.append({
                'target': link_target,
                'description': link_desc,
                'type': 'org_link',
                'position': match.start()
            })
        
        # [[link]] 형태의 링크
        for match in self.simple_link_pattern.finditer(content):
            link_target = match.group(1).strip()
            
            # 이미 description이 있는 링크는 제외
            if not any(link['target'] == link_target and link['position'] == match.start() 
                      for link in links):
                links.append({
                    'target': link_target,
                    'description': link_target,
                    'type': 'org_simple_link',
                    'position': match.start()
                })
        
        # HTTP 링크
        http_pattern = re.compile(r'https?://[^\s\]]+')
        for match in http_pattern.finditer(content):
            links.append({
                'target': match.group(0),
                'description': match.group(0),
                'type': 'http_link',
                'position': match.start()
            })
        
        return links
    
    def _extract_tags(self, content: str) -> List[str]:
        """일반 태그 추출 (#+TAGS나 :tag: 형태)"""
        tags = []
        
        # #+TAGS: 형태
        tags_pattern = re.compile(r'#\+TAGS:\s*(.+)')
        tags_match = tags_pattern.search(content)
        if tags_match:
            tags.extend([tag.strip() for tag in tags_match.group(1).split()])
        
        # :tag: 형태 (헤딩 태그)
        heading_tags_pattern = re.compile(r':\w+:')
        tags.extend([match.group(0).strip(':') for match in heading_tags_pattern.finditer(content)])
        
        return list(set(tags))  # 중복 제거
    
    def _clean_content(self, content: str) -> str:
        """메타데이터를 제거한 깨끗한 내용 반환"""
        lines = content.split('\n')
        clean_lines = []
        
        for line in lines:
            # 메타데이터 라인 제거
            if (line.strip().startswith('#+') or 
                line.strip().startswith(':') and line.strip().endswith(':')):
                continue
            clean_lines.append(line)
        
        return '\n'.join(clean_lines).strip()
    
    def _extract_structure(self, org_doc) -> Dict[str, Any]:
        """org 문서의 구조 정보 추출"""
        structure: Dict[str, Any] = {
            'headings': [],
            'sections': [],
            'todo_items': [],
            'scheduled_items': []
        }
        
        try:
            # 헤딩 정보 추출
            for node in org_doc[1:]:  # 첫 번째는 루트 노드
                if hasattr(node, 'heading'):
                    heading_info = {
                        'level': node.level,
                        'title': node.heading,
                        'todo_keyword': getattr(node, 'todo', None),
                        'tags': getattr(node, 'tags', []),
                        'scheduled': getattr(node, 'scheduled', None),
                        'deadline': getattr(node, 'deadline', None)
                    }
                    structure['headings'].append(heading_info)
                    
                    # TODO 아이템
                    if heading_info['todo_keyword']:
                        structure['todo_items'].append(heading_info)
                    
                    # 스케줄된 아이템
                    if heading_info['scheduled'] or heading_info['deadline']:
                        structure['scheduled_items'].append(heading_info)
        
        except Exception as e:
            logger.warning(f"Failed to extract structure: {e}")
        
        return structure
    
    def _detect_language(self, content: str) -> str:
        """내용의 주요 언어 감지"""
        from app.utils.korean_text import KoreanTextProcessor
        return KoreanTextProcessor.detect_language(content)
    
    def parse_org_roam_directory(self, directory_path: str) -> List[Dict[str, Any]]:
        """org-roam 디렉토리 전체 파싱"""
        documents: List[Dict[str, Any]] = []
        directory = Path(directory_path)
        
        if not directory.exists():
            logger.error(f"Directory not found: {directory_path}")
            return documents
        
        org_files = list(directory.rglob('*.org'))
        logger.info(f"Found {len(org_files)} org files in {directory_path}")
        
        for org_file in org_files:
            try:
                doc = self.parse_org_file(str(org_file))
                if doc:
                    documents.append(doc)
            except Exception as e:
                logger.error(f"Failed to parse {org_file}: {e}")
        
        # 백링크 계산
        self._calculate_backlinks(documents)
        
        return documents
    
    def _calculate_backlinks(self, documents: List[Dict]):
        """문서들 간의 백링크 계산"""
        # 문서 ID와 제목으로 인덱스 생성
        doc_index = {}
        for doc in documents:
            # ID로 인덱싱
            if doc.get('id'):
                doc_index[doc['id']] = doc
            
            # 제목으로 인덱싱
            doc_index[doc['title']] = doc
            
            # 파일명으로 인덱싱
            filename_without_ext = os.path.splitext(doc['filename'])[0]
            doc_index[filename_without_ext] = doc
            
            # 별칭으로 인덱싱
            for alias in doc.get('roam_aliases', []):
                doc_index[alias] = doc
        
        # 각 문서에 백링크 정보 추가
        for doc in documents:
            doc['backlinks'] = []
            doc['outbound_links'] = []
        
        # 링크 관계 계산
        for doc in documents:
            for link in doc.get('links', []):
                target = link['target']
                
                # 링크 대상 문서 찾기
                target_doc = doc_index.get(target)
                if target_doc and target_doc != doc:
                    # 아웃바운드 링크 추가
                    doc['outbound_links'].append({
                        'target_id': target_doc.get('id'),
                        'target_title': target_doc['title'],
                        'target_filename': target_doc['filename'],
                        'link_text': link['description']
                    })
                    
                    # 백링크 추가
                    target_doc['backlinks'].append({
                        'source_id': doc.get('id'),
                        'source_title': doc['title'],
                        'source_filename': doc['filename'],
                        'link_text': link['description']
                    })

class OrgRoamImporter:
    """org-roam 문서를 Minky 시스템으로 임포트"""
    
    def __init__(self, db_session):
        self.db = db_session
        self.parser = OrgRoamParser()
    
    def import_from_directory(self, directory_path: str, user_id: int,
                            import_as_private: bool = True) -> Dict[str, Any]:
        """디렉토리에서 org-roam 문서들을 임포트"""
        from app.models.document import Document
        from app.utils.korean_text import process_korean_document

        results: Dict[str, Any] = {
            'imported': 0,
            'failed': 0,
            'skipped': 0,
            'errors': []
        }
        
        try:
            # org 파일들 파싱
            org_documents = self.parser.parse_org_roam_directory(directory_path)
            
            for org_doc in org_documents:
                try:
                    # 이미 존재하는 문서인지 확인 (파일명 기준)
                    existing_doc = Document.query.filter_by(
                        title=org_doc['title'], 
                        user_id=user_id
                    ).first()
                    
                    if existing_doc:
                        results['skipped'] += 1
                        continue
                    
                    # 마크다운으로 변환
                    markdown_content = self._convert_org_to_markdown(org_doc)
                    
                    # 한국어 처리
                    korean_processing = process_korean_document(
                        org_doc['title'], 
                        markdown_content
                    )
                    
                    # Document 생성
                    document = Document(
                        title=org_doc['title'],
                        markdown_content=markdown_content,
                        author=f"Imported from {org_doc['filename']}",
                        user_id=user_id,
                        is_public=not import_as_private
                    )
                    
                    # 메타데이터 저장
                    document.document_metadata = {
                        'org_roam_id': org_doc.get('id'),
                        'org_filename': org_doc['filename'],
                        'org_file_path': org_doc['file_path'],
                        'roam_tags': org_doc.get('roam_tags', []),
                        'roam_aliases': org_doc.get('roam_aliases', []),
                        'backlinks': org_doc.get('backlinks', []),
                        'outbound_links': org_doc.get('outbound_links', []),
                        'language': org_doc['language'],
                        'import_date': datetime.now(timezone.utc).isoformat()
                    }
                    
                    self.db.add(document)
                    
                    # 태그 추가
                    all_tags = org_doc.get('roam_tags', []) + org_doc.get('tags', [])
                    if korean_processing.get('auto_tags'):
                        all_tags.extend(korean_processing['auto_tags'])
                    
                    if all_tags:
                        document.add_tags(list(set(all_tags)))  # 중복 제거
                    
                    results['imported'] += 1
                    
                except Exception as e:
                    logger.error(f"Failed to import {org_doc.get('filename', 'unknown')}: {e}")
                    results['failed'] += 1
                    results['errors'].append(str(e))
            
            self.db.commit()
            
        except Exception as e:
            logger.error(f"Import process failed: {e}")
            results['errors'].append(str(e))
            self.db.rollback()
        
        return results
    
    def _convert_org_to_markdown(self, org_doc: Dict) -> str:
        """org 형식을 마크다운으로 변환"""
        content = org_doc['content']
        
        # 헤딩 변환 (* -> #)
        content = re.sub(r'^\*{1}\s', '# ', content, flags=re.MULTILINE)
        content = re.sub(r'^\*{2}\s', '## ', content, flags=re.MULTILINE)
        content = re.sub(r'^\*{3}\s', '### ', content, flags=re.MULTILINE)
        content = re.sub(r'^\*{4}\s', '#### ', content, flags=re.MULTILINE)
        content = re.sub(r'^\*{5}\s', '##### ', content, flags=re.MULTILINE)
        content = re.sub(r'^\*{6,}\s', '###### ', content, flags=re.MULTILINE)
        
        # 링크 변환 [[link][desc]] -> [desc](link)
        content = re.sub(r'\[\[([^\]]+)\]\[([^\]]*)\]\]', r'[\2](\1)', content)
        
        # 단순 링크 변환 [[link]] -> [link](link)
        content = re.sub(r'\[\[([^\]]+)\]\]', r'[\1](\1)', content)
        
        # 볼드 텍스트 변환 *text* -> **text**
        content = re.sub(r'\*([^*\n]+)\*', r'**\1**', content)
        
        # 이탤릭 텍스트 변환 /text/ -> *text*
        content = re.sub(r'/([^/\n]+)/', r'*\1*', content)
        
        # 코드 블록 변환
        content = re.sub(r'#\+BEGIN_SRC\s*(\w*)\n(.*?)#\+END_SRC', 
                        r'```\1\n\2```', content, flags=re.DOTALL)
        
        # 인라인 코드 변환 =code= -> `code`
        content = re.sub(r'=([^=\n]+)=', r'`\1`', content)
        
        # 리스트 변환 (org의 - 는 마크다운과 동일)
        
        return content.strip()

def create_org_roam_import_endpoint():
    """org-roam 임포트를 위한 엔드포인트 데코레이터"""
    def decorator(func):
        def wrapper(*args, **kwargs):
            # 여기에 임포트 로직 추가
            return func(*args, **kwargs)
        return wrapper
    return decorator