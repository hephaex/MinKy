"""
Document Clustering API Routes
Provides endpoints for document clustering and similarity detection
"""

from flask import Blueprint, request, jsonify, Response
from flask_jwt_extended import jwt_required
from app.services.document_clustering_service import document_clustering_service
from app.models.document import Document
from app.utils.auth import get_optional_user_id
from app.utils.constants import MAX_SIMILAR_DOCUMENTS
from app import limiter
import logging

logger = logging.getLogger(__name__)

clustering_bp = Blueprint('clustering', __name__)

@clustering_bp.route('/clustering/status', methods=['GET'])
@limiter.limit("60 per minute")
def get_clustering_status() -> Response | tuple[Response, int]:
    """
    Get document clustering service status and capabilities
    """
    try:
        status = {
            'available': document_clustering_service.is_available(),
            'sklearn_available': document_clustering_service.sklearn_available,
            'supported_methods': ['kmeans', 'hierarchical', 'dbscan', 'auto'],
            'features': {
                'document_clustering': document_clustering_service.sklearn_available,
                'similarity_detection': document_clustering_service.sklearn_available,
                'duplicate_detection': document_clustering_service.sklearn_available,
                'cluster_insights': document_clustering_service.sklearn_available
            }
        }
        
        return jsonify({
            'success': True,
            'status': status
        })
        
    except Exception as e:
        logger.error(f"Error getting clustering status: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to get clustering status'
        }), 500

@clustering_bp.route('/clustering/cluster', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required(optional=True)
def cluster_documents() -> Response | tuple[Response, int]:
    """
    Cluster documents using machine learning algorithms
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Document clustering service is not available'
            }), 503
        
        user_id = get_optional_user_id()
        
        # Get request parameters
        data = request.get_json() or {}
        method = data.get('method', 'auto')
        n_clusters = data.get('n_clusters')
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 100)
        document_ids = data.get('document_ids')  # Specific documents to cluster

        # SECURITY: Validate n_clusters to prevent resource exhaustion
        MAX_CLUSTERS = 100
        if n_clusters is not None:
            if not isinstance(n_clusters, int) or n_clusters < 2 or n_clusters > MAX_CLUSTERS:
                return jsonify({
                    'success': False,
                    'error': f'n_clusters must be an integer between 2 and {MAX_CLUSTERS}'
                }), 400

        # SECURITY: Validate max_documents to prevent resource exhaustion
        MAX_CLUSTERING_DOCUMENTS = 500
        max_documents = min(max_documents if isinstance(max_documents, int) else 100, MAX_CLUSTERING_DOCUMENTS)
        if max_documents < 1:
            max_documents = 100

        # SECURITY: Validate scope
        ALLOWED_SCOPES = ['user', 'public', 'all']
        if scope not in ALLOWED_SCOPES:
            scope = 'user'

        # SECURITY: Validate document_ids array
        MAX_DOCUMENT_IDS = 200
        if document_ids:
            if not isinstance(document_ids, list) or len(document_ids) > MAX_DOCUMENT_IDS:
                return jsonify({
                    'success': False,
                    'error': f'document_ids must be an array with maximum {MAX_DOCUMENT_IDS} items'
                }), 400
            if not all(isinstance(id, int) and id > 0 for id in document_ids):
                return jsonify({
                    'success': False,
                    'error': 'All document_ids must be positive integers'
                }), 400

        # Validate method
        if method not in ['kmeans', 'hierarchical', 'dbscan', 'auto']:
            return jsonify({
                'success': False,
                'error': 'Invalid clustering method'
            }), 400
        
        # Get documents to cluster
        if document_ids:
            # Cluster specific documents - filter by access permissions
            documents = Document.query.filter(Document.id.in_(document_ids)).all()
            documents = _filter_accessible_documents(documents, user_id)
        else:
            # Cluster based on scope with proper authorization
            from sqlalchemy import or_
            query = Document.query
            if scope == 'user' and user_id:
                query = query.filter(Document.user_id == user_id)
            elif scope == 'public':
                query = query.filter(Document.is_public == True)
            elif scope == 'all':
                # SECURITY: 'all' means accessible documents (public + user's own), not ALL in DB
                if user_id:
                    query = query.filter(or_(Document.is_public == True, Document.user_id == user_id))
                else:
                    query = query.filter(Document.is_public == True)
            else:
                # Default behavior (unknown scope falls back to user's documents or public)
                if user_id:
                    query = query.filter(Document.user_id == user_id)
                else:
                    query = query.filter(Document.is_public == True)

            documents = query.limit(max_documents).all()
        
        if len(documents) < 3:
            return jsonify({
                'success': False,
                'error': 'Minimum 3 documents required for clustering'
            }), 400
        
        # Perform clustering
        clustering_results = document_clustering_service.cluster_documents(
            documents=documents,
            method=method,
            n_clusters=n_clusters
        )
        
        if 'error' in clustering_results:
            return jsonify({
                'success': False,
                'error': clustering_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'clustering': clustering_results,
            'scope': scope,
            'method': method
        })
        
    except Exception as e:
        logger.error(f"Error clustering documents: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to cluster documents'
        }), 500

@clustering_bp.route('/clustering/similar/<int:document_id>', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required(optional=True)
def find_similar_documents(document_id: int) -> Response | tuple[Response, int]:
    """
    Find documents similar to the specified document
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Document similarity service is not available'
            }), 503
        
        # Check if document exists and user has access
        document = Document.query.get_or_404(document_id)
        
        user_id = get_optional_user_id()
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # Get query parameters with validation
        try:
            similarity_threshold = float(request.args.get('threshold', 0.1))
        except (ValueError, TypeError):
            similarity_threshold = 0.1

        # SECURITY: Validate similarity_threshold range
        if similarity_threshold < 0.0 or similarity_threshold > 1.0:
            return jsonify({
                'success': False,
                'error': 'threshold must be between 0.0 and 1.0'
            }), 400

        max_results = min(request.args.get('max_results', 10, type=int), MAX_SIMILAR_DOCUMENTS)
        scope = request.args.get('scope', 'all')

        # SECURITY: Validate scope
        if scope not in ['user', 'public', 'all']:
            scope = 'all'
        
        # SECURITY: Get candidate documents with proper authorization
        # MAX_CANDIDATE_DOCUMENTS prevents resource exhaustion
        MAX_CANDIDATE_DOCUMENTS = 1000
        if scope == 'user' and user_id:
            candidate_documents = Document.query.filter(
                Document.user_id == user_id,
                Document.id != document_id
            ).limit(MAX_CANDIDATE_DOCUMENTS).all()
        elif scope == 'public':
            candidate_documents = Document.query.filter(
                Document.is_public == True,
                Document.id != document_id
            ).limit(MAX_CANDIDATE_DOCUMENTS).all()
        else:
            # SECURITY: scope='all' means accessible documents only (public + user's own)
            # NOT all documents in the database (which would be IDOR)
            from sqlalchemy import or_
            if user_id:
                candidate_documents = Document.query.filter(
                    Document.id != document_id,
                    or_(Document.is_public == True, Document.user_id == user_id)
                ).limit(MAX_CANDIDATE_DOCUMENTS).all()
            else:
                # Unauthenticated: only public documents
                candidate_documents = Document.query.filter(
                    Document.is_public == True,
                    Document.id != document_id
                ).limit(MAX_CANDIDATE_DOCUMENTS).all()
        
        # Find similar documents
        similarity_results = document_clustering_service.find_similar_documents(
            target_document=document,
            candidate_documents=candidate_documents,
            similarity_threshold=similarity_threshold,
            max_results=max_results
        )
        
        if 'error' in similarity_results:
            return jsonify({
                'success': False,
                'error': similarity_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'similarity': similarity_results,
            'scope': scope
        })
        
    except Exception as e:
        logger.error(f"Error finding similar documents: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to find similar documents'
        }), 500

