import React from 'react';
import PropTypes from 'prop-types';
import './Pagination.css';

const Pagination = ({ pagination, currentPage, onPageChange }) => {
  const { pages, has_prev, has_next, total } = pagination;

  if (pages <= 1) return null;

  const getPageNumbers = () => {
    const delta = 2;
    const range = [];
    const rangeWithDots = [];

    for (
      let i = Math.max(2, currentPage - delta);
      i <= Math.min(pages - 1, currentPage + delta);
      i++
    ) {
      range.push(i);
    }

    if (currentPage - delta > 2) {
      rangeWithDots.push(1, '...');
    } else {
      rangeWithDots.push(1);
    }

    rangeWithDots.push(...range);

    if (currentPage + delta < pages - 1) {
      rangeWithDots.push('...', pages);
    } else {
      rangeWithDots.push(pages);
    }

    return rangeWithDots;
  };

  return (
    <div className="pagination">
      <div className="pagination-info">
        Showing page {currentPage} of {pages} ({total} total documents)
      </div>
      
      <div className="pagination-controls">
        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={!has_prev}
        >
          Previous
        </button>

        {getPageNumbers().map((page, index) => (
          <button
            key={index}
            className={`pagination-btn ${
              page === currentPage ? 'active' : ''
            } ${page === '...' ? 'dots' : ''}`}
            onClick={() => typeof page === 'number' && onPageChange(page)}
            disabled={page === '...'}
          >
            {page}
          </button>
        ))}

        <button
          className="pagination-btn"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={!has_next}
        >
          Next
        </button>
      </div>
    </div>
  );
};

Pagination.propTypes = {
  pagination: PropTypes.shape({
    pages: PropTypes.number.isRequired,
    has_prev: PropTypes.bool.isRequired,
    has_next: PropTypes.bool.isRequired,
    total: PropTypes.number.isRequired
  }).isRequired,
  currentPage: PropTypes.number.isRequired,
  onPageChange: PropTypes.func.isRequired
};

export default Pagination;