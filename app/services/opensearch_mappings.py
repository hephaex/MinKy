"""
OpenSearch Index Mappings and Settings
Defines index configurations for document and org-roam indices
"""

from typing import Dict, Any


def get_document_index_settings() -> Dict[str, Any]:
    """Get index settings for document index (Korean analyzer support)"""
    return {
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
                "id": {"type": "integer"},
                "title": {
                    "type": "text",
                    "fields": {
                        "korean": {"type": "text", "analyzer": "korean_analyzer"},
                        "mixed": {"type": "text", "analyzer": "mixed_analyzer"},
                        "keyword": {"type": "keyword"}
                    }
                },
                "content": {
                    "type": "text",
                    "fields": {
                        "korean": {"type": "text", "analyzer": "korean_analyzer"},
                        "mixed": {"type": "text", "analyzer": "mixed_analyzer"}
                    }
                },
                "author": {
                    "type": "text",
                    "fields": {"keyword": {"type": "keyword"}}
                },
                "tags": {"type": "keyword"},
                "language": {"type": "keyword"},
                "user_id": {"type": "integer"},
                "is_public": {"type": "boolean"},
                "is_published": {"type": "boolean"},
                "created_at": {"type": "date"},
                "updated_at": {"type": "date"},
                "published_at": {"type": "date"},
                "keywords": {
                    "type": "nested",
                    "properties": {
                        "word": {"type": "keyword"},
                        "pos": {"type": "keyword"},
                        "count": {"type": "integer"}
                    }
                },
                "metadata": {"type": "object", "enabled": True},
                "search_vector": {"type": "text", "analyzer": "korean_analyzer"}
            }
        }
    }


def get_org_roam_index_settings() -> Dict[str, Any]:
    """Get index settings for org-roam document index"""
    return {
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
                "content": {"type": "text", "analyzer": "korean_analyzer"},
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
