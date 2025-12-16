"""
Document Clustering and Similarity Detection Service
Advanced document organization using machine learning clustering algorithms
"""

import logging
from typing import Dict, List, Optional, Tuple, Any
from datetime import datetime, timezone
import numpy as np
from collections import defaultdict, Counter
from app.models.document import Document

logger = logging.getLogger(__name__)

class DocumentClusteringService:
    def __init__(self):
        self.sklearn_available = self._check_sklearn()
        self.clustering_cache = {}
        self.similarity_cache = {}
        
        if self.sklearn_available:
            self._initialize_models()
    
    def _check_sklearn(self) -> bool:
        """Check if scikit-learn is available"""
        try:
            import sklearn
            return True
        except ImportError:
            logger.warning("scikit-learn not available. Clustering features will be limited.")
            return False
    
    def _initialize_models(self):
        """Initialize ML models for clustering"""
        try:
            from sklearn.feature_extraction.text import TfidfVectorizer
            from sklearn.cluster import KMeans, AgglomerativeClustering, DBSCAN
            from sklearn.decomposition import PCA
            from sklearn.manifold import TSNE
            from sklearn.metrics.pairwise import cosine_similarity
            from sklearn.preprocessing import StandardScaler
            
            self.vectorizer = TfidfVectorizer(
                max_features=500,
                stop_words='english',
                ngram_range=(1, 2),
                min_df=2,
                max_df=0.8
            )
            
            self.kmeans = None
            self.hierarchical = None
            self.dbscan = None
            self.pca = PCA(n_components=50)
            self.scaler = StandardScaler()
            
        except ImportError as e:
            logger.error(f"Error initializing clustering models: {e}")
            self.sklearn_available = False
    
    def is_available(self) -> bool:
        """Check if clustering service is available"""
        return bool(self.sklearn_available)
    
    def cluster_documents(self, documents: List[Document], method: str = 'kmeans', 
                         n_clusters: Optional[int] = None) -> Dict[str, Any]:
        """
        Cluster documents using specified method
        
        Args:
            documents: List of documents to cluster
            method: Clustering method ('kmeans', 'hierarchical', 'dbscan', 'auto')
            n_clusters: Number of clusters (for methods that require it)
            
        Returns:
            Dictionary with clustering results
        """
        if not self.sklearn_available:
            return {'error': 'Clustering requires scikit-learn'}
        
        if len(documents) < 3:
            return {'error': 'Minimum 3 documents required for clustering'}
        
        try:
            # Prepare document texts
            doc_texts = []
            doc_metadata = []
            
            for doc in documents:
                content = doc.markdown_content or ''
                if content.strip():
                    doc_texts.append(content)
                    doc_metadata.append({
                        'id': doc.id,
                        'title': doc.title,
                        'author': doc.author,
                        'created_at': doc.created_at.isoformat(),
                        'word_count': len(content.split()),
                        'tags': [tag.name for tag in list(doc.tags)] if doc.tags else []
                    })
            
            if len(doc_texts) < 3:
                return {'error': 'Insufficient documents with content'}
            
            # Vectorize documents
            tfidf_matrix = self.vectorizer.fit_transform(doc_texts)
            
            # Determine optimal number of clusters if not specified
            if not n_clusters:
                n_clusters = self._determine_optimal_clusters(tfidf_matrix)
            
            # Perform clustering based on method
            if method == 'auto':
                method = self._select_best_method(tfidf_matrix, n_clusters)
            
            cluster_labels, cluster_info = self._perform_clustering(
                tfidf_matrix, method, n_clusters
            )
            
            # Organize results
            clusters = self._organize_clustering_results(
                cluster_labels, doc_metadata, tfidf_matrix
            )
            
            # Generate cluster insights
            cluster_insights = self._generate_cluster_insights(
                clusters, tfidf_matrix, cluster_labels
            )
            
            # Calculate cluster quality metrics
            quality_metrics = self._calculate_cluster_quality(
                tfidf_matrix, cluster_labels
            )
            
            results = {
                'method': method,
                'n_clusters': n_clusters,
                'documents_processed': len(doc_texts),
                'clusters': clusters,
                'cluster_insights': cluster_insights,
                'quality_metrics': quality_metrics,
                'cluster_info': cluster_info,
                'generated_at': datetime.now(timezone.utc).isoformat()
            }
            
            # Cache results
            cache_key = f"{method}_{n_clusters}_{len(documents)}"
            self.clustering_cache[cache_key] = results
            
            return results
            
        except Exception as e:
            logger.error(f"Clustering error: {e}")
            return {'error': f'Clustering failed: {str(e)}'}
    
    def find_similar_documents(self, target_document: Document,
                             candidate_documents: Optional[List[Document]] = None,
                             similarity_threshold: float = 0.1,
                             max_results: int = 10) -> Dict[str, Any]:
        """
        Find documents similar to the target document
        
        Args:
            target_document: Document to find similarities for
            candidate_documents: Documents to compare against (None for all)
            similarity_threshold: Minimum similarity score
            max_results: Maximum number of results to return
            
        Returns:
            Dictionary with similarity results
        """
        if not self.sklearn_available:
            return {'error': 'Similarity detection requires scikit-learn'}
        
        try:
            from sklearn.metrics.pairwise import cosine_similarity
            
            target_content = target_document.markdown_content or ''
            if not target_content.strip():
                return {'error': 'Target document has no content'}
            
            # Get candidate documents
            if candidate_documents is None:
                candidate_documents = Document.query.filter(
                    Document.id != target_document.id
                ).limit(1000).all()
            
            if not candidate_documents:
                return {'similar_documents': []}
            
            # Prepare texts
            all_texts = [target_content]
            candidate_metadata = []
            
            for doc in candidate_documents:
                content = doc.markdown_content or ''
                if content.strip():
                    all_texts.append(content)
                    candidate_metadata.append({
                        'id': doc.id,
                        'title': doc.title,
                        'author': doc.author,
                        'created_at': doc.created_at.isoformat(),
                        'word_count': len(content.split()),
                        'tags': [tag.name for tag in list(doc.tags)] if doc.tags else []
                    })
            
            if len(all_texts) < 2:
                return {'similar_documents': []}
            
            # Vectorize texts
            tfidf_matrix = self.vectorizer.fit_transform(all_texts)
            
            # Calculate similarities
            target_vector = tfidf_matrix[0:1]
            candidate_vectors = tfidf_matrix[1:]
            
            similarities = cosine_similarity(target_vector, candidate_vectors).flatten()
            
            # Find similar documents
            similar_docs = []
            for idx, similarity_score in enumerate(similarities):
                if similarity_score >= similarity_threshold:
                    doc_info = candidate_metadata[idx].copy()
                    doc_info['similarity_score'] = float(similarity_score)
                    doc_info['similarity_reasons'] = self._explain_similarity(
                        target_content, all_texts[idx + 1], similarity_score
                    )
                    similar_docs.append(doc_info)
            
            # Sort by similarity score
            similar_docs.sort(key=lambda x: x['similarity_score'], reverse=True)
            
            # Limit results
            similar_docs = similar_docs[:max_results]
            
            # Calculate similarity statistics
            similarity_stats = self._calculate_similarity_stats(similarities)
            
            results = {
                'target_document': {
                    'id': target_document.id,
                    'title': target_document.title,
                    'author': target_document.author
                },
                'similar_documents': similar_docs,
                'similarity_stats': similarity_stats,
                'candidates_analyzed': len(candidate_documents),
                'similarity_threshold': similarity_threshold,
                'generated_at': datetime.now(timezone.utc).isoformat()
            }
            
            return results
            
        except Exception as e:
            logger.error(f"Similarity detection error: {e}")
            return {'error': f'Similarity detection failed: {str(e)}'}
    
    def detect_document_duplicates(self, documents: Optional[List[Document]] = None,
                                 similarity_threshold: float = 0.8) -> Dict[str, Any]:
        """
        Detect potential duplicate documents
        
        Args:
            documents: Documents to check (None for all)
            similarity_threshold: Threshold for considering documents duplicates
            
        Returns:
            Dictionary with duplicate detection results
        """
        if not self.sklearn_available:
            return {'error': 'Duplicate detection requires scikit-learn'}
        
        try:
            from sklearn.metrics.pairwise import cosine_similarity
            
            # Get documents to check
            if documents is None:
                documents = Document.query.limit(1000).all()
            
            if len(documents) < 2:
                return {'duplicates': []}
            
            # Prepare texts and metadata
            doc_texts = []
            doc_metadata = []
            
            for doc in documents:
                content = doc.markdown_content or ''
                if content.strip():
                    doc_texts.append(content)
                    doc_metadata.append({
                        'id': doc.id,
                        'title': doc.title,
                        'author': doc.author,
                        'created_at': doc.created_at.isoformat(),
                        'word_count': len(content.split())
                    })
            
            if len(doc_texts) < 2:
                return {'duplicates': []}
            
            # Vectorize documents
            tfidf_matrix = self.vectorizer.fit_transform(doc_texts)
            
            # Calculate pairwise similarities
            similarity_matrix = cosine_similarity(tfidf_matrix)
            
            # Find duplicates
            duplicates = []
            processed_pairs = set()
            
            for i in range(len(similarity_matrix)):
                for j in range(i + 1, len(similarity_matrix)):
                    similarity_score = similarity_matrix[i][j]
                    
                    if similarity_score >= similarity_threshold:
                        pair_key = tuple(sorted([i, j]))
                        if pair_key not in processed_pairs:
                            processed_pairs.add(pair_key)
                            
                            duplicates.append({
                                'document1': doc_metadata[i],
                                'document2': doc_metadata[j],
                                'similarity_score': float(similarity_score),
                                'duplicate_type': self._classify_duplicate_type(
                                    doc_metadata[i], doc_metadata[j], similarity_score
                                )
                            })
            
            # Sort by similarity score
            duplicates.sort(key=lambda x: x['similarity_score'], reverse=True)  # type: ignore[arg-type, return-value]
            
            # Generate duplicate statistics
            duplicate_stats = {
                'total_duplicates': len(duplicates),
                'documents_analyzed': len(doc_texts),
                'avg_similarity': np.mean([d['similarity_score'] for d in duplicates]) if duplicates else 0,
                'similarity_threshold': similarity_threshold
            }
            
            return {
                'duplicates': duplicates,
                'duplicate_stats': duplicate_stats,
                'generated_at': datetime.now(timezone.utc).isoformat()
            }
            
        except Exception as e:
            logger.error(f"Duplicate detection error: {e}")
            return {'error': f'Duplicate detection failed: {str(e)}'}
    
    def _determine_optimal_clusters(self, tfidf_matrix) -> int:
        """Determine optimal number of clusters using elbow method"""
        try:
            from sklearn.cluster import KMeans
            from sklearn.metrics import silhouette_score
            
            n_docs = tfidf_matrix.shape[0]
            max_clusters = min(8, n_docs // 2)
            min_clusters = 2
            
            if max_clusters <= min_clusters:
                return min_clusters
            
            scores = []
            for k in range(min_clusters, max_clusters + 1):
                kmeans = KMeans(n_clusters=k, random_state=42, n_init=10)
                labels = kmeans.fit_predict(tfidf_matrix)
                
                if len(set(labels)) > 1:
                    score = silhouette_score(tfidf_matrix, labels)
                    scores.append((k, score))
            
            if scores:
                # Return k with highest silhouette score
                optimal_k = max(scores, key=lambda x: x[1])[0]
                return optimal_k
            
            return min_clusters
            
        except Exception as e:
            logger.error(f"Error determining optimal clusters: {e}")
            return int(min(3, n_docs // 2))
    
    def _select_best_method(self, tfidf_matrix, n_clusters: int) -> str:
        """Select the best clustering method based on data characteristics"""
        n_docs = tfidf_matrix.shape[0]
        
        # For small datasets, use hierarchical clustering
        if n_docs < 20:
            return 'hierarchical'
        
        # For medium datasets, use k-means
        if n_docs < 100:
            return 'kmeans'
        
        # For large datasets, try DBSCAN first, fall back to k-means
        return 'dbscan'
    
    def _perform_clustering(self, tfidf_matrix, method: str, n_clusters: int) -> Tuple[np.ndarray, Dict]:
        """Perform clustering using specified method"""
        cluster_info = {'method': method, 'parameters': {}}
        
        try:
            if method == 'kmeans':
                from sklearn.cluster import KMeans
                
                kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
                cluster_labels = kmeans.fit_predict(tfidf_matrix)
                cluster_info['parameters'] = {'n_clusters': n_clusters}
                
            elif method == 'hierarchical':
                from sklearn.cluster import AgglomerativeClustering
                
                hierarchical = AgglomerativeClustering(n_clusters=n_clusters)
                cluster_labels = hierarchical.fit_predict(tfidf_matrix.toarray())
                cluster_info['parameters'] = {'n_clusters': n_clusters}
                
            elif method == 'dbscan':
                from sklearn.cluster import DBSCAN
                
                # Determine eps parameter automatically
                eps = self._determine_dbscan_eps(tfidf_matrix)
                dbscan = DBSCAN(eps=eps, min_samples=2, metric='cosine')
                cluster_labels = dbscan.fit_predict(tfidf_matrix)
                
                # If DBSCAN produces too few clusters, fall back to k-means
                n_clusters_found = len(set(cluster_labels)) - (1 if -1 in cluster_labels else 0)
                if n_clusters_found < 2:
                    return self._perform_clustering(tfidf_matrix, 'kmeans', n_clusters)
                
                cluster_info['parameters'] = {'eps': eps, 'min_samples': 2}
                
            else:
                raise ValueError(f"Unknown clustering method: {method}")
            
            return cluster_labels, cluster_info
            
        except Exception as e:
            logger.error(f"Clustering method {method} failed: {e}")
            # Fall back to k-means
            from sklearn.cluster import KMeans
            kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
            cluster_labels = kmeans.fit_predict(tfidf_matrix)
            cluster_info = {'method': 'kmeans_fallback', 'parameters': {'n_clusters': n_clusters}}
            return cluster_labels, cluster_info
    
    def _determine_dbscan_eps(self, tfidf_matrix) -> float:
        """Determine epsilon parameter for DBSCAN"""
        try:
            from sklearn.neighbors import NearestNeighbors
            
            # Calculate k-distance graph
            k = 2
            nbrs = NearestNeighbors(n_neighbors=k, metric='cosine').fit(tfidf_matrix)
            distances, indices = nbrs.kneighbors(tfidf_matrix)
            
            # Sort distances
            k_distances = np.sort(distances[:, k-1])
            
            # Find elbow point (simplified)
            n_points = len(k_distances)
            elbow_idx = n_points // 2  # Simple heuristic
            
            return float(k_distances[elbow_idx])
            
        except Exception as e:
            logger.error(f"Error determining DBSCAN eps: {e}")
            return 0.5  # Default value
    
    def _organize_clustering_results(self, cluster_labels: np.ndarray, 
                                   doc_metadata: List[Dict], 
                                   tfidf_matrix) -> Dict[int, List[Dict]]:
        """Organize clustering results by cluster"""
        clusters = defaultdict(list)
        
        for idx, label in enumerate(cluster_labels):
            if idx < len(doc_metadata):
                doc_info = doc_metadata[idx].copy()
                doc_info['cluster_label'] = int(label)
                clusters[int(label)].append(doc_info)
        
        return dict(clusters)
    
    def _generate_cluster_insights(self, clusters: Dict, tfidf_matrix, cluster_labels: np.ndarray) -> Dict:
        """Generate insights about each cluster"""
        insights = {}
        feature_names = self.vectorizer.get_feature_names_out()
        
        for cluster_id, docs in clusters.items():
            # Get documents in this cluster
            cluster_indices = [i for i, label in enumerate(cluster_labels) if label == cluster_id]
            
            if not cluster_indices:
                continue
            
            # Calculate cluster centroid
            cluster_vectors = tfidf_matrix[cluster_indices]
            centroid = np.mean(cluster_vectors, axis=0).A1
            
            # Get top terms for this cluster
            top_indices = centroid.argsort()[-10:][::-1]
            top_terms = [feature_names[i] for i in top_indices if centroid[i] > 0]
            
            # Cluster statistics
            word_counts = [doc['word_count'] for doc in docs]
            authors = [doc['author'] for doc in docs if doc['author']]
            tags = []
            for doc in docs:
                tags.extend(doc.get('tags', []))
            
            insights[cluster_id] = {
                'size': len(docs),
                'top_terms': top_terms[:5],
                'avg_word_count': int(np.mean(word_counts)) if word_counts else 0,
                'authors': list(set(authors)),
                'common_tags': [tag for tag, count in Counter(tags).most_common(5)],
                'creation_dates': [doc['created_at'] for doc in docs],
                'cluster_coherence': float(np.std(centroid))
            }
        
        return insights
    
    def _calculate_cluster_quality(self, tfidf_matrix, cluster_labels: np.ndarray) -> Dict:
        """Calculate clustering quality metrics"""
        try:
            from sklearn.metrics import silhouette_score, calinski_harabasz_score
            
            n_clusters = len(set(cluster_labels)) - (1 if -1 in cluster_labels else 0)

            metrics: Dict[str, Any] = {
                'n_clusters': n_clusters,
                'n_noise_points': int(np.sum(cluster_labels == -1)),
            }
            
            if n_clusters > 1:
                # Silhouette score
                metrics['silhouette_score'] = float(silhouette_score(tfidf_matrix, cluster_labels))
                
                # Calinski-Harabasz score
                metrics['calinski_harabasz_score'] = float(calinski_harabasz_score(tfidf_matrix.toarray(), cluster_labels))
            
            # Cluster size distribution
            cluster_sizes = Counter(cluster_labels)
            metrics['cluster_sizes'] = dict(cluster_sizes)
            metrics['avg_cluster_size'] = float(np.mean(list(cluster_sizes.values())))
            metrics['cluster_size_std'] = float(np.std(list(cluster_sizes.values())))
            
            return metrics
            
        except Exception as e:
            logger.error(f"Error calculating cluster quality: {e}")
            return {'error': f'Quality calculation failed: {str(e)}'}
    
    def _explain_similarity(self, text1: str, text2: str, similarity_score: float) -> Dict:
        """Explain why two documents are similar"""
        try:
            # Simple explanation based on common terms
            words1 = set(text1.lower().split())
            words2 = set(text2.lower().split())
            
            common_words = words1 & words2
            unique_words1 = words1 - words2
            unique_words2 = words2 - words1
            
            return {
                'similarity_score': float(similarity_score),
                'common_words_count': len(common_words),
                'common_words_sample': list(common_words)[:10],
                'jaccard_similarity': len(common_words) / len(words1 | words2) if words1 | words2 else 0,
                'length_ratio': len(text2) / len(text1) if text1 else 0
            }
            
        except Exception as e:
            logger.error(f"Error explaining similarity: {e}")
            return {'error': 'Could not explain similarity'}
    
    def _calculate_similarity_stats(self, similarities: np.ndarray) -> Dict:
        """Calculate statistics about similarity scores"""
        return {
            'mean_similarity': float(np.mean(similarities)),
            'median_similarity': float(np.median(similarities)),
            'std_similarity': float(np.std(similarities)),
            'max_similarity': float(np.max(similarities)),
            'min_similarity': float(np.min(similarities)),
            'high_similarity_count': int(np.sum(similarities > 0.5)),
            'medium_similarity_count': int(np.sum((similarities > 0.2) & (similarities <= 0.5))),
            'low_similarity_count': int(np.sum(similarities <= 0.2))
        }
    
    def _classify_duplicate_type(self, doc1_metadata: Dict, doc2_metadata: Dict, 
                               similarity_score: float) -> str:
        """Classify the type of duplicate detected"""
        # Exact duplicate
        if similarity_score > 0.95:
            return 'exact_duplicate'
        
        # Near duplicate
        if similarity_score > 0.8:
            # Check if same author
            if doc1_metadata['author'] == doc2_metadata['author']:
                return 'author_near_duplicate'
            else:
                return 'cross_author_near_duplicate'
        
        # Similar content
        if similarity_score > 0.6:
            return 'similar_content'
        
        return 'potential_duplicate'

# Global document clustering service instance
document_clustering_service = DocumentClusteringService()