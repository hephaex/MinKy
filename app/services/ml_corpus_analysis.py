"""
ML Corpus Analysis
Provides clustering, topic modeling, and trend analysis for document collections
"""

import logging
from typing import Dict, List, Any
from datetime import datetime, timedelta, timezone
from collections import Counter, defaultdict
import numpy as np

logger = logging.getLogger(__name__)


def perform_document_clustering(documents: List, sklearn_available: bool) -> Dict[str, Any]:
    """Perform clustering analysis on documents"""
    if not sklearn_available or len(documents) < 3:
        return {'error': 'Insufficient documents or ML libraries not available'}

    try:
        from sklearn.feature_extraction.text import TfidfVectorizer
        from sklearn.cluster import KMeans

        doc_texts = [doc.markdown_content or '' for doc in documents]
        doc_texts = [text for text in doc_texts if text.strip()]

        if len(doc_texts) < 3:
            return {'error': 'Insufficient documents with content'}

        vectorizer = TfidfVectorizer(max_features=100, stop_words='english')
        tfidf_matrix = vectorizer.fit_transform(doc_texts)

        n_clusters = min(8, max(3, len(doc_texts) // 3))

        kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        cluster_labels = kmeans.fit_predict(tfidf_matrix)

        clusters = defaultdict(list)
        for idx, label in enumerate(cluster_labels):
            if idx < len(documents):
                clusters[int(label)].append({
                    'id': documents[idx].id,
                    'title': documents[idx].title,
                    'author': documents[idx].author
                })

        feature_names = vectorizer.get_feature_names_out()
        cluster_topics = {}

        for i, center in enumerate(kmeans.cluster_centers_):
            top_indices = center.argsort()[-5:][::-1]
            cluster_topics[i] = [feature_names[idx] for idx in top_indices]

        return {
            'n_clusters': n_clusters,
            'clusters': dict(clusters),
            'cluster_topics': cluster_topics,
            'silhouette_score': calculate_silhouette_score(tfidf_matrix, cluster_labels)
        }

    except Exception as e:
        logger.error(f"Clustering error: {e}")
        return {'error': f'Clustering failed: {str(e)}'}


def calculate_silhouette_score(X, labels) -> float:
    """Calculate silhouette score for clustering quality"""
    try:
        from sklearn.metrics import silhouette_score
        if len(set(labels)) > 1:
            return float(silhouette_score(X, labels))
    except Exception:
        pass
    return 0.0


def perform_topic_modeling(documents: List, sklearn_available: bool) -> Dict[str, Any]:
    """Perform topic modeling on document corpus"""
    if not sklearn_available or len(documents) < 5:
        return {'error': 'Insufficient documents or ML libraries not available'}

    try:
        from sklearn.feature_extraction.text import CountVectorizer
        from sklearn.decomposition import LatentDirichletAllocation

        doc_texts = [doc.markdown_content or '' for doc in documents]
        doc_texts = [text for text in doc_texts if len(text.split()) > 20]

        if len(doc_texts) < 5:
            return {'error': 'Insufficient documents with substantial content'}

        vectorizer = CountVectorizer(max_features=100, stop_words='english', min_df=2)
        doc_term_matrix = vectorizer.fit_transform(doc_texts)

        n_topics = min(5, max(2, len(doc_texts) // 3))
        lda = LatentDirichletAllocation(n_components=n_topics, random_state=42)
        lda.fit(doc_term_matrix)

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


def analyze_document_trends(documents: List) -> Dict[str, Any]:
    """Analyze trends in document creation and content"""
    try:
        doc_dates = [doc.created_at.date() for doc in documents]
        date_counts = Counter(doc_dates)

        now = datetime.now(timezone.utc).date()
        last_week = now - timedelta(days=7)
        last_month = now - timedelta(days=30)

        recent_docs = sum(1 for date in doc_dates if date >= last_week)
        monthly_docs = sum(1 for date in doc_dates if date >= last_month)

        authors = [doc.author for doc in documents if doc.author]
        author_counts = Counter(authors)

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
            'peak_creation_day': max(date_counts, key=lambda x: date_counts[x]).isoformat() if date_counts else None
        }

    except Exception as e:
        logger.error(f"Trend analysis error: {e}")
        return {'error': f'Trend analysis failed: {str(e)}'}


def analyze_collaboration_patterns(documents: List, db_session) -> Dict[str, Any]:
    """Analyze collaboration patterns in documents"""
    try:
        from app.models.comment import Comment

        doc_ids = [doc.id for doc in documents]
        comments = Comment.query.filter(Comment.document_id.in_(doc_ids)).all()

        commented_docs = len(set(comment.document_id for comment in comments))
        total_comments = len(comments)

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


def analyze_content_evolution(documents: List, calculate_complexity_fn) -> Dict[str, Any]:
    """Analyze how content has evolved over time"""
    try:
        sorted_docs = sorted(documents, key=lambda x: x.created_at)

        if len(sorted_docs) < 2:
            return {'error': 'Insufficient documents for evolution analysis'}

        early_period = sorted_docs[:len(sorted_docs)//3] if len(sorted_docs) > 3 else sorted_docs[:1]
        late_period = sorted_docs[-len(sorted_docs)//3:] if len(sorted_docs) > 3 else sorted_docs[-1:]

        early_avg_words = np.mean([len((doc.markdown_content or '').split()) for doc in early_period])
        late_avg_words = np.mean([len((doc.markdown_content or '').split()) for doc in late_period])

        early_avg_complexity = np.mean([calculate_complexity_fn(doc.markdown_content or '') for doc in early_period])
        late_avg_complexity = np.mean([calculate_complexity_fn(doc.markdown_content or '') for doc in late_period])

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


def calculate_performance_metrics(documents: List, db_session, get_basic_stats_fn) -> Dict[str, Any]:
    """Calculate performance metrics for documents"""
    try:
        from sqlalchemy import text

        doc_ids = [doc.id for doc in documents]

        comment_query = db_session.execute(
            text("""
                SELECT document_id, COUNT(*) as comment_count
                FROM comments
                WHERE document_id = ANY(:doc_ids)
                GROUP BY document_id
            """),
            {'doc_ids': doc_ids}
        )
        comment_counts: Dict[int, int] = {row[0]: row[1] for row in comment_query.fetchall()}

        total_comments = sum(comment_counts.values())
        avg_comments = total_comments / len(documents) if documents else 0

        quality_scores = []
        for doc in documents:
            stats = get_basic_stats_fn(doc.markdown_content or '')
            comment_count = comment_counts.get(doc.id, 0)

            length_score = min(100, stats['word_count'] / 10)
            structure_score = min(100, stats['header_count'] * 20)
            engagement_score = min(100, comment_count * 25)

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
            ][:5]
        }

    except Exception as e:
        logger.error(f"Performance metrics error: {e}")
        return {'error': f'Performance metrics calculation failed: {str(e)}'}
