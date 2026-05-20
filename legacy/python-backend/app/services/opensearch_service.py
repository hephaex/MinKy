import logging
import re
from typing import Dict, List, Optional, Any
from opensearchpy import OpenSearch, RequestsHttpConnection
from opensearchpy.exceptions import NotFoundError
from app.utils.korean_text import KoreanTextProcessor
from app.services.opensearch_mappings import (
    get_document_index_settings,
    get_org_roam_index_settings
)

logger = logging.getLogger(__name__)

class OpenSearchService:
    """OpenSearch 기반 다국어 검색 서비스"""
    
    def __init__(self, host='localhost', port=9200, use_ssl=False,
                 username=None, password=None, verify_certs=None):
        """
        OpenSearch 클라이언트 초기화

        Args:
            host: OpenSearch 호스트
            port: OpenSearch 포트
            use_ssl: SSL 사용 여부
            username: 인증 사용자명
            password: 인증 비밀번호
            verify_certs: SSL 인증서 검증 여부 (기본값: 환경변수 또는 True)
        """
        import os
        self.korean_processor = KoreanTextProcessor()

        auth = None
        if username and password:
            auth = (username, password)

        # SSL 인증서 검증 설정 (보안: 기본값 True)
        if verify_certs is None:
            verify_certs = os.getenv('OPENSEARCH_VERIFY_CERTS', 'true').lower() == 'true'

        self.client = OpenSearch(
            hosts=[{'host': host, 'port': port}],
            http_auth=auth,
            use_ssl=use_ssl,
            verify_certs=verify_certs,
            connection_class=RequestsHttpConnection
        )
        
        # 인덱스 설정
        self.document_index = 'minky_documents'
        self.org_roam_index = 'minky_org_roam'
        
        # 연결 테스트
        try:
            info = self.client.info()
            logger.info(f"Connected to OpenSearch: {info['version']['number']}")
        except Exception as e:
            logger.error(f"Failed to connect to OpenSearch: {e}")
    
    def create_document_index(self):
        """문서용 인덱스 생성 (한국어 분석기 포함)"""
        index_settings = get_document_index_settings()

        try:
            if self.client.indices.exists(index=self.document_index):
                logger.info(f"Index {self.document_index} already exists")
                return True
            
            response = self.client.indices.create(
                index=self.document_index,
                body=index_settings
            )
            logger.info(f"Created index {self.document_index}: {response}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to create index {self.document_index}: {e}")
            return False
    
    def create_org_roam_index(self):
        """org-roam 문서용 특별 인덱스 생성"""
        index_settings = get_org_roam_index_settings()

        try:
            if self.client.indices.exists(index=self.org_roam_index):
                logger.info(f"Index {self.org_roam_index} already exists")
                return True
            
            response = self.client.indices.create(
                index=self.org_roam_index,
                body=index_settings
            )
            logger.info(f"Created org-roam index: {response}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to create org-roam index: {e}")
            return False
    
    def index_document(self, document_data: Dict) -> bool:
        """문서를 OpenSearch에 인덱싱"""
        try:
            # 한국어 처리
            if document_data.get('content'):
                from app.utils.korean_text import process_korean_document
                korean_processing = process_korean_document(
                    document_data.get('title', ''),
                    document_data.get('content', '')
                )
                document_data.update(korean_processing)
            
            response = self.client.index(
                index=self.document_index,
                id=document_data.get('id'),
                body=document_data
            )
            
            logger.debug(f"Indexed document {document_data.get('id')}: {response['result']}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to index document {document_data.get('id')}: {e}")
            return False
    
    # SECURITY: Pagination limits to prevent resource exhaustion
    MAX_PAGE_SIZE = 100
    MAX_PAGE_NUMBER = 1000

    def search_documents(self, query: str, filters: Optional[Dict] = None,
                        page: int = 1, per_page: int = 20,
                        user_id: Optional[int] = None) -> Dict:
        """문서 검색 (한국어 지원)"""

        # SECURITY: Validate and bound pagination parameters
        if not isinstance(page, int) or page < 1:
            page = 1
        page = min(page, self.MAX_PAGE_NUMBER)

        if not isinstance(per_page, int) or per_page < 1:
            per_page = 20
        per_page = min(per_page, self.MAX_PAGE_SIZE)

        # 검색 쿼리 구성
        must_clauses: List[Dict[str, Any]] = []
        filter_clauses: List[Dict[str, Any]] = []

        search_body: Dict[str, Any] = {
            "from": (page - 1) * per_page,
            "size": per_page,
            "query": {
                "bool": {
                    "must": must_clauses,
                    "filter": filter_clauses
                }
            },
            "highlight": {
                "fields": {
                    "title": {"pre_tags": ["<mark>"], "post_tags": ["</mark>"]},
                    "content": {
                        "pre_tags": ["<mark>"],
                        "post_tags": ["</mark>"],
                        "fragment_size": 150,
                        "number_of_fragments": 3
                    }
                }
            },
            "sort": [
                {"_score": {"order": "desc"}},
                {"updated_at": {"order": "desc"}}
            ]
        }
        
        # 텍스트 검색
        if query:
            # 언어 감지
            language = self.korean_processor.detect_language(query)
            
            if language == 'korean':
                # 한국어 검색
                must_clauses.append({
                    "multi_match": {
                        "query": query,
                        "fields": [
                            "title.korean^3",
                            "content.korean",
                            "search_vector^2"
                        ],
                        "type": "best_fields",
                        "fuzziness": "AUTO"
                    }
                })
            else:
                # 영어/혼합 검색
                must_clauses.append({
                    "multi_match": {
                        "query": query,
                        "fields": [
                            "title^3",
                            "title.mixed^2",
                            "content",
                            "content.mixed"
                        ],
                        "type": "best_fields",
                        "fuzziness": "AUTO"
                    }
                })
        else:
            # 전체 검색
            must_clauses.append({"match_all": {}})
        
        # 접근 권한 필터
        if user_id:
            filter_clauses.append({
                "bool": {
                    "should": [
                        {"term": {"is_public": True}},
                        {"term": {"user_id": user_id}}
                    ]
                }
            })
        else:
            filter_clauses.append({"term": {"is_public": True}})
        
        # 추가 필터 적용
        if filters:
            if filters.get('author'):
                filter_clauses.append({
                    "term": {"author.keyword": filters['author']}
                })

            if filters.get('tags'):
                if isinstance(filters['tags'], list):
                    filter_clauses.append({
                        "terms": {"tags": filters['tags']}
                    })
                else:
                    filter_clauses.append({
                        "term": {"tags": filters['tags']}
                    })

            if filters.get('language'):
                filter_clauses.append({
                    "term": {"language": filters['language']}
                })

            if filters.get('date_from') or filters.get('date_to'):
                date_range: Dict[str, Any] = {}
                if filters.get('date_from'):
                    date_range['gte'] = filters['date_from']
                if filters.get('date_to'):
                    date_range['lte'] = filters['date_to']

                filter_clauses.append({
                    "range": {"updated_at": date_range}
                })
        
        try:
            response = self.client.search(
                index=self.document_index,
                body=search_body
            )
            
            # 결과 처리
            hits = response['hits']
            documents = []
            
            for hit in hits['hits']:
                doc = hit['_source']
                doc['search_score'] = hit['_score']
                
                # 하이라이트 정보 추가
                if 'highlight' in hit:
                    doc['highlights'] = hit['highlight']
                
                documents.append(doc)
            
            return {
                'documents': documents,
                'total': hits['total']['value'],
                'page': page,
                'per_page': per_page,
                'pages': (hits['total']['value'] + per_page - 1) // per_page
            }
            
        except Exception as e:
            # SECURITY: Log detailed error but return generic message
            logger.error(f"Search failed: {e}", exc_info=True)
            return {
                'documents': [],
                'total': 0,
                'page': page,
                'per_page': per_page,
                'pages': 0,
                'error': 'Search failed. Please try again later.'
            }
    
    # SECURITY: Maximum limit for tag suggestions
    MAX_TAG_SUGGESTIONS = 50

    def suggest_tags(self, query: str, limit: int = 10) -> List[str]:
        """태그 자동완성 제안"""
        try:
            # SECURITY: Validate and bound limit parameter
            if not isinstance(limit, int) or limit < 1:
                limit = 10
            limit = min(limit, self.MAX_TAG_SUGGESTIONS)

            # Escape regex metacharacters to prevent regex injection
            escaped_query = re.escape(query)
            search_body = {
                "size": 0,
                "aggs": {
                    "tag_suggestions": {
                        "terms": {
                            "field": "tags",
                            "include": f".*{escaped_query}.*",
                            "size": limit
                        }
                    }
                }
            }
            
            response = self.client.search(
                index=self.document_index,
                body=search_body
            )
            
            suggestions = []
            if 'aggregations' in response:
                buckets = response['aggregations']['tag_suggestions']['buckets']
                suggestions = [bucket['key'] for bucket in buckets]
            
            return suggestions
            
        except Exception as e:
            logger.error(f"Tag suggestion failed: {e}")
            return []
    
    def get_document_statistics(self, user_id: Optional[int] = None) -> Dict:
        """문서 통계 정보 (with access control)

        Args:
            user_id: Filter statistics to documents accessible by this user
        """
        try:
            stats_body: Dict[str, Any] = {
                "size": 0,
                "aggs": {
                    "total_docs": {"value_count": {"field": "id"}},
                    "by_language": {"terms": {"field": "language"}},
                    "by_author": {"terms": {"field": "author.keyword", "size": 10}},
                    "by_tags": {"terms": {"field": "tags", "size": 20}},
                    "date_histogram": {
                        "date_histogram": {
                            "field": "created_at",
                            "calendar_interval": "month"
                        }
                    }
                }
            }

            # SECURITY: Apply access control filter to statistics
            if user_id is not None:
                # User can see public docs + their own private docs
                stats_body["query"] = {
                    "bool": {
                        "should": [
                            {"term": {"is_public": True}},
                            {"term": {"user_id": user_id}}
                        ],
                        "minimum_should_match": 1
                    }
                }
            else:
                # Unauthenticated: only public documents
                stats_body["query"] = {
                    "term": {"is_public": True}
                }

            response = self.client.search(
                index=self.document_index,
                body=stats_body
            )
            
            aggs = response['aggregations']
            
            return {
                'total_documents': aggs['total_docs']['value'],
                'by_language': {bucket['key']: bucket['doc_count'] 
                              for bucket in aggs['by_language']['buckets']},
                'top_authors': [(bucket['key'], bucket['doc_count']) 
                               for bucket in aggs['by_author']['buckets']],
                'popular_tags': [(bucket['key'], bucket['doc_count']) 
                                for bucket in aggs['by_tags']['buckets']],
                'monthly_distribution': [(bucket['key_as_string'], bucket['doc_count']) 
                                       for bucket in aggs['date_histogram']['buckets']]
            }
            
        except Exception as e:
            logger.error(f"Statistics query failed: {e}")
            return {}
    
    def delete_document(self, document_id: int) -> bool:
        """문서 삭제"""
        try:
            response = self.client.delete(
                index=self.document_index,
                id=document_id
            )
            logger.debug(f"Deleted document {document_id}: {response['result']}")
            return True
            
        except NotFoundError:
            logger.warning(f"Document {document_id} not found for deletion")
            return True  # 이미 없으므로 성공으로 간주
        except Exception as e:
            logger.error(f"Failed to delete document {document_id}: {e}")
            return False
    
    def bulk_index_documents(self, documents: List[Dict]) -> Dict:
        """문서 일괄 인덱싱"""
        from opensearchpy.helpers import bulk
        
        actions = []
        for doc in documents:
            # 한국어 처리
            if doc.get('content'):
                from app.utils.korean_text import process_korean_document
                korean_processing = process_korean_document(
                    doc.get('title', ''),
                    doc.get('content', '')
                )
                doc.update(korean_processing)
            
            action = {
                "_index": self.document_index,
                "_id": doc.get('id'),
                "_source": doc
            }
            actions.append(action)
        
        try:
            success_count, failed_items = bulk(
                self.client,
                actions,
                chunk_size=100,
                request_timeout=60
            )
            
            return {
                'success_count': success_count,
                'failed_count': len(failed_items),
                'failed_items': failed_items
            }
            
        except Exception as e:
            # SECURITY: Log detailed error but return generic message
            logger.error(f"Bulk indexing failed: {e}", exc_info=True)
            return {
                'success_count': 0,
                'failed_count': len(documents),
                'error': 'Indexing failed. Please try again later.'
            }
    
    def health_check(self) -> Dict:
        """OpenSearch 클러스터 상태 확인"""
        try:
            health = self.client.cluster.health()
            indices_info = self.client.cat.indices(format='json')
            
            return {
                'status': health['status'],
                'cluster_name': health['cluster_name'],
                'number_of_nodes': health['number_of_nodes'],
                'indices': [
                    {
                        'index': idx['index'],
                        'docs_count': idx.get('docs.count', 0),
                        'store_size': idx.get('store.size', '0b')
                    }
                    for idx in indices_info
                    if idx['index'].startswith('minky_')
                ]
            }
            
        except Exception as e:
            # SECURITY: Log detailed error but return generic message
            logger.error(f"Health check failed: {e}", exc_info=True)
            return {'status': 'error', 'error': 'Health check failed.'}

