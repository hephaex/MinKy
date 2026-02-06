"""
ML Analytics API Routes
Provides endpoints for machine learning-powered document analytics
"""

from flask import Blueprint, request, jsonify, Response
from flask_jwt_extended import jwt_required
from app.services.ml_analytics_service import ml_analytics_service
from app.models.document import Document
from app.utils.auth import get_optional_user_id
from app import limiter
import logging

logger = logging.getLogger(__name__)

ml_analytics_bp = Blueprint('ml_analytics', __name__)

@ml_analytics_bp.route('/ml-analytics/status', methods=['GET'])
def get_ml_analytics_status() -> Response | tuple[Response, int]:
    """
    Get ML analytics service status and capabilities
    """
    try:
        status = {
            'available': ml_analytics_service.is_available(),
            'sklearn_available': ml_analytics_service.sklearn_available,
            'nltk_available': ml_analytics_service.nltk_available,
            'textblob_available': ml_analytics_service.textblob_available,
            'features': {
                'document_insights': ml_analytics_service.is_available(),
                'corpus_analysis': ml_analytics_service.sklearn_available,
                'clustering': ml_analytics_service.sklearn_available,
                'topic_modeling': ml_analytics_service.sklearn_available,
                'sentiment_analysis': ml_analytics_service.textblob_available or True,  # fallback available
                'similarity_analysis': ml_analytics_service.sklearn_available,
                'content_analysis': ml_analytics_service.nltk_available or True,  # basic analysis available
                'trend_analysis': True,
                'collaboration_analysis': True
            }
        }
        
        return jsonify({
            'success': True,
            'status': status
        })
        
    except Exception as e:
        logger.error(f"Error getting ML analytics status: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to get ML analytics status'
        }), 500

@ml_analytics_bp.route('/ml-analytics/document/<int:document_id>/insights', methods=['GET'])
@jwt_required(optional=True)
def get_document_insights(document_id: int) -> Response | tuple[Response, int]:
    """
    Get comprehensive ML insights for a specific document
    """
    try:
        if not ml_analytics_service.is_available():
            return jsonify({
                'success': False,
                'error': 'ML analytics service is not available'
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
        
        # Generate insights
        insights = ml_analytics_service.get_document_insights(document_id)
        
        if 'error' in insights:
            return jsonify({
                'success': False,
                'error': insights['error']
            }), 400
        
        return jsonify({
            'success': True,
            'insights': insights
        })
        
    except Exception as e:
        logger.error(f"Error getting document insights: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to generate document insights'
        }), 500

@ml_analytics_bp.route('/ml-analytics/corpus/insights', methods=['GET'])
@limiter.limit("10 per hour")
@jwt_required(optional=True)
def get_corpus_insights() -> Response | tuple[Response, int]:
    """
    Get ML insights for the entire document corpus or user's documents
    """
    try:
        if not ml_analytics_service.is_available():
            return jsonify({
                'success': False,
                'error': 'ML analytics service is not available'
            }), 503
        
        user_id = get_optional_user_id()
        
        # Get query parameters
        scope = request.args.get('scope', 'user')  # 'user' or 'public'
        
        # Determine which documents to analyze
        analysis_user_id = None
        if scope == 'user' and user_id:
            analysis_user_id = user_id
        elif scope == 'public':
            analysis_user_id = None  # Analyze all public documents
        else:
            # Default to user's documents if authenticated, public otherwise
            analysis_user_id = user_id
        
        # Generate corpus insights
        insights = ml_analytics_service.get_corpus_insights(analysis_user_id)
        
        if 'error' in insights:
            return jsonify({
                'success': False,
                'error': insights['error']
            }), 400
        
        return jsonify({
            'success': True,
            'insights': insights,
            'scope': scope
        })
        
    except Exception as e:
        logger.error(f"Error getting corpus insights: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to generate corpus insights'
        }), 500

@ml_analytics_bp.route('/ml-analytics/document/<int:document_id>/similar', methods=['GET'])
@jwt_required(optional=True)
def get_similar_documents(document_id: int) -> Response | tuple[Response, int]:
    """
    Get documents similar to the specified document
    """
    try:
        if not ml_analytics_service.sklearn_available:
            return jsonify({
                'success': False,
                'error': 'Similarity analysis requires ML libraries'
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
        
        # Get similarity analysis
        similarity_data = ml_analytics_service._get_document_similarities(document)
        
        if 'error' in similarity_data:
            return jsonify({
                'success': False,
                'error': similarity_data['error']
            }), 400
        
        return jsonify({
            'success': True,
            'document_id': document_id,
            'similar_documents': similarity_data['similar_documents']
        })
        
    except Exception as e:
        logger.error(f"Error getting similar documents: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to find similar documents'
        }), 500

@ml_analytics_bp.route('/ml-analytics/document/<int:document_id>/sentiment', methods=['GET'])
@jwt_required(optional=True)
def get_document_sentiment(document_id: int) -> Response | tuple[Response, int]:
    """
    Get sentiment analysis for a specific document
    """
    try:
        # Check if document exists and user has access
        document = Document.query.get_or_404(document_id)
        
        user_id = get_optional_user_id()
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # Get sentiment analysis
        sentiment_data = ml_analytics_service._get_sentiment_analysis(document)
        
        return jsonify({
            'success': True,
            'document_id': document_id,
            'sentiment': sentiment_data
        })
        
    except Exception as e:
        logger.error(f"Error getting document sentiment: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to analyze document sentiment'
        }), 500

