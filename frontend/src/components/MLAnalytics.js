import React, { useState, useEffect } from 'react';
import { authService } from '../services/api';
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
      console.error('Error loading ML status:', err);
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
      console.error('ML Analytics error:', err);
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
      console.error('Corpus Analytics error:', err);
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
      console.error('Error loading similar documents:', err);
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
        <h3>🤖 ML Analytics</h3>
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

const DocumentInsights = ({ analytics, onLoadSimilar }) => {
  return (
    <div className="document-insights">
      {/* Basic Statistics */}
      {analytics.basic_stats && (
        <div className="insight-section">
          <h4>📊 Document Statistics</h4>
          <div className="stats-grid">
            <div className="stat-item">
              <span className="stat-label">Words</span>
              <span className="stat-value">{analytics.basic_stats.word_count.toLocaleString()}</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Reading Time</span>
              <span className="stat-value">{analytics.basic_stats.reading_time_minutes} min</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Headers</span>
              <span className="stat-value">{analytics.basic_stats.header_count}</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Links</span>
              <span className="stat-value">{analytics.basic_stats.link_count}</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Images</span>
              <span className="stat-value">{analytics.basic_stats.image_count}</span>
            </div>
            <div className="stat-item">
              <span className="stat-label">Code Blocks</span>
              <span className="stat-value">{analytics.basic_stats.code_block_count}</span>
            </div>
          </div>
        </div>
      )}

      {/* Content Analysis */}
      {analytics.content_analysis && (
        <div className="insight-section">
          <h4>🔍 Content Analysis</h4>
          <div className="content-analysis">
            <div className="analysis-item">
              <span className="analysis-label">Complexity Score</span>
              <div className="progress-bar">
                <div 
                  className="progress-fill" 
                  style={{ width: `${analytics.content_analysis.complexity_score}%` }}
                ></div>
              </div>
              <span className="analysis-value">{analytics.content_analysis.complexity_score.toFixed(1)}/100</span>
            </div>
            
            {analytics.content_analysis.keyword_density && (
              <div className="keyword-density">
                <h5>Top Keywords</h5>
                <div className="keywords">
                  {Object.entries(analytics.content_analysis.keyword_density).map(([keyword, density]) => (
                    <span key={keyword} className="keyword-tag">
                      {keyword} ({density.toFixed(1)}%)
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Sentiment Analysis */}
      {analytics.sentiment_analysis && (
        <div className="insight-section">
          <h4>😊 Sentiment Analysis</h4>
          <div className="sentiment-analysis">
            <div className="sentiment-main">
              <span className={`sentiment-label ${analytics.sentiment_analysis.sentiment}`}>
                {analytics.sentiment_analysis.sentiment.toUpperCase()}
              </span>
              <span className="sentiment-score">
                Polarity: {analytics.sentiment_analysis.polarity.toFixed(2)}
              </span>
              <span className="sentiment-confidence">
                Confidence: {(analytics.sentiment_analysis.confidence * 100).toFixed(0)}%
              </span>
            </div>
          </div>
        </div>
      )}

      {/* Topic Analysis */}
      {analytics.topic_analysis && analytics.topic_analysis.topics && (
        <div className="insight-section">
          <h4>🏷️ Topics</h4>
          <div className="topics">
            {analytics.topic_analysis.topics.map((topic, index) => (
              <div key={index} className="topic-item">
                <span className="topic-term">{topic.term}</span>
                <span className="topic-frequency">×{topic.frequency}</span>
                <div className="topic-relevance">
                  <div 
                    className="relevance-bar" 
                    style={{ width: `${topic.relevance * 100}%` }}
                  ></div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Similar Documents */}
      {analytics.similarity_analysis && analytics.similarity_analysis.similar_documents && (
        <div className="insight-section">
          <h4>🔗 Similar Documents</h4>
          {analytics.similarity_analysis.similar_documents.length > 0 ? (
            <div className="similar-documents">
              {analytics.similarity_analysis.similar_documents.map((doc) => (
                <div key={doc.id} className="similar-doc">
                  <div className="doc-info">
                    <a href={`/documents/${doc.id}`} className="doc-title">
                      {doc.title}
                    </a>
                    <span className="doc-author">by {doc.author}</span>
                  </div>
                  <div className="similarity-score">
                    {(doc.similarity_score * 100).toFixed(0)}% similar
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="no-similar">
              <p>No similar documents found.</p>
              <button onClick={onLoadSimilar} className="btn btn-secondary btn-sm">
                Find Similar Documents
              </button>
            </div>
          )}
        </div>
      )}

      {/* Recommendations */}
      {analytics.recommendations && analytics.recommendations.length > 0 && (
        <div className="insight-section">
          <h4>💡 Recommendations</h4>
          <div className="recommendations">
            {analytics.recommendations.map((rec, index) => (
              <div key={index} className={`recommendation ${rec.severity}`}>
                <div className="rec-message">{rec.message}</div>
                <div className="rec-suggestion">{rec.suggestion}</div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const CorpusInsights = ({ analytics }) => {
  return (
    <div className="corpus-insights">
      {/* Corpus Overview */}
      <div className="insight-section">
        <h4>📈 Corpus Overview</h4>
        <div className="corpus-stats">
          <div className="stat-item large">
            <span className="stat-label">Total Documents</span>
            <span className="stat-value">{analytics.corpus_size}</span>
          </div>
        </div>
      </div>

      {/* Cluster Analysis */}
      {analytics.cluster_analysis && analytics.cluster_analysis.clusters && (
        <div className="insight-section">
          <h4>🎯 Document Clusters</h4>
          <div className="cluster-info">
            <p>Found {analytics.cluster_analysis.n_clusters} clusters with {analytics.cluster_analysis.silhouette_score?.toFixed(3)} quality score</p>
          </div>
          <div className="clusters">
            {Object.entries(analytics.cluster_analysis.clusters).map(([clusterId, docs]) => (
              <div key={clusterId} className="cluster">
                <div className="cluster-header">
                  <h5>Cluster {clusterId}</h5>
                  <span className="cluster-size">{docs.length} documents</span>
                </div>
                <div className="cluster-topics">
                  {analytics.cluster_analysis.cluster_topics[clusterId]?.map((topic, i) => (
                    <span key={i} className="cluster-topic">{topic}</span>
                  ))}
                </div>
                <div className="cluster-docs">
                  {docs.slice(0, 3).map((doc) => (
                    <div key={doc.id} className="cluster-doc">
                      <a href={`/documents/${doc.id}`}>{doc.title}</a>
                    </div>
                  ))}
                  {docs.length > 3 && <div className="more-docs">+{docs.length - 3} more</div>}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Topic Modeling */}
      {analytics.topic_modeling && analytics.topic_modeling.topics && (
        <div className="insight-section">
          <h4>🏷️ Topic Modeling</h4>
          <div className="topic-modeling">
            {analytics.topic_modeling.topics.map((topic) => (
              <div key={topic.topic_id} className="topic-model">
                <h5>Topic {topic.topic_id}</h5>
                <div className="topic-words">
                  {topic.words.slice(0, 5).map((word, i) => (
                    <span key={i} className="topic-word" style={{
                      opacity: topic.weights[i] / Math.max(...topic.weights)
                    }}>
                      {word}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Trend Analysis */}
      {analytics.trend_analysis && (
        <div className="insight-section">
          <h4>📊 Trends</h4>
          <div className="trend-analysis">
            <div className="trend-item">
              <span className="trend-label">Recent Activity (7 days)</span>
              <span className="trend-value">{analytics.trend_analysis.recent_activity?.last_week || 0} documents</span>
            </div>
            <div className="trend-item">
              <span className="trend-label">Monthly Activity</span>
              <span className="trend-value">{analytics.trend_analysis.recent_activity?.last_month || 0} documents</span>
            </div>
            <div className="trend-item">
              <span className="trend-label">Average Word Count</span>
              <span className="trend-value">{analytics.trend_analysis.avg_word_count} words</span>
            </div>
          </div>

          {analytics.trend_analysis.top_authors && (
            <div className="top-authors">
              <h5>Top Authors</h5>
              {Object.entries(analytics.trend_analysis.top_authors).map(([author, count]) => (
                <div key={author} className="author-stat">
                  <span className="author-name">{author}</span>
                  <span className="author-count">{count} documents</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Collaboration Patterns */}
      {analytics.collaboration_patterns && (
        <div className="insight-section">
          <h4>🤝 Collaboration</h4>
          <div className="collaboration-stats">
            <div className="collab-item">
              <span className="collab-label">Collaboration Rate</span>
              <span className="collab-value">
                {(analytics.collaboration_patterns.collaboration_rate * 100).toFixed(1)}%
              </span>
            </div>
            <div className="collab-item">
              <span className="collab-label">Total Comments</span>
              <span className="collab-value">{analytics.collaboration_patterns.total_comments}</span>
            </div>
            <div className="collab-item">
              <span className="collab-label">Unique Contributors</span>
              <span className="collab-value">{analytics.collaboration_patterns.unique_contributors}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default MLAnalytics;