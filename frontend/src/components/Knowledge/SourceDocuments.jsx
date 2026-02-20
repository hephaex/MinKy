import React, { useState } from 'react';
import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import './SourceDocuments.css';

const SourceDocuments = ({ sources = [] }) => {
  const [expanded, setExpanded] = useState(false);

  if (sources.length === 0) return null;

  const visibleSources = expanded ? sources : sources.slice(0, 3);
  const hasMore = sources.length > 3;

  return (
    <aside className="kb-sources" aria-label="참조 문서">
      <div className="kb-sources-header">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
          <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
        </svg>
        <span>참조 문서 ({sources.length})</span>
      </div>

      <ul className="kb-sources-list">
        {visibleSources.map((source, i) => (
          <li key={source.id || i} className="kb-source-item">
            <Link to={`/documents/${source.id}`} className="kb-source-link">
              <span className="kb-source-index" aria-hidden="true">{i + 1}</span>
              <span className="kb-source-title">{source.title || '제목 없음'}</span>
              {source.score != null && (
                <span className="kb-source-relevance" aria-label={`관련도 ${Math.round(source.score * 100)}%`}>
                  {Math.round(source.score * 100)}%
                </span>
              )}
            </Link>
          </li>
        ))}
      </ul>

      {hasMore && (
        <button
          type="button"
          className="kb-sources-toggle"
          onClick={() => setExpanded((prev) => !prev)}
          aria-expanded={expanded}
        >
          {expanded ? '접기' : `${sources.length - 3}개 더 보기`}
        </button>
      )}
    </aside>
  );
};

SourceDocuments.propTypes = {
  sources: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]),
      title: PropTypes.string,
      score: PropTypes.number,
    })
  ),
};

export default SourceDocuments;
