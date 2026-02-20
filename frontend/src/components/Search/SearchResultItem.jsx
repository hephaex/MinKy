import React from 'react';
import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import './SearchResultItem.css';

const SearchResultItem = ({ result, query = '' }) => {
  const {
    id,
    title,
    excerpt,
    score,
    tags = [],
    updated_at,
    source,
  } = result;

  const highlightText = (text, searchQuery) => {
    if (!searchQuery || !text) return text;
    const escaped = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const parts = text.split(new RegExp(`(${escaped})`, 'gi'));
    return parts.map((part, i) =>
      part.toLowerCase() === searchQuery.toLowerCase()
        ? <mark key={i} className="kb-result-highlight">{part}</mark>
        : part
    );
  };

  const formatScore = (s) => {
    if (s == null) return null;
    return `${Math.round(s * 100)}% 관련도`;
  };

  const formatDate = (dateStr) => {
    if (!dateStr) return null;
    return new Date(dateStr).toLocaleDateString('ko-KR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  };

  return (
    <article className="kb-result-item">
      <Link to={`/documents/${id}`} className="kb-result-link">
        <div className="kb-result-header">
          <svg className="kb-result-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
          </svg>
          <h3 className="kb-result-title">
            {highlightText(title || '제목 없음', query)}
          </h3>
          {score != null && (
            <span className="kb-result-score" aria-label={`관련도 ${formatScore(score)}`}>
              {formatScore(score)}
            </span>
          )}
        </div>

        {excerpt && (
          <p className="kb-result-excerpt">
            {highlightText(excerpt, query)}
          </p>
        )}

        <div className="kb-result-footer">
          {tags.length > 0 && (
            <div className="kb-result-tags" aria-label="태그">
              {tags.slice(0, 3).map((tag, i) => (
                <span key={i} className="kb-result-tag">
                  {tag.name || tag}
                </span>
              ))}
              {tags.length > 3 && (
                <span className="kb-result-tag kb-result-tag--overflow">
                  +{tags.length - 3}
                </span>
              )}
            </div>
          )}
          <div className="kb-result-meta">
            {source && <span className="kb-result-source">{source}</span>}
            {updated_at && (
              <span className="kb-result-date">
                {formatDate(updated_at)}
              </span>
            )}
          </div>
        </div>
      </Link>
    </article>
  );
};

SearchResultItem.propTypes = {
  result: PropTypes.shape({
    id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]).isRequired,
    title: PropTypes.string,
    excerpt: PropTypes.string,
    score: PropTypes.number,
    tags: PropTypes.array,
    updated_at: PropTypes.string,
    source: PropTypes.string,
  }).isRequired,
  query: PropTypes.string,
};

export default SearchResultItem;
