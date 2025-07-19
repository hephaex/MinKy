import React, { useState, useEffect } from 'react';
import { authService } from '../services/api';
import './DocumentClustering.css';

const DocumentClustering = ({ documentId, showFullInterface = false }) => {
  const [clusteringResults, setClusteringResults] = useState(null);
  const [similarDocs, setSimilarDocs] = useState(null);
  const [duplicates, setDuplicates] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState('similar');
  const [clusteringStatus, setClusteringStatus] = useState(null);
  const [clusteringConfig, setClusteringConfig] = useState({
    method: 'auto',
    n_clusters: null,
    scope: 'user',
    max_documents: 100
  });

  useEffect(() => {
    loadClusteringStatus();
    if (documentId && activeTab === 'similar') {
      loadSimilarDocuments();
    }
  }, [documentId, activeTab]);

  const loadClusteringStatus = async () => {
    try {
      const response = await fetch('/api/clustering/status');
      const data = await response.json();
      setClusteringStatus(data.status);
    } catch (err) {
      console.error('Error loading clustering status:', err);
    }
  };

  const loadSimilarDocuments = async () => {
    if (!documentId) return;
    
    setLoading(true);
    setError(null);
    
    try {
      const token = authService.getToken();
      const headers = {};
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch(`/api/clustering/similar/${documentId}?threshold=0.1&max_results=10`, {
        headers
      });
      
      const data = await response.json();
      
      if (data.success) {
        setSimilarDocs(data.similarity);
      } else {
        setError(data.error || 'Failed to find similar documents');
      }
    } catch (err) {
      setError('Error finding similar documents');
      console.error('Similarity error:', err);
    } finally {
      setLoading(false);
    }
  };

  const performDocumentClustering = async () => {
    setLoading(true);
    setError(null);
    
    try {
      const token = authService.getToken();
      const headers = {
        'Content-Type': 'application/json'
      };
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch('/api/clustering/cluster', {
        method: 'POST',
        headers,
        body: JSON.stringify(clusteringConfig)
      });
      
      const data = await response.json();
      
      if (data.success) {
        setClusteringResults(data.clustering);
      } else {
        setError(data.error || 'Failed to cluster documents');
      }
    } catch (err) {
      setError('Error clustering documents');
      console.error('Clustering error:', err);
    } finally {
      setLoading(false);
    }
  };

  const detectDuplicates = async () => {
    setLoading(true);
    setError(null);
    
    try {
      const token = authService.getToken();
      const headers = {
        'Content-Type': 'application/json'
      };
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch('/api/clustering/duplicates', {
        method: 'POST',
        headers,
        body: JSON.stringify({
          threshold: 0.8,
          scope: 'user',
          max_documents: 500
        })
      });
      
      const data = await response.json();
      
      if (data.success) {
        setDuplicates(data.duplicates);
      } else {
        setError(data.error || 'Failed to detect duplicates');
      }
    } catch (err) {
      setError('Error detecting duplicates');
      console.error('Duplicate detection error:', err);
    } finally {
      setLoading(false);
    }
  };

  if (!clusteringStatus) {
    return <div className="document-clustering loading">Loading clustering service...</div>;
  }

  if (!clusteringStatus.available) {
    return (
      <div className="document-clustering unavailable">
        <h3>Document Clustering Unavailable</h3>
        <p>Document clustering requires machine learning libraries to be installed.</p>
        <div className="clustering-status">
          <div className="status-item">
            <span className="label">Scikit-learn:</span>
            <span className={`status ${clusteringStatus.sklearn_available ? 'available' : 'unavailable'}`}>
              {clusteringStatus.sklearn_available ? 'Available' : 'Not Available'}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="document-clustering">
      <div className="clustering-header">
        <h3>🎯 Document Clustering</h3>
        {showFullInterface && (
          <div className="clustering-tabs">
            <button 
              className={`tab ${activeTab === 'similar' ? 'active' : ''}`}
              onClick={() => setActiveTab('similar')}
            >
              Similar Documents
            </button>
            <button 
              className={`tab ${activeTab === 'clusters' ? 'active' : ''}`}
              onClick={() => setActiveTab('clusters')}
            >
              Document Clusters
            </button>
            <button 
              className={`tab ${activeTab === 'duplicates' ? 'active' : ''}`}
              onClick={() => setActiveTab('duplicates')}
            >
              Duplicate Detection
            </button>
          </div>
        )}
      </div>

      {loading && <div className="loading">Processing documents...</div>}
      {error && <div className="error">{error}</div>}

      <div className="clustering-content">
        {activeTab === 'similar' && (
          <SimilarDocuments 
            documentId={documentId}
            similarDocs={similarDocs}
            onRefresh={loadSimilarDocuments}
          />
        )}
        
        {activeTab === 'clusters' && showFullInterface && (
          <DocumentClusters 
            clusteringResults={clusteringResults}
            clusteringConfig={clusteringConfig}
            setClusteringConfig={setClusteringConfig}
            onCluster={performDocumentClustering}
          />
        )}
        
        {activeTab === 'duplicates' && showFullInterface && (
          <DuplicateDetection 
            duplicates={duplicates}
            onDetect={detectDuplicates}
          />
        )}
      </div>
    </div>
  );
};

const SimilarDocuments = ({ documentId, similarDocs, onRefresh }) => {
  if (!documentId) {
    return (
      <div className="similar-documents">
        <div className="no-document">
          <p>Select a document to find similar content.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="similar-documents">
      <div className="section-header">
        <h4>📄 Similar Documents</h4>
        <button onClick={onRefresh} className="btn btn-secondary btn-sm">
          🔄 Refresh
        </button>
      </div>

      {similarDocs && (
        <div className="similarity-results">
          {similarDocs.similar_documents && similarDocs.similar_documents.length > 0 ? (
            <div className="similar-docs-list">
              {similarDocs.similar_documents.map((doc) => (
                <div key={doc.id} className="similar-doc-card">
                  <div className="doc-header">
                    <a href={`/documents/${doc.id}`} className="doc-title">
                      {doc.title}
                    </a>
                    <div className="similarity-badge">
                      {(doc.similarity_score * 100).toFixed(0)}% similar
                    </div>
                  </div>
                  <div className="doc-meta">
                    <span className="doc-author">by {doc.author}</span>
                    <span className="doc-date">
                      {new Date(doc.created_at).toLocaleDateString()}
                    </span>
                  </div>
                  {doc.similarity_reasons && (
                    <div className="similarity-reasons">
                      <div className="reason-item">
                        <span className="reason-label">Common words:</span>
                        <span className="reason-value">{doc.similarity_reasons.common_words_count}</span>
                      </div>
                      <div className="reason-item">
                        <span className="reason-label">Jaccard similarity:</span>
                        <span className="reason-value">
                          {(doc.similarity_reasons.jaccard_similarity * 100).toFixed(1)}%
                        </span>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="no-similar">
              <p>No similar documents found.</p>
              <p className="hint">This document appears to be unique in your collection.</p>
            </div>
          )}

          {similarDocs.similarity_stats && (
            <div className="similarity-stats">
              <h5>Similarity Statistics</h5>
              <div className="stats-grid">
                <div className="stat-item">
                  <span className="stat-label">Documents Analyzed</span>
                  <span className="stat-value">{similarDocs.candidates_analyzed}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Average Similarity</span>
                  <span className="stat-value">
                    {(similarDocs.similarity_stats.mean_similarity * 100).toFixed(1)}%
                  </span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">High Similarity Count</span>
                  <span className="stat-value">{similarDocs.similarity_stats.high_similarity_count}</span>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

const DocumentClusters = ({ clusteringResults, clusteringConfig, setClusteringConfig, onCluster }) => {
  const handleConfigChange = (key, value) => {
    setClusteringConfig(prev => ({
      ...prev,
      [key]: value
    }));
  };

  return (
    <div className="document-clusters">
      <div className="section-header">
        <h4>🎯 Document Clustering</h4>
      </div>

      <div className="clustering-config">
        <div className="config-row">
          <div className="config-group">
            <label>Clustering Method:</label>
            <select 
              value={clusteringConfig.method}
              onChange={(e) => handleConfigChange('method', e.target.value)}
              className="form-control"
            >
              <option value="auto">Auto-select</option>
              <option value="kmeans">K-Means</option>
              <option value="hierarchical">Hierarchical</option>
              <option value="dbscan">DBSCAN</option>
            </select>
          </div>

          <div className="config-group">
            <label>Number of Clusters:</label>
            <input
              type="number"
              value={clusteringConfig.n_clusters || ''}
              onChange={(e) => handleConfigChange('n_clusters', e.target.value ? parseInt(e.target.value) : null)}
              placeholder="Auto-detect"
              min="2"
              max="20"
              className="form-control"
            />
          </div>

          <div className="config-group">
            <label>Scope:</label>
            <select 
              value={clusteringConfig.scope}
              onChange={(e) => handleConfigChange('scope', e.target.value)}
              className="form-control"
            >
              <option value="user">My Documents</option>
              <option value="public">Public Documents</option>
            </select>
          </div>

          <div className="config-group">
            <label>Max Documents:</label>
            <input
              type="number"
              value={clusteringConfig.max_documents}
              onChange={(e) => handleConfigChange('max_documents', parseInt(e.target.value))}
              min="10"
              max="1000"
              className="form-control"
            />
          </div>
        </div>

        <button onClick={onCluster} className="btn btn-primary">
          🎯 Cluster Documents
        </button>
      </div>

      {clusteringResults && (
        <div className="clustering-results">
          <div className="cluster-overview">
            <h5>Clustering Results</h5>
            <div className="overview-stats">
              <div className="overview-item">
                <span className="overview-label">Method Used:</span>
                <span className="overview-value">{clusteringResults.method}</span>
              </div>
              <div className="overview-item">
                <span className="overview-label">Clusters Found:</span>
                <span className="overview-value">{clusteringResults.n_clusters}</span>
              </div>
              <div className="overview-item">
                <span className="overview-label">Documents Processed:</span>
                <span className="overview-value">{clusteringResults.documents_processed}</span>
              </div>
              {clusteringResults.quality_metrics && clusteringResults.quality_metrics.silhouette_score && (
                <div className="overview-item">
                  <span className="overview-label">Quality Score:</span>
                  <span className="overview-value">
                    {clusteringResults.quality_metrics.silhouette_score.toFixed(3)}
                  </span>
                </div>
              )}
            </div>
          </div>

          <div className="clusters-display">
            {Object.entries(clusteringResults.clusters).map(([clusterId, docs]) => (
              <div key={clusterId} className="cluster-card">
                <div className="cluster-header">
                  <h6>Cluster {clusterId}</h6>
                  <span className="cluster-size">{docs.length} documents</span>
                </div>

                {clusteringResults.cluster_insights && clusteringResults.cluster_insights[clusterId] && (
                  <div className="cluster-insights">
                    <div className="cluster-topics">
                      <span className="topics-label">Key terms:</span>
                      <div className="topics-list">
                        {clusteringResults.cluster_insights[clusterId].top_terms.map((term, i) => (
                          <span key={i} className="topic-tag">{term}</span>
                        ))}
                      </div>
                    </div>
                    
                    {clusteringResults.cluster_insights[clusterId].common_tags.length > 0 && (
                      <div className="cluster-tags">
                        <span className="tags-label">Common tags:</span>
                        <div className="tags-list">
                          {clusteringResults.cluster_insights[clusterId].common_tags.map((tag, i) => (
                            <span key={i} className="tag-badge">{tag}</span>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                <div className="cluster-documents">
                  {docs.slice(0, 5).map((doc) => (
                    <div key={doc.id} className="cluster-doc">
                      <a href={`/documents/${doc.id}`} className="doc-link">
                        {doc.title}
                      </a>
                      <span className="doc-author">by {doc.author}</span>
                    </div>
                  ))}
                  {docs.length > 5 && (
                    <div className="more-docs">
                      +{docs.length - 5} more documents
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const DuplicateDetection = ({ duplicates, onDetect }) => {
  return (
    <div className="duplicate-detection">
      <div className="section-header">
        <h4>🔍 Duplicate Detection</h4>
        <button onClick={onDetect} className="btn btn-primary">
          🔍 Detect Duplicates
        </button>
      </div>

      {duplicates && (
        <div className="duplicate-results">
          {duplicates.duplicates && duplicates.duplicates.length > 0 ? (
            <div className="duplicates-list">
              <div className="duplicates-stats">
                <div className="stat-item">
                  <span className="stat-label">Duplicates Found:</span>
                  <span className="stat-value">{duplicates.duplicate_stats.total_duplicates}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Documents Analyzed:</span>
                  <span className="stat-value">{duplicates.duplicate_stats.documents_analyzed}</span>
                </div>
                <div className="stat-item">
                  <span className="stat-label">Average Similarity:</span>
                  <span className="stat-value">
                    {(duplicates.duplicate_stats.avg_similarity * 100).toFixed(1)}%
                  </span>
                </div>
              </div>

              {duplicates.duplicates.map((duplicate, index) => (
                <div key={index} className="duplicate-pair">
                  <div className="duplicate-header">
                    <span className={`duplicate-type ${duplicate.duplicate_type.replace(/_/g, '-')}`}>
                      {duplicate.duplicate_type.replace(/_/g, ' ').toUpperCase()}
                    </span>
                    <span className="similarity-score">
                      {(duplicate.similarity_score * 100).toFixed(1)}% similar
                    </span>
                  </div>
                  
                  <div className="duplicate-documents">
                    <div className="duplicate-doc">
                      <a href={`/documents/${duplicate.document1.id}`} className="doc-title">
                        {duplicate.document1.title}
                      </a>
                      <div className="doc-meta">
                        <span className="doc-author">by {duplicate.document1.author}</span>
                        <span className="doc-date">
                          {new Date(duplicate.document1.created_at).toLocaleDateString()}
                        </span>
                        <span className="doc-words">{duplicate.document1.word_count} words</span>
                      </div>
                    </div>
                    
                    <div className="vs-separator">↔</div>
                    
                    <div className="duplicate-doc">
                      <a href={`/documents/${duplicate.document2.id}`} className="doc-title">
                        {duplicate.document2.title}
                      </a>
                      <div className="doc-meta">
                        <span className="doc-author">by {duplicate.document2.author}</span>
                        <span className="doc-date">
                          {new Date(duplicate.document2.created_at).toLocaleDateString()}
                        </span>
                        <span className="doc-words">{duplicate.document2.word_count} words</span>
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="no-duplicates">
              <p>No duplicate documents found!</p>
              <p className="hint">Your document collection appears to be unique.</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default DocumentClustering;