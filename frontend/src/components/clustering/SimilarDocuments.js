import React from 'react';

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
        <h4>Similar Documents</h4>
        <button onClick={onRefresh} className="btn btn-secondary btn-sm">
          Refresh
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

export default SimilarDocuments;
