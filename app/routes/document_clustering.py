"""
Document Clustering API Routes
Provides endpoints for document clustering and similarity detection
"""

from flask import Blueprint, request, jsonify
from flask_jwt_extended import jwt_required
from app.services.document_clustering_service import document_clustering_service
from app.models.document import Document
from app.utils.auth import get_current_user_id
import logging

logger = logging.getLogger(__name__)

clustering_bp = Blueprint('clustering', __name__)

@clustering_bp.route('/clustering/status', methods=['GET'])
def get_clustering_status():
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
@jwt_required(optional=True)
def cluster_documents():
    """
    Cluster documents using machine learning algorithms
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Document clustering service is not available'
            }), 503
        
        user_id = None
        try:
            user_id = get_current_user_id()
        except:
            pass
        
        # Get request parameters
        data = request.get_json() or {}
        method = data.get('method', 'auto')
        n_clusters = data.get('n_clusters')
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 100)
        document_ids = data.get('document_ids')  # Specific documents to cluster
        
        # Validate method
        if method not in ['kmeans', 'hierarchical', 'dbscan', 'auto']:
            return jsonify({
                'success': False,
                'error': 'Invalid clustering method'
            }), 400
        
        # Get documents to cluster
        if document_ids:
            # Cluster specific documents
            documents = Document.query.filter(Document.id.in_(document_ids)).all()
        else:
            # Cluster based on scope
            query = Document.query
            if scope == 'user' and user_id:
                query = query.filter(Document.user_id == user_id)
            elif scope == 'public':
                query = query.filter(Document.is_public == True)
            else:
                # Default behavior
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
@jwt_required(optional=True)
def find_similar_documents(document_id):
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
        
        user_id = None
        try:
            user_id = get_current_user_id()
        except:
            pass
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # Get query parameters
        similarity_threshold = float(request.args.get('threshold', 0.1))
        max_results = int(request.args.get('max_results', 10))
        scope = request.args.get('scope', 'all')
        
        # Get candidate documents
        candidate_documents = None
        if scope == 'user' and user_id:
            candidate_documents = Document.query.filter(
                Document.user_id == user_id,
                Document.id != document_id
            ).all()
        elif scope == 'public':
            candidate_documents = Document.query.filter(
                Document.is_public == True,
                Document.id != document_id
            ).all()
        # If scope == 'all', candidate_documents stays None (use all accessible documents)
        
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
@jwt_required(optional=True)
def detect_duplicates():
    """
    Detect potential duplicate documents
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Duplicate detection service is not available'
            }), 503
        
        user_id = None
        try:
            user_id = get_current_user_id()
        except:
            pass
        
        # Get request parameters
        data = request.get_json() or {}
        similarity_threshold = data.get('threshold', 0.8)
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 500)
        
        # Get documents to check for duplicates
        query = Document.query
        if scope == 'user' and user_id:
            query = query.filter(Document.user_id == user_id)
        elif scope == 'public':
            query = query.filter(Document.is_public == True)
        else:
            # Default behavior
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

@clustering_bp.route('/clustering/batch-similarity', methods=['POST'])
@jwt_required(optional=True)
def batch_similarity_analysis():
    """
    Perform similarity analysis on a batch of documents
    """
    try:
        if not document_clustering_service.is_available():
            return jsonify({
                'success': False,
                'error': 'Similarity analysis service is not available'
            }), 503
        
        user_id = None
        try:
            user_id = get_current_user_id()
        except:
            pass
        
        # Get request parameters
        data = request.get_json() or {}
        document_ids = data.get('document_ids', [])
        similarity_threshold = data.get('threshold', 0.1)
        
        if not document_ids or len(document_ids) < 2:
            return jsonify({
                'success': False,
                'error': 'At least 2 document IDs required'
            }), 400
        
        # Get documents
        documents = Document.query.filter(Document.id.in_(document_ids)).all()
        
        # Check access permissions
        accessible_docs = []
        for doc in documents:
            if doc.is_public or (user_id and doc.user_id == user_id):
                accessible_docs.append(doc)
        
        if len(accessible_docs) < 2:
            return jsonify({
                'success': False,
                'error': 'Insufficient accessible documents'
            }), 403
        
        # Perform pairwise similarity analysis
        similarity_matrix = []
        
        for i, doc1 in enumerate(accessible_docs):
            similarity_row = []
            for j, doc2 in enumerate(accessible_docs):
                if i == j:
                    similarity_row.append({
                        'document_id': doc2.id,
                        'similarity_score': 1.0,
                        'is_self': True
                    })
                elif i < j:  # Avoid duplicate calculations
                    # Find similarity between doc1 and doc2
                    similarity_result = document_clustering_service.find_similar_documents(
                        target_document=doc1,
                        candidate_documents=[doc2],
                        similarity_threshold=0.0,  # Get all results
                        max_results=1
                    )
                    
                    if similarity_result.get('similar_documents'):
                        score = similarity_result['similar_documents'][0]['similarity_score']
                    else:
                        score = 0.0
                    
                    similarity_row.append({
                        'document_id': doc2.id,
                        'similarity_score': score,
                        'is_similar': score >= similarity_threshold
                    })
                else:
                    # Use symmetric property
                    score = similarity_matrix[j][i]['similarity_score']
                    similarity_row.append({
                        'document_id': doc2.id,
                        'similarity_score': score,
                        'is_similar': score >= similarity_threshold
                    })
            
            similarity_matrix.append(similarity_row)
        
        # Create document metadata
        document_metadata = []
        for doc in accessible_docs:
            document_metadata.append({
                'id': doc.id,
                'title': doc.title,
                'author': doc.author,
                'created_at': doc.created_at.isoformat()
            })
        
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
@jwt_required(optional=True)
def get_clustering_recommendations(document_id):
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
        
        user_id = None
        try:
            user_id = get_current_user_id()
        except:
            pass
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # Get similar documents for recommendations
        similarity_results = document_clustering_service.find_similar_documents(
            target_document=document,
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