@ml_analytics_bp.route('/ml-analytics/document/<int:document_id>/recommendations', methods=['GET'])
@jwt_required(optional=True)
def get_document_recommendations(document_id: int) -> Response | tuple[Response, int]:
    """
    Get improvement recommendations for a specific document
    """
    try:
        # Check if document exists and user has access
        document = Document.query.get_or_404(document_id)
        
        user_id = get_optional_user_id()
        
        # Check access permissions
        if not document.is_public and (not user_id or document.user_id != user_id):
            return jsonify({
                'success': False,
                'error': 'Access denied'
            }), 403
        
        # Get recommendations
        recommendations = ml_analytics_service._get_document_recommendations(document)
        
        return jsonify({
            'success': True,
            'document_id': document_id,
            'recommendations': recommendations
        })
        
    except Exception as e:
        logger.error(f"Error getting document recommendations: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to generate document recommendations'
        }), 500

@ml_analytics_bp.route('/ml-analytics/corpus/clustering', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required(optional=True)
def perform_document_clustering() -> Response | tuple[Response, int]:
    """
    Perform clustering analysis on documents
    """
    try:
        if not ml_analytics_service.sklearn_available:
            return jsonify({
                'success': False,
                'error': 'Clustering requires ML libraries'
            }), 503
        
        user_id = get_optional_user_id()
        
        # Get request parameters
        data = request.get_json() or {}
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 100)
        
        # Get documents for clustering
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
                'error': 'Insufficient documents for clustering (minimum 3 required)'
            }), 400
        
        # Perform clustering
        clustering_results = ml_analytics_service._perform_document_clustering(documents)
        
        if 'error' in clustering_results:
            return jsonify({
                'success': False,
                'error': clustering_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'clustering': clustering_results,
            'scope': scope,
            'documents_analyzed': len(documents)
        })
        
    except Exception as e:
        logger.error(f"Error performing document clustering: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to perform document clustering'
        }), 500

@ml_analytics_bp.route('/ml-analytics/corpus/topics', methods=['POST'])
@limiter.limit("5 per hour")
@jwt_required(optional=True)
def perform_topic_modeling() -> Response | tuple[Response, int]:
    """
    Perform topic modeling on documents
    """
    try:
        if not ml_analytics_service.sklearn_available:
            return jsonify({
                'success': False,
                'error': 'Topic modeling requires ML libraries'
            }), 503
        
        user_id = get_optional_user_id()
        
        # Get request parameters
        data = request.get_json() or {}
        scope = data.get('scope', 'user')
        max_documents = data.get('max_documents', 100)
        
        # Get documents for topic modeling
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
        
        if len(documents) < 5:
            return jsonify({
                'success': False,
                'error': 'Insufficient documents for topic modeling (minimum 5 required)'
            }), 400
        
        # Perform topic modeling
        topic_results = ml_analytics_service._perform_topic_modeling(documents)
        
        if 'error' in topic_results:
            return jsonify({
                'success': False,
                'error': topic_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'topics': topic_results,
            'scope': scope,
            'documents_analyzed': len(documents)
        })
        
    except Exception as e:
        logger.error(f"Error performing topic modeling: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to perform topic modeling'
        }), 500

@ml_analytics_bp.route('/ml-analytics/trends', methods=['GET'])
@jwt_required(optional=True)
def get_document_trends() -> Response | tuple[Response, int]:
    """
    Get document creation and content trends
    """
    try:
        user_id = get_optional_user_id()
        
        # Get query parameters
        scope = request.args.get('scope', 'user')
        days = int(request.args.get('days', 30))
        
        # Get documents for trend analysis
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
        
        # Filter by date range if specified
        if days > 0:
            from datetime import datetime, timedelta, timezone
            start_date = datetime.now(timezone.utc) - timedelta(days=days)
            query = query.filter(Document.created_at >= start_date)

        # Add limit to prevent loading excessive data
        max_documents = 2000
        documents = query.limit(max_documents).all()
        
        if not documents:
            return jsonify({
                'success': False,
                'error': 'No documents found for trend analysis'
            }), 400
        
        # Perform trend analysis
        trend_results = ml_analytics_service._analyze_document_trends(documents)
        
        if 'error' in trend_results:
            return jsonify({
                'success': False,
                'error': trend_results['error']
            }), 400
        
        return jsonify({
            'success': True,
            'trends': trend_results,
            'scope': scope,
            'time_period_days': days
        })
        
    except Exception as e:
        logger.error(f"Error getting document trends: {e}")
        return jsonify({
            'success': False,
            'error': 'Failed to analyze document trends'
        }), 500