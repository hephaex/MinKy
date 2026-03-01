import PropTypes from 'prop-types';

const DocumentInsights = ({ analytics, onLoadSimilar }) => {
  return (
    <div className="document-insights">
      {/* Basic Statistics */}
      {analytics.basic_stats && (
        <div className="insight-section">
          <h4>Document Statistics</h4>
          <div className="stats-grid">
            <div className="stat-item">
              <span className="stat-label">Words</span>
              <span className="stat-value">
                {analytics.basic_stats.word_count.toLocaleString()}
              </span>
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
          <h4>Content Analysis</h4>
          <div className="content-analysis">
            <div className="analysis-item">
              <span className="analysis-label">Complexity Score</span>
              <div className="progress-bar">
                <div
                  className="progress-fill"
                  style={{ width: `${analytics.content_analysis.complexity_score}%` }}
                ></div>
              </div>
              <span className="analysis-value">
                {analytics.content_analysis.complexity_score.toFixed(1)}/100
              </span>
            </div>

            {analytics.content_analysis.keyword_density && (
              <div className="keyword-density">
                <h5>Top Keywords</h5>
                <div className="keywords">
                  {Object.entries(analytics.content_analysis.keyword_density).map(
                    ([keyword, density]) => (
                      <span key={keyword} className="keyword-tag">
                        {keyword} ({density.toFixed(1)}%)
                      </span>
                    )
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Sentiment Analysis */}
      {analytics.sentiment_analysis && (
        <div className="insight-section">
          <h4>Sentiment Analysis</h4>
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
          <h4>Topics</h4>
          <div className="topics">
            {analytics.topic_analysis.topics.map((topic, index) => (
              <div key={index} className="topic-item">
                <span className="topic-term">{topic.term}</span>
                <span className="topic-frequency">x{topic.frequency}</span>
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
          <h4>Similar Documents</h4>
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
          <h4>Recommendations</h4>
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

DocumentInsights.propTypes = {
  analytics: PropTypes.shape({
    basic_stats: PropTypes.shape({
      word_count: PropTypes.number,
      reading_time_minutes: PropTypes.number,
      header_count: PropTypes.number,
      link_count: PropTypes.number,
      image_count: PropTypes.number,
      code_block_count: PropTypes.number,
    }),
    content_analysis: PropTypes.shape({
      complexity_score: PropTypes.number,
      keyword_density: PropTypes.objectOf(PropTypes.number),
    }),
    sentiment_analysis: PropTypes.shape({
      sentiment: PropTypes.string,
      polarity: PropTypes.number,
      confidence: PropTypes.number,
    }),
    topic_analysis: PropTypes.shape({
      topics: PropTypes.arrayOf(
        PropTypes.shape({
          term: PropTypes.string,
          frequency: PropTypes.number,
          relevance: PropTypes.number,
        })
      ),
    }),
    similarity_analysis: PropTypes.shape({
      similar_documents: PropTypes.arrayOf(
        PropTypes.shape({
          id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]).isRequired,
          title: PropTypes.string,
          author: PropTypes.string,
          similarity_score: PropTypes.number,
        })
      ),
    }),
    recommendations: PropTypes.arrayOf(
      PropTypes.shape({
        severity: PropTypes.string,
        message: PropTypes.string,
        suggestion: PropTypes.string,
      })
    ),
  }).isRequired,
  onLoadSimilar: PropTypes.func,
};

DocumentInsights.defaultProps = {
  onLoadSimilar: () => {},
};

export default DocumentInsights;