@clustering_bp.route('/clustering/duplicates', methods=['POST'])
@limiter.limit("10 per hour")
@jwt_required(optional=True)
def detect_duplicates() -> Response | tuple[Response, int]:
    """
    Detect potential duplicate documents
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Duplicate detection service is not available'
            }), 503
        
        user_id = get_optional_user_id()
        
        # Get request parameters
        data = request.get_json() or {}
        similarity_threshold = data.get('threshold', 0.8)
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 500)

        # SECURITY: Validate similarity_threshold range
        try:
            similarity_threshold = float(similarity_threshold)
        except (ValueError, TypeError):
            similarity_threshold = 0.8

        if similarity_threshold < 0.0 or similarity_threshold > 1.0:
            return jsonify({
                'success': False,
                'error': 'threshold must be between 0.0 and 1.0'
            }), 400

        # SECURITY: Validate scope
        if scope not in ['user', 'public', 'all']:
            scope = 'user'

        # SECURITY: Validate max_documents to prevent resource exhaustion
        MAX_DUPLICATE_DOCUMENTS = 500
        try:
            max_documents = int(max_documents)
        except (ValueError, TypeError):
            max_documents = 500
        max_documents = min(max(1, max_documents), MAX_DUPLICATE_DOCUMENTS)

        # Get documents to check for duplicates with proper authorization
        from sqlalchemy import or_
        query = Document.query
        if scope == 'user' and user_id:
            query = query.filter(Document.user_id == user_id)
        elif scope == 'public':
            query = query.filter(Document.is_public == True)
        elif scope == 'all':
            # SECURITY: 'all' means accessible documents (public + user's own), not ALL in DB
            if user_id:
                query = query.filter(or_(Document.is_public == True, Document.user_id == user_id))
            else:
                query = query.filter(Document.is_public == True)
        else:
            # Default behavior (unknown scope falls back to user's documents or public)
            if user_id:
                query = query.filter(Document.user_id == user_id)
            else:
                query = query.filter(Document.is_public == True)

        documents = query.limit(max_documents).all()
        
        if len(documents) < 2:
            return jsonify({
                'success': False,
                'error': 'Minimum 2 documents required for duplicate detection'
            }), 400
        
        # Detect duplicates
        duplicate_results = document_clustering_service.detect_document_duplicates(
            documents=documents,
            similarity_threshold=similarity_threshold
        )
        
        if 'error' in duplicate_results:
            return jsonify({
                'success': False,
                'error': duplicate_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'duplicates': duplicate_results,
            'scope': scope,
            'threshold': similarity_threshold
        })
        
    except Exception as e:
        logger.error(f"Error detecting duplicates: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to detect duplicates'
        }), 500

def _filter_accessible_documents(documents: list[Document], user_id: int | None) -> list[Document]:
    """Filter documents based on user access permissions"""
    accessible_docs = []
    for doc in documents:
        if doc.is_public or (user_id and doc.user_id == user_id):
            accessible_docs.append(doc)
    return accessible_docs


def _calculate_similarity_score(doc1: Document, doc2: Document) -> float:
    """Calculate similarity score between two documents"""
    similarity_result = document_clustering_service.find_similar_documents(
        target_document=doc1,
        candidate_documents=[doc2],
        similarity_threshold=0.0,
        max_results=1
    )

    if similarity_result.get('similar_documents'):
        return similarity_result['similar_documents'][0]['similarity_score']

    return 0.0


def _build_similarity_row(
    doc1_index: int,
    accessible_docs: list[Document],
    similarity_matrix: list[list[dict]],
    similarity_threshold: float
) -> list[dict]:
    """Build a single row of the similarity matrix"""
    similarity_row = []

    for j, doc2 in enumerate(accessible_docs):
        if doc1_index == j:
            similarity_row.append({
                'document_id': doc2.id,
                'similarity_score': 1.0,
                'is_self': True
            })
        elif doc1_index < j:
            score = _calculate_similarity_score(accessible_docs[doc1_index], doc2)
            similarity_row.append({
                'document_id': doc2.id,
                'similarity_score': score,
                'is_similar': score >= similarity_threshold
            })
        else:
            score = similarity_matrix[j][doc1_index]['similarity_score']
            similarity_row.append({
                'document_id': doc2.id,
                'similarity_score': score,
                'is_similar': score >= similarity_threshold
            })

    return similarity_row


def _build_similarity_matrix(accessible_docs: list[Document], similarity_threshold: float) -> list[list[dict]]:
    """Build complete similarity matrix for all documents"""
    similarity_matrix = []

    for i in range(len(accessible_docs)):
        similarity_row = _build_similarity_row(i, accessible_docs, similarity_matrix, similarity_threshold)
        similarity_matrix.append(similarity_row)

    return similarity_matrix


def _extract_document_metadata(documents: list[Document]) -> list[dict]:
    """Extract metadata from documents"""
    return [
        {
            'id': doc.id,
            'title': doc.title,
            'author': doc.author,
            'created_at': doc.created_at.isoformat()
        }
        for doc in documents
    ]


@clustering_bp.route('/clustering/batch-similarity', methods=['POST'])
@limiter.limit("10 per hour")
@jwt_required(optional=True)
def batch_similarity_analysis() -> Response | tuple[Response, int]:
    """
    Perform similarity analysis on a batch of documents
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Similarity analysis service is not available'
            }), 503

        user_id = get_optional_user_id()

        data = request.get_json() or {}
        document_ids = data.get('document_ids', [])
        similarity_threshold = data.get('threshold', 0.1)

        # SECURITY: Validate similarity_threshold range
        try:
            similarity_threshold = float(similarity_threshold)
        except (ValueError, TypeError):
            similarity_threshold = 0.1

        if similarity_threshold < 0.0 or similarity_threshold > 1.0:
            return jsonify({
                'success': False,
                'error': 'threshold must be between 0.0 and 1.0'
            }), 400

        # SECURITY: Validate document_ids array size
        MAX_BATCH_DOCUMENTS = 100
        if len(document_ids) > MAX_BATCH_DOCUMENTS:
            return jsonify({
                'success': False,
                'error': f'Maximum {MAX_BATCH_DOCUMENTS} documents allowed for batch analysis'
            }), 400

        if not document_ids or len(document_ids) < 2:
            return jsonify({
                'success': False,
                'error': 'At least 2 document IDs required'
            }), 400

        # SECURITY: Validate document_ids are positive integers within valid range
        MAX_DOCUMENT_ID = 2**31 - 1  # 32-bit signed integer max
        if not isinstance(document_ids, list):
            return jsonify({
                'success': False,
                'error': 'document_ids must be an array'
            }), 400
        if not all(isinstance(id, int) and 0 < id <= MAX_DOCUMENT_ID for id in document_ids):
            return jsonify({
                'success': False,
                'error': 'All document_ids must be positive integers within valid range'
            }), 400

        documents = Document.query.filter(Document.id.in_(document_ids)).all()
        accessible_docs = _filter_accessible_documents(documents, user_id)

        if len(accessible_docs) < 2:
            return jsonify({
                'success': False,
                'error': 'Insufficient accessible documents'
            }), 403

        similarity_matrix = _build_similarity_matrix(accessible_docs, similarity_threshold)
        document_metadata = _extract_document_metadata(accessible_docs)

        return jsonify({
            'success': True,
            'batch_similarity': {
                'documents': document_metadata,
                'similarity_matrix': similarity_matrix,
                'threshold': similarity_threshold,
                'documents_analyzed': len(accessible_docs)
            }
        })

    except Exception as e:
        logger.error(f"Error in batch similarity analysis: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to perform batch similarity analysis'
        }), 500

