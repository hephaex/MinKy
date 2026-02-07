import React from 'react';

const DuplicateDetection = ({ duplicates, onDetect }) => {
  return (
    <div className="duplicate-detection">
      <div className="section-header">
        <h4>Duplicate Detection</h4>
        <button onClick={onDetect} className="btn btn-primary">
          Detect Duplicates
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

export default DuplicateDetection;
