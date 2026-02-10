"""
ML Analytics Service for advanced document insights
Provides machine learning-powered analytics and recommendations
"""

import logging
from typing import Dict, List, Optional, Any
from datetime import datetime, timezone
from app import db
from app.models.document import Document

from app.services.ml_document_analysis import (
    get_basic_document_stats,
    calculate_complexity_score,
    calculate_keyword_density,
    analyze_document_structure,
    analyze_language_patterns,
    nltk_content_analysis,
    simple_sentiment_analysis,
    get_document_recommendations,
)
from app.services.ml_corpus_analysis import (
    perform_document_clustering,
    perform_topic_modeling,
    analyze_document_trends,
    analyze_collaboration_patterns,
    analyze_content_evolution,
    calculate_performance_metrics,
)

logger = logging.getLogger(__name__)


class MLAnalyticsService:
    def __init__(self):
        self.sklearn_available = self._check_sklearn()
        self.nltk_available = self._check_nltk()
        self.textblob_available = self._check_textblob()

        if self.sklearn_available:
            self._initialize_sklearn_models()

        if self.nltk_available:
            self._initialize_nltk()

    def _check_sklearn(self) -> bool:
        """Check if scikit-learn is available"""
        try:
            import sklearn  # noqa: F401
            return True
        except ImportError:
            logger.warning("scikit-learn not available. ML features will be limited.")
            return False

    def _check_nltk(self) -> bool:
        """Check if NLTK is available"""
        try:
            import nltk  # noqa: F401
            return True
        except ImportError:
            logger.warning("NLTK not available. Natural language processing will be limited.")
            return False

    def _check_textblob(self) -> bool:
        """Check if TextBlob is available"""
        try:
            import textblob  # noqa: F401
            return True
        except ImportError:
            logger.warning("TextBlob not available. Sentiment analysis will be limited.")
            return False

    def _initialize_sklearn_models(self):
        """Initialize sklearn models"""
        try:
            from sklearn.feature_extraction.text import TfidfVectorizer

            self.tfidf_vectorizer = TfidfVectorizer(
                max_features=1000,
                stop_words='english',
                ngram_range=(1, 2),
                min_df=2
            )
            self.kmeans = None
            self.lda = None

        except ImportError as e:
            logger.error(f"Error initializing sklearn models: {e}")
            self.sklearn_available = False

    def _initialize_nltk(self):
        """Initialize NLTK resources"""
        try:
            import nltk
            try:
                nltk.data.find('tokenizers/punkt')
            except LookupError:
                nltk.download('punkt', quiet=True)

            try:
                nltk.data.find('corpora/stopwords')
            except LookupError:
                nltk.download('stopwords', quiet=True)

            try:
                nltk.data.find('taggers/averaged_perceptron_tagger')
            except LookupError:
                nltk.download('averaged_perceptron_tagger', quiet=True)

        except Exception as e:
            logger.error(f"Error initializing NLTK: {e}")
            self.nltk_available = False

    def is_available(self) -> bool:
        """Check if ML analytics service is available"""
        return bool(self.sklearn_available or self.nltk_available or self.textblob_available)

    def get_document_insights(self, document_id: int) -> Dict[str, Any]:
        """Get comprehensive ML insights for a specific document"""
        try:
            document = db.session.get(Document, document_id)
            if not document:
                return {'error': 'Document not found'}

            content = document.markdown_content or ''
            basic_stats = get_basic_document_stats(content)

            insights = {
                'document_id': document_id,
                'title': document.title,
                'basic_stats': basic_stats,
                'content_analysis': self._analyze_document_content(content),
                'similarity_analysis': self._get_document_similarities(document),
                'topic_analysis': self._get_document_topics(document),
                'sentiment_analysis': self._get_sentiment_analysis(content),
                'recommendations': get_document_recommendations(
                    basic_stats['word_count'],
                    basic_stats['header_count'],
                    basic_stats['reading_time_minutes'],
                    basic_stats['link_count']
                ),
                'generated_at': datetime.now(timezone.utc).isoformat()
            }

            return insights

        except Exception as e:
            logger.error(f"Error getting document insights: {e}")
            return {'error': f'Failed to generate insights: {str(e)}'}

    def get_corpus_insights(self, user_id: Optional[int] = None) -> Dict[str, Any]:
        """Get ML insights for the entire document corpus or user's documents"""
        try:
            query = Document.query
            if user_id:
                query = query.filter(Document.user_id == user_id)

            documents = query.all()

            if not documents:
                return {'error': 'No documents found'}

            insights = {
                'corpus_size': len(documents),
                'user_id': user_id,
                'cluster_analysis': perform_document_clustering(documents, self.sklearn_available),
                'topic_modeling': perform_topic_modeling(documents, self.sklearn_available),
                'trend_analysis': analyze_document_trends(documents),
                'collaboration_patterns': analyze_collaboration_patterns(documents, db.session),
                'content_evolution': analyze_content_evolution(documents, calculate_complexity_score),
                'performance_metrics': calculate_performance_metrics(
                    documents, db.session, get_basic_document_stats
                ),
                'generated_at': datetime.now(timezone.utc).isoformat()
            }

            return insights

        except Exception as e:
            logger.error(f"Error getting corpus insights: {e}")
            return {'error': f'Failed to generate corpus insights: {str(e)}'}

    def _analyze_document_content(self, content: str) -> Dict[str, Any]:
        """Analyze document content using NLP techniques"""
        analysis = {
            'complexity_score': calculate_complexity_score(content),
            'keyword_density': calculate_keyword_density(content),
            'structure_analysis': analyze_document_structure(content),
            'language_patterns': analyze_language_patterns(content)
        }

        if self.nltk_available:
            analysis.update(nltk_content_analysis(content, self.nltk_available))

        return analysis

    def _get_document_similarities(self, document: Document) -> Dict[str, Any]:
        """Find similar documents using TF-IDF and cosine similarity"""
        if not self.sklearn_available:
            return {'error': 'ML libraries not available'}

        try:
            from sklearn.feature_extraction.text import TfidfVectorizer
            from sklearn.metrics.pairwise import cosine_similarity

            all_docs = Document.query.filter(Document.id != document.id).limit(100).all()

            if not all_docs:
                return {'similar_documents': []}

            documents_text = [document.markdown_content or '']
            documents_text.extend([doc.markdown_content or '' for doc in all_docs])

            vectorizer = TfidfVectorizer(max_features=500, stop_words='english')
            tfidf_matrix = vectorizer.fit_transform(documents_text)

            similarities = cosine_similarity(tfidf_matrix[0:1], tfidf_matrix[1:]).flatten()

            similar_indices = similarities.argsort()[-5:][::-1]
            similar_docs = []

            for idx in similar_indices:
                if similarities[idx] > 0.1:
                    similar_doc = all_docs[idx]
                    similar_docs.append({
                        'id': similar_doc.id,
                        'title': similar_doc.title,
                        'similarity_score': float(similarities[idx]),
                        'author': similar_doc.author,
                        'created_at': similar_doc.created_at.isoformat()
                    })

            return {'similar_documents': similar_docs}

        except Exception as e:
            logger.error(f"Similarity analysis error: {e}")
            return {'error': f'Similarity analysis failed: {str(e)}'}

    def _get_document_topics(self, document: Document) -> Dict[str, Any]:
        """Extract topics from document content"""
        if not self.sklearn_available:
            return {'error': 'ML libraries not available'}

        try:
            from sklearn.feature_extraction.text import CountVectorizer

            content = document.markdown_content or ''
            if not content.strip():
                return {'topics': []}

            vectorizer = CountVectorizer(max_features=20, stop_words='english', ngram_range=(1, 2))
            doc_term_matrix = vectorizer.fit_transform([content])

            feature_names = vectorizer.get_feature_names_out()
            term_frequencies = doc_term_matrix.toarray()[0]

            top_indices = term_frequencies.argsort()[-10:][::-1]
            topics = []

            for idx in top_indices:
                if term_frequencies[idx] > 0:
                    topics.append({
                        'term': feature_names[idx],
                        'frequency': int(term_frequencies[idx]),
                        'relevance': float(term_frequencies[idx] / max(term_frequencies))
                    })

            return {'topics': topics}

        except Exception as e:
            logger.error(f"Topic analysis error: {e}")
            return {'error': f'Topic analysis failed: {str(e)}'}

    def _get_sentiment_analysis(self, content: str) -> Dict[str, Any]:
        """Perform sentiment analysis on document content"""
        if not self.textblob_available:
            return simple_sentiment_analysis(content)

        try:
            from textblob import TextBlob

            if not content.strip():
                return {'sentiment': 'neutral', 'polarity': 0.0, 'subjectivity': 0.0}

            blob = TextBlob(content)

            sentiment_score = blob.sentiment.polarity
            subjectivity_score = blob.sentiment.subjectivity

            if sentiment_score > 0.1:
                sentiment_label = 'positive'
            elif sentiment_score < -0.1:
                sentiment_label = 'negative'
            else:
                sentiment_label = 'neutral'

            return {
                'sentiment': sentiment_label,
                'polarity': round(sentiment_score, 3),
                'subjectivity': round(subjectivity_score, 3),
                'confidence': min(1.0, abs(sentiment_score) * 2)
            }

        except Exception as e:
            logger.error(f"Sentiment analysis error: {e}")
            return simple_sentiment_analysis(content)


# Global ML analytics service instance
ml_analytics_service = MLAnalyticsService()