@clustering_bp.route('/clustering/recommendations/<int:document_id>', methods=['GET'])
@limiter.limit("30 per minute")
@jwt_required(optional=True)
def get_clustering_recommendations(document_id: int) -> Response | tuple[Response, int]:
    """
    Get clustering-based recommendations for a document
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Clustering recommendations service is not available'
            }), 503
        
        # Check if document exists and user has access
        document = Document.query.get_or_404(document_id)
        
        user_id = get_optional_user_id()
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # SECURITY: Get candidate documents with proper authorization
        # This prevents IDOR by ensuring only accessible documents are considered
        from sqlalchemy import or_
        MAX_CANDIDATE_DOCUMENTS = 1000
        if user_id:
            candidate_documents = Document.query.filter(
                Document.id != document_id,
                or_(Document.is_public == True, Document.user_id == user_id)
            ).limit(MAX_CANDIDATE_DOCUMENTS).all()
        else:
            candidate_documents = Document.query.filter(
                Document.is_public == True,
                Document.id != document_id
            ).limit(MAX_CANDIDATE_DOCUMENTS).all()

        # Get similar documents for recommendations
        similarity_results = document_clustering_service.find_similar_documents(
            target_document=document,
            candidate_documents=candidate_documents,
            similarity_threshold=0.1,
            max_results=20
        )
        
        if 'error' in similarity_results:
            return jsonify({
                'success': False,
                'error': similarity_results['error']
            }), 400
        
        # Generate recommendations based on similarity
        recommendations = {
            'related_documents': similarity_results['similar_documents'][:5],
            'suggested_tags': [],
            'content_suggestions': [],
            'collaboration_opportunities': []
        }
        
        # Extract suggested tags from similar documents
        similar_docs = similarity_results['similar_documents']
        all_tags = []
        for similar_doc in similar_docs:
            all_tags.extend(similar_doc.get('tags', []))
        
        if all_tags:
            from collections import Counter
            tag_counts = Counter(all_tags)
            recommendations['suggested_tags'] = [
                {'tag': tag, 'frequency': count, 'relevance': count / len(similar_docs)}
                for tag, count in tag_counts.most_common(5)
            ]
        
        # Suggest collaboration opportunities (same authors)
        authors = set()
        for similar_doc in similar_docs:
            if similar_doc.get('author') and similar_doc['author'] != document.author:
                authors.add(similar_doc['author'])
        
        recommendations['collaboration_opportunities'] = [
            {'author': author, 'reason': 'Has written similar content'}
            for author in list(authors)[:3]
        ]
        
        # Content improvement suggestions
        content_suggestions = []
        if len(similar_docs) > 0:
            avg_similarity = sum(doc['similarity_score'] for doc in similar_docs) / len(similar_docs)
            if avg_similarity < 0.3:
                content_suggestions.append({
                    'type': 'uniqueness',
                    'message': 'This document appears to be quite unique. Consider adding more context or references.',
                    'priority': 'low'
                })
            elif avg_similarity > 0.7:
                content_suggestions.append({
                    'type': 'differentiation',
                    'message': 'This document is very similar to existing content. Consider highlighting unique aspects.',
                    'priority': 'medium'
                })
        
        recommendations['content_suggestions'] = content_suggestions
        
        return jsonify({
            'success': True,
            'document_id': document_id,
            'recommendations': recommendations
        })
        
    except Exception as e:
        logger.error(f"Error getting clustering recommendations: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to generate clustering recommendations'
        }), 500