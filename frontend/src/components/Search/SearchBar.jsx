import React, { useState, useRef, useEffect } from 'react';
import PropTypes from 'prop-types';
import './SearchBar.css';

const SearchBar = ({
  onSearch,
  onModeChange,
  mode = 'ask',
  placeholder,
  initialValue = '',
  loading = false,
}) => {
  const [query, setQuery] = useState(initialValue);
  const inputRef = useRef(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const defaultPlaceholder =
    mode === 'ask'
      ? '팀 지식에 대해 질문하세요... (예: 우리 팀이 Redis 캐싱 문제를 어떻게 해결했나요?)'
      : '문서를 의미 기반으로 검색하세요...';

  const handleSubmit = (e) => {
    e.preventDefault();
    const trimmed = query.trim();
    if (trimmed) {
      onSearch(trimmed);
    }
  };

  const handleClear = () => {
    setQuery('');
    inputRef.current?.focus();
  };

  return (
    <form className="kb-search-bar" onSubmit={handleSubmit} role="search">
      {onModeChange && (
        <div className="kb-search-modes" role="group" aria-label="검색 모드">
          <button
            type="button"
            className={`kb-mode-btn ${mode === 'ask' ? 'kb-mode-btn--active' : ''}`}
            onClick={() => onModeChange('ask')}
            aria-pressed={mode === 'ask'}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
              <circle cx="12" cy="12" r="10" />
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            AI에게 질문
          </button>
          <button
            type="button"
            className={`kb-mode-btn ${mode === 'semantic' ? 'kb-mode-btn--active' : ''}`}
            onClick={() => onModeChange('semantic')}
            aria-pressed={mode === 'semantic'}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            유사 문서 검색
          </button>
        </div>
      )}

      <div className="kb-search-input-group">
        <svg
          className="kb-search-icon"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>

        <input
          ref={inputRef}
          type="text"
          className="kb-search-input"
          placeholder={placeholder || defaultPlaceholder}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label={mode === 'ask' ? 'AI 질문 입력' : '검색어 입력'}
          disabled={loading}
        />

        {query && (
          <button
            type="button"
            className="kb-search-clear"
            onClick={handleClear}
            aria-label="검색어 지우기"
            disabled={loading}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" aria-hidden="true">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}

        <button
          type="submit"
          className="kb-search-submit"
          disabled={!query.trim() || loading}
          aria-label={mode === 'ask' ? '질문하기' : '검색하기'}
        >
          {loading ? (
            <span className="kb-search-submit-spinner" aria-hidden="true" />
          ) : (
            mode === 'ask' ? '질문하기' : '검색'
          )}
        </button>
      </div>
    </form>
  );
};

SearchBar.propTypes = {
  onSearch: PropTypes.func.isRequired,
  onModeChange: PropTypes.func,
  mode: PropTypes.oneOf(['ask', 'semantic']),
  placeholder: PropTypes.string,
  initialValue: PropTypes.string,
  loading: PropTypes.bool,
};

export default SearchBar;
