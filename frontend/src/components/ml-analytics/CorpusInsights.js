import React from 'react';

const CorpusInsights = ({ analytics }) => {
  return (
    <div className="corpus-insights">
      {/* Corpus Overview */}
      <div className="insight-section">
        <h4>Corpus Overview</h4>
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
          <h4>Document Clusters</h4>
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
          <h4>Topic Modeling</h4>
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
          <h4>Trends</h4>
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
          <h4>Collaboration</h4>
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

export default CorpusInsights;
