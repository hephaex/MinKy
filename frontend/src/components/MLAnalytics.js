import React, { useState, useEffect } from 'react';
import { authService } from '../services/api';
import { DocumentInsights, CorpusInsights } from './ml-analytics';
import './MLAnalytics.css';

const MLAnalytics = ({ documentId, showCorpusAnalysis = false }) => {
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [activeTab, setActiveTab] = useState('insights');
  const [mlStatus, setMlStatus] = useState(null);

  useEffect(() => {
    loadMLStatus();
    if (documentId && activeTab === 'insights') {
      loadDocumentInsights();
    } else if (showCorpusAnalysis && activeTab === 'corpus') {
      loadCorpusInsights();
    }
  }, [documentId, activeTab, showCorpusAnalysis]);

  const loadMLStatus = async () => {
    try {
      const response = await fetch('/api/ml-analytics/status');
      const data = await response.json();
      setMlStatus(data.status);
    } catch (err) {
      setError('Failed to load ML status');
    }
  };

  const loadDocumentInsights = async () => {
    if (!documentId) return;

    setLoading(true);
    setError(null);

    try {
      const token = authService.getToken();
      const headers = {};
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch(`/api/ml-analytics/document/${documentId}/insights`, {
        headers
      });

      const data = await response.json();

      if (data.success) {
        setAnalytics(data.insights);
      } else {
        setError(data.error || 'Failed to load insights');
      }
    } catch (err) {
      setError('Error loading document insights');
    } finally {
      setLoading(false);
    }
  };

  const loadCorpusInsights = async () => {
    setLoading(true);
    setError(null);

    try {
      const token = authService.getToken();
      const headers = {};
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch('/api/ml-analytics/corpus/insights?scope=user', {
        headers
      });

      const data = await response.json();

      if (data.success) {
        setAnalytics(data.insights);
      } else {
        setError(data.error || 'Failed to load corpus insights');
      }
    } catch (err) {
      setError('Error loading corpus insights');
    } finally {
      setLoading(false);
    }
  };

  const getSimilarDocuments = async () => {
    if (!documentId) return;

    setLoading(true);
    try {
      const token = authService.getToken();
      const headers = {};
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch(`/api/ml-analytics/document/${documentId}/similar`, {
        headers
      });

      const data = await response.json();

      if (data.success) {
        setAnalytics(prev => ({
          ...prev,
          similar_documents: data.similar_documents
        }));
      }
    } catch (err) {
      setError('Error loading similar documents');
    } finally {
      setLoading(false);
    }
  };

  if (!mlStatus) {
    return <div className="ml-analytics loading">Loading ML Analytics...</div>;
  }

  if (!mlStatus.available) {
    return (
      <div className="ml-analytics unavailable">
        <h3>ML Analytics Unavailable</h3>
        <p>Machine learning analytics requires additional libraries to be installed.</p>
        <div className="ml-status">
          <div className="status-item">
            <span className="label">Scikit-learn:</span>
            <span className={`status ${mlStatus.sklearn_available ? 'available' : 'unavailable'}`}>
              {mlStatus.sklearn_available ? 'Available' : 'Not Available'}
            </span>
          </div>
          <div className="status-item">
            <span className="label">NLTK:</span>
            <span className={`status ${mlStatus.nltk_available ? 'available' : 'unavailable'}`}>
              {mlStatus.nltk_available ? 'Available' : 'Not Available'}
            </span>
          </div>
          <div className="status-item">
            <span className="label">TextBlob:</span>
            <span className={`status ${mlStatus.textblob_available ? 'available' : 'unavailable'}`}>
              {mlStatus.textblob_available ? 'Available' : 'Not Available'}
            </span>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="ml-analytics">
      <div className="analytics-header">
        <h3>ML Analytics</h3>
        {(documentId && showCorpusAnalysis) && (
          <div className="analytics-tabs">
            <button
              className={`tab ${activeTab === 'insights' ? 'active' : ''}`}
              onClick={() => setActiveTab('insights')}
            >
              Document Insights
            </button>
            <button
              className={`tab ${activeTab === 'corpus' ? 'active' : ''}`}
              onClick={() => setActiveTab('corpus')}
            >
              Corpus Analysis
            </button>
          </div>
        )}
      </div>

      {loading && <div className="loading">Analyzing content...</div>}
      {error && <div className="error">{error}</div>}

      {analytics && (
        <div className="analytics-content">
          {activeTab === 'insights' && documentId && (
            <DocumentInsights analytics={analytics} onLoadSimilar={getSimilarDocuments} />
          )}
          {activeTab === 'corpus' && (
            <CorpusInsights analytics={analytics} />
          )}
        </div>
      )}
    </div>
  );
};

export default MLAnalytics;
