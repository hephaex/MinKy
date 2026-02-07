import React, { useState, useEffect } from 'react';
import { authService } from '../services/api';
import { SimilarDocuments, DocumentClusters, DuplicateDetection } from './clustering';
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
      setError('Failed to load clustering status');
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
        <h3>Document Clustering</h3>
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

export default DocumentClustering;
