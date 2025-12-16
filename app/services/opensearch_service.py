import logging
from typing import Dict, List, Optional, Any
from opensearchpy import OpenSearch, RequestsHttpConnection
from opensearchpy.exceptions import NotFoundError
from app.utils.korean_text import KoreanTextProcessor

logger = logging.getLogger(__name__)

class OpenSearchService:
    """OpenSearch 기반 다국어 검색 서비스"""
    
    def __init__(self, host='localhost', port=9200, use_ssl=False, 
                 username=None, password=None):
        """
        OpenSearch 클라이언트 초기화
        
        Args:
            host: OpenSearch 호스트
            port: OpenSearch 포트
            use_ssl: SSL 사용 여부
            username: 인증 사용자명
            password: 인증 비밀번호
        """
        self.korean_processor = KoreanTextProcessor()
        
        auth = None
        if username and password:
            auth = (username, password)
        
        self.client = OpenSearch(
            hosts=[{'host': host, 'port': port}],
            http_auth=auth,
            use_ssl=use_ssl,
            verify_certs=False,  # 개발 환경에서는 False
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
        
        index_settings = {
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0,
                "analysis": {
                    "tokenizer": {
                        "korean_tokenizer": {
                            "type": "nori_tokenizer",
                            "decompound_mode": "mixed"
                        }
                    },
                    "analyzer": {
                        "korean_analyzer": {
                            "type": "custom",
                            "tokenizer": "korean_tokenizer",
                            "filter": [
                                "lowercase",
                                "nori_part_of_speech",
                                "nori_readingform"
                            ]
                        },
                        "mixed_analyzer": {
                            "type": "custom",
                            "tokenizer": "standard",
                            "filter": [
                                "lowercase",
                                "asciifolding"
                            ]
                        }
                    },
                    "filter": {
                        "nori_part_of_speech": {
                            "type": "nori_part_of_speech",
                            "stoptags": [
                                "E", "IC", "J", "MAG", "MAJ", 
                                "MM", "SP", "SSC", "SSO", "SC"
                            ]
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "id": {
                        "type": "integer"
                    },
                    "title": {
                        "type": "text",
                        "fields": {
                            "korean": {
                                "type": "text",
                                "analyzer": "korean_analyzer"
                            },
                            "mixed": {
                                "type": "text",
                                "analyzer": "mixed_analyzer"
                            },
                            "keyword": {
                                "type": "keyword"
                            }
                        }
                    },
                    "content": {
                        "type": "text",
                        "fields": {
                            "korean": {
                                "type": "text",
                                "analyzer": "korean_analyzer"
                            },
                            "mixed": {
                                "type": "text",
                                "analyzer": "mixed_analyzer"
                            }
                        }
                    },
                    "author": {
                        "type": "text",
                        "fields": {
                            "keyword": {
                                "type": "keyword"
                            }
                        }
                    },
                    "tags": {
                        "type": "keyword"
                    },
                    "language": {
                        "type": "keyword"
                    },
                    "user_id": {
                        "type": "integer"
                    },
                    "is_public": {
                        "type": "boolean"
                    },
                    "is_published": {
                        "type": "boolean"
                    },
                    "created_at": {
                        "type": "date"
                    },
                    "updated_at": {
                        "type": "date"
                    },
                    "published_at": {
                        "type": "date"
                    },
                    "keywords": {
                        "type": "nested",
                        "properties": {
                            "word": {"type": "keyword"},
                            "pos": {"type": "keyword"},
                            "count": {"type": "integer"}
                        }
                    },
                    "metadata": {
                        "type": "object",
                        "enabled": True
                    },
                    "search_vector": {
                        "type": "text",
                        "analyzer": "korean_analyzer"
                    }
                }
            }
        }
        
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
        
        index_settings = {
            "settings": {
                "number_of_shards": 1,
                "number_of_replicas": 0,
                "analysis": {
                    "analyzer": {
                        "korean_analyzer": {
                            "type": "custom",
                            "tokenizer": "nori_tokenizer",
                            "filter": ["lowercase", "nori_part_of_speech"]
                        }
                    }
                }
            },
            "mappings": {
                "properties": {
                    "id": {"type": "integer"},
                    "org_roam_id": {"type": "keyword"},
                    "title": {
                        "type": "text",
                        "fields": {
                            "korean": {"type": "text", "analyzer": "korean_analyzer"},
                            "keyword": {"type": "keyword"}
                        }
                    },
                    "content": {
                        "type": "text",
                        "analyzer": "korean_analyzer"
                    },
                    "filename": {"type": "keyword"},
                    "file_path": {"type": "keyword"},
                    "roam_tags": {"type": "keyword"},
                    "roam_aliases": {"type": "keyword"},
                    "tags": {"type": "keyword"},
                    "language": {"type": "keyword"},
                    "backlinks": {
                        "type": "nested",
                        "properties": {
                            "source_title": {"type": "text"},
                            "source_filename": {"type": "keyword"},
                            "link_text": {"type": "text"}
                        }
                    },
                    "outbound_links": {
                        "type": "nested",
                        "properties": {
                            "target_title": {"type": "text"},
                            "target_filename": {"type": "keyword"},
                            "link_text": {"type": "text"}
                        }
                    },
                    "created_at": {"type": "date"},
                    "modified_at": {"type": "date"}
                }
            }
        }
        
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
    
    def search_documents(self, query: str, filters: Optional[Dict] = None,
                        page: int = 1, per_page: int = 20,
                        user_id: Optional[int] = None) -> Dict:
        """문서 검색 (한국어 지원)"""

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
            logger.error(f"Search failed: {e}")
            return {
                'documents': [],
                'total': 0,
                'page': page,
                'per_page': per_page,
                'pages': 0,
                'error': str(e)
            }
    
    def suggest_tags(self, query: str, limit: int = 10) -> List[str]:
        """태그 자동완성 제안"""
        try:
            search_body = {
                "size": 0,
                "aggs": {
                    "tag_suggestions": {
                        "terms": {
                            "field": "tags",
                            "include": f".*{query}.*",
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
    
    def get_document_statistics(self) -> Dict:
        """문서 통계 정보"""
        try:
            stats_body = {
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
            logger.error(f"Bulk indexing failed: {e}")
            return {
                'success_count': 0,
                'failed_count': len(documents),
                'error': str(e)
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
            logger.error(f"Health check failed: {e}")
            return {'status': 'error', 'error': str(e)}

# 전역 인스턴스 (설정에 따라 초기화)
opensearch_service = None

def initialize_opensearch(config: Dict):
    """OpenSearch 서비스 초기화"""
    global opensearch_service
    
    try:
        opensearch_service = OpenSearchService(
            host=config.get('host', 'localhost'),
            port=config.get('port', 9200),
            use_ssl=config.get('use_ssl', False),
            username=config.get('username'),
            password=config.get('password')
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