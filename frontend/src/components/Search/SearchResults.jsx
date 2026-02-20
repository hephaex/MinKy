import React from 'react';
import PropTypes from 'prop-types';
import SearchResultItem from './SearchResultItem';
import LoadingSpinner from '../LoadingSpinner';
import ErrorMessage from '../ErrorMessage';
import './SearchResults.css';

const SearchResults = ({
  results = [],
  query = '',
  loading = false,
  error = null,
  totalCount = null,
  onRetry,
}) => {
  if (loading) {
    return (
      <div className="kb-results-state">
        <LoadingSpinner size="medium" message="검색 중..." />
      </div>
    );
  }

  if (error) {
    return (
      <div className="kb-results-state">
        <ErrorMessage
          error={error}
          title="검색 중 오류가 발생했습니다"
          onRetry={onRetry}
        />
      </div>
    );
  }

  if (!query) {
    return null;
  }

  if (results.length === 0) {
    return (
      <div className="kb-results-empty" role="status" aria-live="polite">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#ccc" strokeWidth="1.5" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
          <line x1="8" y1="11" x2="14" y2="11" />
        </svg>
        <p className="kb-results-empty-title">검색 결과가 없습니다</p>
        <p className="kb-results-empty-desc">
          <strong>"{query}"</strong>에 대한 문서를 찾을 수 없습니다.
          다른 키워드로 검색해보세요.
        </p>
      </div>
    );
  }

  return (
    <section className="kb-results" aria-label="검색 결과">
      <div className="kb-results-header">
        <p className="kb-results-count" role="status" aria-live="polite">
          {totalCount != null ? (
            <><strong>{totalCount.toLocaleString()}</strong>개 결과</>
          ) : (
            <><strong>{results.length}</strong>개 결과</>
          )}
          {query && <> — <span className="kb-results-query">"{query}"</span></>}
        </p>
      </div>

      <ul className="kb-results-list">
        {results.map((result) => (
          <li key={result.id}>
            <SearchResultItem result={result} query={query} />
          </li>
        ))}
      </ul>
    </section>
  );
};

SearchResults.propTypes = {
  results: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.oneOfType([PropTypes.string, PropTypes.number]).isRequired,
    })
  ),
  query: PropTypes.string,
  loading: PropTypes.bool,
  error: PropTypes.oneOfType([PropTypes.string, PropTypes.object]),
  totalCount: PropTypes.number,
  onRetry: PropTypes.func,
};

export default SearchResults;
