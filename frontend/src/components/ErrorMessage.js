import React from 'react';
import PropTypes from 'prop-types';
import './ErrorMessage.css';

/**
 * Reusable error message component
 */
const ErrorMessage = ({
  error,
  title = '오류가 발생했습니다',
  onRetry = null,
  showDetails = false
}) => {
  const errorMessage = typeof error === 'string'
    ? error
    : error?.message || '알 수 없는 오류가 발생했습니다.';

  return (
    <div className="error-message">
      <div className="error-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
      </div>
      <h3 className="error-title">{title}</h3>
      <p className="error-text">{errorMessage}</p>
      {showDetails && error?.stack && (
        <details className="error-details">
          <summary>상세 정보</summary>
          <pre>{error.stack}</pre>
        </details>
      )}
      {onRetry && (
        <button className="error-retry-btn" onClick={onRetry}>
          다시 시도
        </button>
      )}
    </div>
  );
};

ErrorMessage.propTypes = {
  error: PropTypes.oneOfType([
    PropTypes.string,
    PropTypes.shape({
      message: PropTypes.string,
      stack: PropTypes.string
    })
  ]).isRequired,
  title: PropTypes.string,
  onRetry: PropTypes.func,
  showDetails: PropTypes.bool
};

export default ErrorMessage;
