"""
ML Analytics Service for advanced document insights
Provides machine learning-powered analytics and recommendations
"""

import os
import re
import logging
from typing import Dict, List, Optional, Tuple, Any
from datetime import datetime, timedelta
from collections import Counter, defaultdict
import json
import numpy as np
from sqlalchemy import text
from app import db
from app.models.document import Document
from app.models.tag import Tag
from app.models.comment import Comment

logger = logging.getLogger(__name__)

class MLAnalyticsService:
    def __init__(self):
        self.sklearn_available = self._check_sklearn()
        self.nltk_available = self._check_nltk()
        self.textblob_available = self._check_textblob()
        
        # Initialize ML models if available
        if self.sklearn_available:
            self._initialize_sklearn_models()
        
        if self.nltk_available:
            self._initialize_nltk()
    
    def _check_sklearn(self) -> bool:
        """Check if scikit-learn is available"""
        try:
            import sklearn
            return True
        except ImportError:
            logger.warning("scikit-learn not available. ML features will be limited.")
            return False
    
    def _check_nltk(self) -> bool:
        """Check if NLTK is available"""
        try:
            import nltk
            return True
        except ImportError:
            logger.warning("NLTK not available. Natural language processing will be limited.")
            return False
    
    def _check_textblob(self) -> bool:
        """Check if TextBlob is available"""
        try:
            import textblob
            return True
        except ImportError:
            logger.warning("TextBlob not available. Sentiment analysis will be limited.")
            return False
    
    def _initialize_sklearn_models(self):
        """Initialize sklearn models"""
        try:
            from sklearn.feature_extraction.text import TfidfVectorizer
            from sklearn.cluster import KMeans
            from sklearn.decomposition import LatentDirichletAllocation
            from sklearn.metrics.pairwise import cosine_similarity
            
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
            # Try to download required resources
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
        return self.sklearn_available or self.nltk_available or self.textblob_available
    
    def get_document_insights(self, document_id: int) -> Dict[str, Any]:
        """Get comprehensive ML insights for a specific document"""
        try:
            document = Document.query.get(document_id)
            if not document:
                return {'error': 'Document not found'}
            
            insights = {
                'document_id': document_id,
                'title': document.title,
                'basic_stats': self._get_basic_document_stats(document),
                'content_analysis': self._analyze_document_content(document),
                'similarity_analysis': self._get_document_similarities(document),
                'topic_analysis': self._get_document_topics(document),
                'sentiment_analysis': self._get_sentiment_analysis(document),
                'recommendations': self._get_document_recommendations(document),
                'generated_at': datetime.utcnow().isoformat()
            }
            
            return insights
            
        except Exception as e:
            logger.error(f"Error getting document insights: {e}")
            return {'error': f'Failed to generate insights: {str(e)}'}
    
    def get_corpus_insights(self, user_id: Optional[int] = None) -> Dict[str, Any]:
        """Get ML insights for the entire document corpus or user's documents"""
        try:
            # Get documents query
            query = Document.query
            if user_id:
                query = query.filter(Document.user_id == user_id)
            
            documents = query.all()
            
            if not documents:
                return {'error': 'No documents found'}
            
            insights = {
                'corpus_size': len(documents),
                'user_id': user_id,
                'cluster_analysis': self._perform_document_clustering(documents),
                'topic_modeling': self._perform_topic_modeling(documents),
                'trend_analysis': self._analyze_document_trends(documents),
                'collaboration_patterns': self._analyze_collaboration_patterns(documents),
                'content_evolution': self._analyze_content_evolution(documents),
                'performance_metrics': self._calculate_performance_metrics(documents),
                'generated_at': datetime.utcnow().isoformat()
            }
            
            return insights
            
        except Exception as e:
            logger.error(f"Error getting corpus insights: {e}")
            return {'error': f'Failed to generate corpus insights: {str(e)}'}
    
    def _get_basic_document_stats(self, document: Document) -> Dict[str, Any]:
        """Get basic statistical information about a document"""
        content = document.markdown_content or ''
        
        # Text statistics
        word_count = len(content.split())
        char_count = len(content)
        line_count = len(content.split('\n'))
        paragraph_count = len([p for p in content.split('\n\n') if p.strip()])
        
        # Markdown-specific statistics
        header_count = len(re.findall(r'^#+\s', content, re.MULTILINE))
        link_count = len(re.findall(r'\[.*?\]\(.*?\)', content))
        image_count = len(re.findall(r'!\[.*?\]\(.*?\)', content))
        code_block_count = len(re.findall(r'```[\s\S]*?```', content))
        
        # Reading time estimation (average 200 words per minute)
        reading_time_minutes = max(1, word_count // 200)
        
        return {
            'word_count': word_count,
            'char_count': char_count,
            'line_count': line_count,
            'paragraph_count': paragraph_count,
            'header_count': header_count,
            'link_count': link_count,
            'image_count': image_count,
            'code_block_count': code_block_count,
            'reading_time_minutes': reading_time_minutes,
            'avg_words_per_paragraph': word_count / max(1, paragraph_count)
        }
    
    def _analyze_document_content(self, document: Document) -> Dict[str, Any]:
        """Analyze document content using NLP techniques"""
        content = document.markdown_content or ''
        
        analysis = {
            'complexity_score': self._calculate_complexity_score(content),
            'keyword_density': self._calculate_keyword_density(content),
            'structure_analysis': self._analyze_document_structure(content),
            'language_patterns': self._analyze_language_patterns(content)
        }
        
        if self.nltk_available:
            analysis.update(self._nltk_content_analysis(content))
        
        return analysis
    
    def _calculate_complexity_score(self, content: str) -> float:
        """Calculate a simple complexity score based on various factors"""
        if not content:
            return 0.0
        
        words = content.split()
        sentences = re.split(r'[.!?]+', content)
        
        if not words or not sentences:
            return 0.0
        
        # Average words per sentence
        avg_words_per_sentence = len(words) / len(sentences)
        
        # Average characters per word
        avg_chars_per_word = sum(len(word) for word in words) / len(words)
        
        # Unique word ratio
        unique_words_ratio = len(set(words)) / len(words)
        
        # Complex punctuation usage
        complex_punct = len(re.findall(r'[;:(){}[\]"]', content)) / len(content)
        
        # Normalize to 0-100 scale
        complexity = (
            (avg_words_per_sentence / 20.0) * 25 +
            (avg_chars_per_word / 8.0) * 25 +
            unique_words_ratio * 25 +
            (complex_punct * 1000) * 25
        )
        
        return min(100.0, complexity)
    
    def _calculate_keyword_density(self, content: str) -> Dict[str, float]:
        """Calculate keyword density for top terms"""
        words = re.findall(r'\b\w+\b', content.lower())
        
        if not words:
            return {}
        
        # Filter out short words and common stop words
        stop_words = {'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by', 'is', 'are', 'was', 'were', 'be', 'been', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could', 'should'}
        filtered_words = [word for word in words if len(word) > 3 and word not in stop_words]
        
        word_counts = Counter(filtered_words)
        total_words = len(filtered_words)
        
        # Return top 10 keywords with their density
        keyword_density = {}
        for word, count in word_counts.most_common(10):
            keyword_density[word] = (count / total_words) * 100
        
        return keyword_density
    
    def _analyze_document_structure(self, content: str) -> Dict[str, Any]:
        """Analyze the structural elements of the document"""
        headers = re.findall(r'^(#+)\s+(.+)$', content, re.MULTILINE)
        
        structure = {
            'header_hierarchy': {},
            'section_lengths': [],
            'toc_depth': 0,
            'has_introduction': False,
            'has_conclusion': False
        }
        
        # Analyze header hierarchy
        for level_marks, title in headers:
            level = len(level_marks)
            if level not in structure['header_hierarchy']:
                structure['header_hierarchy'][level] = []
            structure['header_hierarchy'][level].append(title.strip())
            structure['toc_depth'] = max(structure['toc_depth'], level)
        
        # Check for introduction and conclusion patterns
        content_lower = content.lower()
        intro_patterns = ['introduction', 'overview', 'getting started', 'background']
        conclusion_patterns = ['conclusion', 'summary', 'final thoughts', 'wrap up']
        
        structure['has_introduction'] = any(pattern in content_lower for pattern in intro_patterns)
        structure['has_conclusion'] = any(pattern in content_lower for pattern in conclusion_patterns)
        
        # Analyze section lengths
        sections = re.split(r'^#+\s+.+$', content, flags=re.MULTILINE)
        structure['section_lengths'] = [len(section.split()) for section in sections if section.strip()]
        
        return structure
    
    def _analyze_language_patterns(self, content: str) -> Dict[str, Any]:
        """Analyze language patterns and writing style"""
        patterns = {
            'question_count': len(re.findall(r'\?', content)),
            'exclamation_count': len(re.findall(r'!', content)),
            'code_ratio': 0.0,
            'list_items': len(re.findall(r'^\s*[-*+]\s', content, re.MULTILINE)),
            'numbered_items': len(re.findall(r'^\s*\d+\.\s', content, re.MULTILINE)),
            'emphasis_usage': {
                'bold': len(re.findall(r'\*\*.*?\*\*', content)),
                'italic': len(re.findall(r'\*.*?\*', content)),
                'code_inline': len(re.findall(r'`.*?`', content))
            }
        }
        
        # Calculate code ratio
        code_blocks = re.findall(r'```[\s\S]*?```', content)
        code_chars = sum(len(block) for block in code_blocks)
        patterns['code_ratio'] = (code_chars / len(content)) * 100 if content else 0
        
        return patterns
    
    def _nltk_content_analysis(self, content: str) -> Dict[str, Any]:
        """Perform NLTK-based content analysis"""
        try:
            import nltk
            from nltk.tokenize import sent_tokenize, word_tokenize
            from nltk.corpus import stopwords
            from nltk.tag import pos_tag
            
            sentences = sent_tokenize(content)
            words = word_tokenize(content.lower())
            
            # POS tagging
            pos_tags = pos_tag(words)
            pos_counts = Counter([tag for word, tag in pos_tags])
            
            # Most common POS tags normalized
            total_tags = len(pos_tags)
            pos_distribution = {tag: (count / total_tags) * 100 
                              for tag, count in pos_counts.most_common(5)}
            
            return {
                'sentence_count': len(sentences),
                'avg_sentence_length': len(words) / len(sentences) if sentences else 0,
                'pos_distribution': pos_distribution,
                'lexical_diversity': len(set(words)) / len(words) if words else 0
            }
            
        except Exception as e:
            logger.error(f"NLTK analysis error: {e}")
            return {}
    
    def _get_document_similarities(self, document: Document) -> Dict[str, Any]:
        """Find similar documents using TF-IDF and cosine similarity"""
        if not self.sklearn_available:
            return {'error': 'ML libraries not available'}
        
        try:
            from sklearn.feature_extraction.text import TfidfVectorizer
            from sklearn.metrics.pairwise import cosine_similarity
            
            # Get all documents for comparison
            all_docs = Document.query.filter(Document.id != document.id).limit(100).all()
            
            if not all_docs:
                return {'similar_documents': []}
            
            # Prepare text corpus
            documents_text = [document.markdown_content or '']
            documents_text.extend([doc.markdown_content or '' for doc in all_docs])
            
            # Calculate TF-IDF
            vectorizer = TfidfVectorizer(max_features=500, stop_words='english')
            tfidf_matrix = vectorizer.fit_transform(documents_text)
            
            # Calculate similarities
            similarities = cosine_similarity(tfidf_matrix[0:1], tfidf_matrix[1:]).flatten()
            
            # Get top 5 similar documents
            similar_indices = similarities.argsort()[-5:][::-1]
            similar_docs = []
            
            for idx in similar_indices:
                if similarities[idx] > 0.1:  # Minimum similarity threshold
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
            from sklearn.decomposition import LatentDirichletAllocation
            
            content = document.markdown_content or ''
            if not content.strip():
                return {'topics': []}
            
            # Simple topic extraction using frequent terms
            vectorizer = CountVectorizer(max_features=20, stop_words='english', ngram_range=(1, 2))
            doc_term_matrix = vectorizer.fit_transform([content])
            
            feature_names = vectorizer.get_feature_names_out()
            term_frequencies = doc_term_matrix.toarray()[0]
            
            # Get top terms as topics
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
    
    def _get_sentiment_analysis(self, document: Document) -> Dict[str, Any]:
        """Perform sentiment analysis on document content"""
        if not self.textblob_available:
            return self._simple_sentiment_analysis(document.markdown_content or '')
        
        try:
            from textblob import TextBlob
            
            content = document.markdown_content or ''
            if not content.strip():
                return {'sentiment': 'neutral', 'polarity': 0.0, 'subjectivity': 0.0}
            
            blob = TextBlob(content)
            
            # TextBlob sentiment analysis
            sentiment_score = blob.sentiment.polarity
            subjectivity_score = blob.sentiment.subjectivity
            
            # Classify sentiment
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
            return self._simple_sentiment_analysis(document.markdown_content or '')
    
    def _simple_sentiment_analysis(self, content: str) -> Dict[str, Any]:
        """Simple rule-based sentiment analysis as fallback"""
        positive_words = ['good', 'great', 'excellent', 'amazing', 'wonderful', 'fantastic', 'love', 'like', 'best', 'awesome']
        negative_words = ['bad', 'terrible', 'awful', 'horrible', 'hate', 'dislike', 'worst', 'poor', 'disappointing']
        
        content_lower = content.lower()
        positive_count = sum(1 for word in positive_words if word in content_lower)
        negative_count = sum(1 for word in negative_words if word in content_lower)
        
        if positive_count > negative_count:
            sentiment = 'positive'
            polarity = min(1.0, (positive_count - negative_count) / 10)
        elif negative_count > positive_count:
            sentiment = 'negative'
            polarity = max(-1.0, (positive_count - negative_count) / 10)
        else:
            sentiment = 'neutral'
            polarity = 0.0
        
        return {
            'sentiment': sentiment,
            'polarity': polarity,
            'subjectivity': 0.5,
            'confidence': min(1.0, abs(polarity))
        }
    
    def _get_document_recommendations(self, document: Document) -> List[Dict[str, Any]]:
        """Generate recommendations for improving the document"""
        recommendations = []
        content = document.markdown_content or ''
        stats = self._get_basic_document_stats(document)
        
        # Content length recommendations
        if stats['word_count'] < 100:
            recommendations.append({
                'type': 'content_length',
                'severity': 'high',
                'message': 'Document is very short. Consider adding more detailed content.',
                'suggestion': 'Aim for at least 200-300 words for better engagement.'
            })
        elif stats['word_count'] > 3000:
            recommendations.append({
                'type': 'content_length',
                'severity': 'medium',
                'message': 'Document is quite long. Consider breaking it into sections.',
                'suggestion': 'Use more headers to organize content into digestible sections.'
            })
        
        # Structure recommendations
        if stats['header_count'] == 0 and stats['word_count'] > 200:
            recommendations.append({
                'type': 'structure',
                'severity': 'medium',
                'message': 'Document lacks headers for organization.',
                'suggestion': 'Add section headers to improve readability and navigation.'
            })
        
        # Reading time recommendations
        if stats['reading_time_minutes'] > 10:
            recommendations.append({
                'type': 'readability',
                'severity': 'low',
                'message': f'Long reading time ({stats["reading_time_minutes"]} minutes).',
                'suggestion': 'Consider adding a summary or table of contents.'
            })
        
        # Link recommendations
        if stats['word_count'] > 500 and stats['link_count'] == 0:
            recommendations.append({
                'type': 'engagement',
                'severity': 'low',
                'message': 'Document has no external links.',
                'suggestion': 'Consider adding relevant links to external resources.'
            })
        
        return recommendations
    
    def _perform_document_clustering(self, documents: List[Document]) -> Dict[str, Any]:
        """Perform clustering analysis on documents"""
        if not self.sklearn_available or len(documents) < 3:
            return {'error': 'Insufficient documents or ML libraries not available'}
        
        try:
            from sklearn.feature_extraction.text import TfidfVectorizer
            from sklearn.cluster import KMeans
            
            # Prepare document texts
            doc_texts = [doc.markdown_content or '' for doc in documents]
            doc_texts = [text for text in doc_texts if text.strip()]
            
            if len(doc_texts) < 3:
                return {'error': 'Insufficient documents with content'}
            
            # TF-IDF vectorization
            vectorizer = TfidfVectorizer(max_features=100, stop_words='english')
            tfidf_matrix = vectorizer.fit_transform(doc_texts)
            
            # Determine optimal number of clusters (3-8)
            n_clusters = min(8, max(3, len(doc_texts) // 3))
            
            # K-means clustering
            kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
            cluster_labels = kmeans.fit_predict(tfidf_matrix)
            
            # Analyze clusters
            clusters = defaultdict(list)
            for idx, label in enumerate(cluster_labels):
                if idx < len(documents):
                    clusters[int(label)].append({
                        'id': documents[idx].id,
                        'title': documents[idx].title,
                        'author': documents[idx].author
                    })
            
            # Get cluster topics (top terms)
            feature_names = vectorizer.get_feature_names_out()
            cluster_topics = {}
            
            for i, center in enumerate(kmeans.cluster_centers_):
                top_indices = center.argsort()[-5:][::-1]
                cluster_topics[i] = [feature_names[idx] for idx in top_indices]
            
            return {
                'n_clusters': n_clusters,
                'clusters': dict(clusters),
                'cluster_topics': cluster_topics,
                'silhouette_score': self._calculate_silhouette_score(tfidf_matrix, cluster_labels)
            }
            
        except Exception as e:
            logger.error(f"Clustering error: {e}")
            return {'error': f'Clustering failed: {str(e)}'}
    
    def _calculate_silhouette_score(self, X, labels):
        """Calculate silhouette score for clustering quality"""
        try:
            from sklearn.metrics import silhouette_score
            if len(set(labels)) > 1:
                return float(silhouette_score(X, labels))
        except:
            pass
        return 0.0
    
    def _perform_topic_modeling(self, documents: List[Document]) -> Dict[str, Any]:
        """Perform topic modeling on document corpus"""
        if not self.sklearn_available or len(documents) < 5:
            return {'error': 'Insufficient documents or ML libraries not available'}
        
        try:
            from sklearn.feature_extraction.text import CountVectorizer
            from sklearn.decomposition import LatentDirichletAllocation
            
            # Prepare texts
            doc_texts = [doc.markdown_content or '' for doc in documents]
            doc_texts = [text for text in doc_texts if len(text.split()) > 20]
            
            if len(doc_texts) < 5:
                return {'error': 'Insufficient documents with substantial content'}
            
            # Vectorization
            vectorizer = CountVectorizer(max_features=100, stop_words='english', min_df=2)
            doc_term_matrix = vectorizer.fit_transform(doc_texts)
            
            # LDA topic modeling
            n_topics = min(5, max(2, len(doc_texts) // 3))
            lda = LatentDirichletAllocation(n_components=n_topics, random_state=42)
            lda.fit(doc_term_matrix)
            
            # Extract topics
            feature_names = vectorizer.get_feature_names_out()
            topics = []
            
            for topic_idx, topic in enumerate(lda.components_):
                top_words_idx = topic.argsort()[-10:][::-1]
                top_words = [feature_names[i] for i in top_words_idx]
                word_weights = [float(topic[i]) for i in top_words_idx]
                
                topics.append({
                    'topic_id': topic_idx,
                    'words': top_words,
                    'weights': word_weights
                })
            
            return {
                'n_topics': n_topics,
                'topics': topics,
                'perplexity': float(lda.perplexity(doc_term_matrix))
            }
            
        except Exception as e:
            logger.error(f"Topic modeling error: {e}")
            return {'error': f'Topic modeling failed: {str(e)}'}
    
    def _analyze_document_trends(self, documents: List[Document]) -> Dict[str, Any]:
        """Analyze trends in document creation and content"""
        try:
            # Time-based analysis
            doc_dates = [doc.created_at.date() for doc in documents]
            date_counts = Counter(doc_dates)
            
            # Calculate trends
            now = datetime.utcnow().date()
            last_week = now - timedelta(days=7)
            last_month = now - timedelta(days=30)
            
            recent_docs = sum(1 for date in doc_dates if date >= last_week)
            monthly_docs = sum(1 for date in doc_dates if date >= last_month)
            
            # Author analysis
            authors = [doc.author for doc in documents if doc.author]
            author_counts = Counter(authors)
            
            # Content trends
            avg_word_count = np.mean([len((doc.markdown_content or '').split()) for doc in documents])
            
            return {
                'total_documents': len(documents),
                'recent_activity': {
                    'last_week': recent_docs,
                    'last_month': monthly_docs
                },
                'top_authors': dict(author_counts.most_common(5)),
                'avg_word_count': int(avg_word_count),
                'creation_pattern': dict(date_counts),
                'peak_creation_day': max(date_counts, key=date_counts.get).isoformat() if date_counts else None
            }
            
        except Exception as e:
            logger.error(f"Trend analysis error: {e}")
            return {'error': f'Trend analysis failed: {str(e)}'}
    
    def _analyze_collaboration_patterns(self, documents: List[Document]) -> Dict[str, Any]:
        """Analyze collaboration patterns in documents"""
        try:
            # Get comments for analysis
            doc_ids = [doc.id for doc in documents]
            comments = Comment.query.filter(Comment.document_id.in_(doc_ids)).all()
            
            # Collaboration metrics
            commented_docs = len(set(comment.document_id for comment in comments))
            total_comments = len(comments)
            
            # Author collaboration
            doc_authors = [doc.author for doc in documents if doc.author]
            comment_authors = [comment.author for comment in comments if comment.author]
            
            unique_doc_authors = set(doc_authors)
            unique_comment_authors = set(comment_authors)
            
            collaboration_score = len(unique_comment_authors & unique_doc_authors) / max(1, len(unique_doc_authors))
            
            return {
                'total_comments': total_comments,
                'commented_documents': commented_docs,
                'collaboration_rate': commented_docs / len(documents) if documents else 0,
                'collaboration_score': collaboration_score,
                'unique_contributors': len(unique_doc_authors | unique_comment_authors),
                'avg_comments_per_doc': total_comments / len(documents) if documents else 0
            }
            
        except Exception as e:
            logger.error(f"Collaboration analysis error: {e}")
            return {'error': f'Collaboration analysis failed: {str(e)}'}
    
    def _analyze_content_evolution(self, documents: List[Document]) -> Dict[str, Any]:
        """Analyze how content has evolved over time"""
        try:
            # Sort documents by creation date
            sorted_docs = sorted(documents, key=lambda x: x.created_at)
            
            if len(sorted_docs) < 2:
                return {'error': 'Insufficient documents for evolution analysis'}
            
            # Analyze evolution metrics
            word_counts = [len((doc.markdown_content or '').split()) for doc in sorted_docs]
            complexity_scores = [self._calculate_complexity_score(doc.markdown_content or '') for doc in sorted_docs]
            
            # Calculate trends
            early_period = sorted_docs[:len(sorted_docs)//3] if len(sorted_docs) > 3 else sorted_docs[:1]
            late_period = sorted_docs[-len(sorted_docs)//3:] if len(sorted_docs) > 3 else sorted_docs[-1:]
            
            early_avg_words = np.mean([len((doc.markdown_content or '').split()) for doc in early_period])
            late_avg_words = np.mean([len((doc.markdown_content or '').split()) for doc in late_period])
            
            early_avg_complexity = np.mean([self._calculate_complexity_score(doc.markdown_content or '') for doc in early_period])
            late_avg_complexity = np.mean([self._calculate_complexity_score(doc.markdown_content or '') for doc in late_period])
            
            return {
                'word_count_trend': 'increasing' if late_avg_words > early_avg_words else 'decreasing',
                'complexity_trend': 'increasing' if late_avg_complexity > early_avg_complexity else 'decreasing',
                'avg_length_change': float(late_avg_words - early_avg_words),
                'avg_complexity_change': float(late_avg_complexity - early_avg_complexity),
                'evolution_score': (late_avg_complexity + late_avg_words/100) / (early_avg_complexity + early_avg_words/100)
            }
            
        except Exception as e:
            logger.error(f"Content evolution analysis error: {e}")
            return {'error': f'Content evolution analysis failed: {str(e)}'}
    
    def _calculate_performance_metrics(self, documents: List[Document]) -> Dict[str, Any]:
        """Calculate performance metrics for documents"""
        try:
            # Get engagement metrics (comments, views, etc.)
            doc_ids = [doc.id for doc in documents]
            
            # Comment engagement
            comment_query = db.session.execute(
                text("""
                    SELECT document_id, COUNT(*) as comment_count
                    FROM comments 
                    WHERE document_id = ANY(:doc_ids)
                    GROUP BY document_id
                """),
                {'doc_ids': doc_ids}
            )
            comment_counts = dict(comment_query.fetchall())
            
            # Calculate metrics
            total_comments = sum(comment_counts.values())
            avg_comments = total_comments / len(documents) if documents else 0
            
            # Document quality score (based on length, structure, engagement)
            quality_scores = []
            for doc in documents:
                stats = self._get_basic_document_stats(doc)
                comment_count = comment_counts.get(doc.id, 0)
                
                # Quality factors
                length_score = min(100, stats['word_count'] / 10)  # 1000 words = 100 points
                structure_score = min(100, stats['header_count'] * 20)  # Headers improve score
                engagement_score = min(100, comment_count * 25)  # Comments indicate engagement
                
                quality_score = (length_score + structure_score + engagement_score) / 3
                quality_scores.append(quality_score)
            
            return {
                'avg_quality_score': float(np.mean(quality_scores)) if quality_scores else 0,
                'avg_comments_per_doc': avg_comments,
                'top_performing_docs': [
                    {
                        'id': doc.id,
                        'title': doc.title,
                        'quality_score': quality_scores[i],
                        'comment_count': comment_counts.get(doc.id, 0)
                    }
                    for i, doc in enumerate(documents)
                    if i < len(quality_scores)
                ][:5]  # Top 5
            }
            
        except Exception as e:
            logger.error(f"Performance metrics error: {e}")
            return {'error': f'Performance metrics calculation failed: {str(e)}'}

# Global ML analytics service instance
ml_analytics_service = MLAnalyticsService()