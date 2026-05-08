import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import { highlightTextReact, truncateWithHighlight } from '../utils/highlightText';
import { formatDateTime } from '../utils/dateUtils';
import { formatAuthor } from '../utils/documentUtils';
import './DocumentCard.css';

const DocumentCard = ({ document, searchQuery = '', showPreview = false, formatDate, onReprocess }) => {
  const dateFormatter = formatDate || formatDateTime;

  // Show max 3 tags, with overflow indicator
  const maxVisibleTags = 3;
  const visibleTags = document.tags ? document.tags.slice(0, maxVisibleTags) : [];
  const remainingTagsCount = document.tags ? document.tags.length - maxVisibleTags : 0;

  return (
    <div className="document-card">
      <Link to={`/documents/${document.id}`} className="document-link">
        <div className="document-header">
          <svg className="document-icon" viewBox="0 0 16 16" fill="currentColor">
            <path d="M4 1h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V3a2 2 0 0 1 2-2zm0 1a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1H4z" />
            <path d="M4.5 5.5A.5.5 0 0 1 5 5h6a.5.5 0 0 1 0 1H5a.5.5 0 0 1-.5-.5zM5 7a.5.5 0 0 0 0 1h6a.5.5 0 0 0 0-1H5zm0 2a.5.5 0 0 0 0 1h3a.5.5 0 0 0 0-1H5z" />
          </svg>
          <div className="document-content">
            <h3 className="document-title">
              {searchQuery ? highlightTextReact(document.title, searchQuery) : document.title}
            </h3>
          </div>
        </div>

        <div className="document-meta">
          <span>Updated {dateFormatter(document.updated_at)}</span>
          {document.author && (
            <>
              <span className="meta-separator">•</span>
              <span className="document-author">
                {searchQuery
                  ? highlightTextReact(formatAuthor(document.author), searchQuery)
                  : formatAuthor(document.author)}
              </span>
            </>
          )}
          {document.processing_status === 'pending' && (
            <>
              <span className="meta-separator">•</span>
              <span className="processing-badge processing-badge--pending" aria-label="Processing pending">Pending</span>
            </>
          )}
          {document.processing_status === 'failed' && (
            <>
              <span className="meta-separator">•</span>
              {onReprocess ? (
                <button
                  className="processing-badge processing-badge--failed processing-badge--clickable"
                  aria-label="Processing failed. Click to retry."
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    onReprocess(document.id);
                  }}
                >
                  Failed — Retry
                </button>
              ) : (
                <span className="processing-badge processing-badge--failed" aria-label="Processing failed">Failed</span>
              )}
            </>
          )}
        </div>

        {/* Tags display with overflow indicator */}
        {document.tags && document.tags.length > 0 && (
          <div className="document-tags">
            {visibleTags.map((tag, index) => (
              <span key={index} className="document-tag">
                {tag.name || tag}
              </span>
            ))}
            {remainingTagsCount > 0 && (
              <span className="document-tag document-tag-overflow">+{remainingTagsCount}</span>
            )}
          </div>
        )}

        {/* Optional preview text */}
        {showPreview && document.markdown_content && (
          <div className="document-preview">
            {searchQuery ? (
              highlightTextReact(
                truncateWithHighlight(document.markdown_content, searchQuery, 150),
                searchQuery
              )
            ) : (
              <>
                {document.markdown_content.substring(0, 150)}
                {document.markdown_content.length > 150 && '...'}
              </>
            )}
          </div>
        )}
      </Link>
    </div>
  );
};

DocumentCard.propTypes = {
  document: PropTypes.shape({
    id: PropTypes.number.isRequired,
    title: PropTypes.string,
    author: PropTypes.oneOfType([PropTypes.string, PropTypes.array]),
    tags: PropTypes.arrayOf(
      PropTypes.oneOfType([
        PropTypes.string,
        PropTypes.shape({
          name: PropTypes.string,
        }),
      ])
    ),
    updated_at: PropTypes.string,
    markdown_content: PropTypes.string,
    processing_status: PropTypes.oneOf(['pending', 'completed', 'failed']),
  }).isRequired,
  searchQuery: PropTypes.string,
  showPreview: PropTypes.bool,
  formatDate: PropTypes.func,
  onReprocess: PropTypes.func,
};

export default DocumentCard;
