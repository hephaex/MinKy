import React, { useEffect, useCallback } from 'react';
import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import LoadingSpinner from '../LoadingSpinner';
import ErrorMessage from '../ErrorMessage';
import { searchService } from '../../services/api';
import useAsync from '../../hooks/useAsync';
import './RelatedDocsList.css';

const RelatedDocsList = ({ documentId, limit = 5 }) => {
  const fetchRelated = useCallback(
    () => searchService.getSimilar(documentId, limit),
    [documentId, limit]
  );

  const { execute, loading, error, data } = useAsync(fetchRelated);

  useEffect(() => {
    if (documentId) {
      execute();
    }
  }, [documentId, execute]);

  const docs = data?.documents || data?.data || data || [];

  if (loading) {
    return (
      <aside className="kb-related" aria-label="관련 문서">
        <h3 className="kb-related-title">관련 문서</h3>
        <LoadingSpinner size="small" message="관련 문서 불러오는 중..." />
      </aside>
    );
  }

  if (error) {
    return (
      <aside className="kb-related" aria-label="관련 문서">
        <h3 className="kb-related-title">관련 문서</h3>
        <ErrorMessage
          error={error}
          title="관련 문서를 불러올 수 없습니다"
          onRetry={execute}
        />
      </aside>
    );
  }

  if (!docs || docs.length === 0) return null;

  return (
    <aside className="kb-related" aria-label="관련 문서">
      <h3 className="kb-related-title">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
          <path d="M9 3H5a2 2 0 0 0-2 2v4m6-6h10a2 2 0 0 1 2 2v4M9 3v18m0 0h10a2 2 0 0 0 2-2V9M9 21H5a2 2 0 0 1-2-2V9m0 0h18" />
        </svg>
        관련 문서
      </h3>

      <ul className="kb-related-list">
        {docs.map((doc, i) => (
          <li key={doc.id || i} className="kb-related-item">
            <Link to={`/documents/${doc.id}`} className="kb-related-link">
              <div className="kb-related-meta">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                  <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                  <polyline points="14 2 14 8 20 8" />
                </svg>
                <span className="kb-related-doc-title">
                  {doc.title || '제목 없음'}
                </span>
              </div>

              {doc.summary && (
                <p className="kb-related-summary">{doc.summary}</p>
              )}

              <div className="kb-related-footer">
                {doc.score != null && (
                  <span className="kb-related-score">
                    {Math.round(doc.score * 100)}% 유사
                  </span>
                )}
                {doc.tags && doc.tags.length > 0 && (
                  <div className="kb-related-tags" aria-label="태그">
                    {doc.tags.slice(0, 2).map((tag, ti) => (
                      <span key={ti} className="kb-related-tag">
                        {tag.name || tag}
                      </span>
                    ))}
                  </div>
                )}
              </div>
            </Link>
          </li>
        ))}
      </ul>
    </aside>
  );
};

RelatedDocsList.propTypes = {
  documentId: PropTypes.oneOfType([PropTypes.string, PropTypes.number]).isRequired,
  limit: PropTypes.number,
};

export default RelatedDocsList;