# 전역 인스턴스 (설정에 따라 초기화)
opensearch_service = None

def initialize_opensearch(config: Dict):
    """OpenSearch 서비스 초기화"""
    global opensearch_service

    try:
        username = config.get('username')
        password = config.get('password')
        use_ssl = config.get('use_ssl', False)

        # Validate credentials are either both provided or both None
        if (username and not password) or (password and not username):
            logger.error("OpenSearch credentials incomplete - need both username and password")
            return False

        # Security warning: credentials without SSL is insecure
        if username and password and not use_ssl:
            logger.warning("OpenSearch credentials provided without SSL - connection may be insecure")

        opensearch_service = OpenSearchService(
            host=config.get('host', 'localhost'),
            port=config.get('port', 9200),
            use_ssl=use_ssl,
            username=username,
            password=password,
            verify_certs=config.get('verify_certs')
        )
        
        # 인덱스 생성
        opensearch_service.create_document_index()
        opensearch_service.create_org_roam_index()
        
        logger.info("OpenSearch service initialized successfully")
        return True
        
    except Exception as e:
        logger.error(f"Failed to initialize OpenSearch service: {e}")
        return False

def get_opensearch_service() -> Optional[OpenSearchService]:
    """OpenSearch 서비스 인스턴스 반환"""
    return opensearch_service