import { useEffect, useState } from 'react';
import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import { documentService } from '../services/api';
import { logError } from '../utils/logger';
import { formatDate } from '../utils/dateUtils';
import './DocumentToc.css';

export const TOC_MODES = {
  TOC: 'toc',
  RECENT: 'recent',
};

/**
 * Side panel shown next to an open document. Defaults to a table-of-contents
 * built from the document's own headings (`headings`, computed by the caller
 * via `utils/tocUtils.extractHeadings` from the exact markdown string being
 * rendered) — clicking an entry smooth-scrolls to the matching heading.
 * Toggling to "최근 보기" swaps in a small recently-viewed-documents list
 * instead (fetched on demand, only while that mode is active).
 */
const DocumentToc = ({ headings = [], currentDocumentId, mode, onModeChange }) => {
  const [recentDocuments, setRecentDocuments] = useState([]);
  const [recentLoading, setRecentLoading] = useState(false);
  const [recentError, setRecentError] = useState(null);

  useEffect(() => {
    if (mode !== TOC_MODES.RECENT) return undefined;
    let cancelled = false;
    const fetchRecent = async () => {
      try {
        setRecentLoading(true);
        setRecentError(null);
        const response = await documentService.getDocuments(1, 5, '');
        if (!cancelled) setRecentDocuments(response.documents || []);
      } catch (err) {
        if (!cancelled) {
          setRecentError('최근 문서를 불러올 수 없습니다');
          logError('DocumentToc.fetchRecent', err);
        }
      } finally {
        if (!cancelled) setRecentLoading(false);
      }
    };
    fetchRecent();
    return () => {
      cancelled = true;
    };
  }, [mode]);

  const handleTocClick = (event, slug) => {
    event.preventDefault();
    const target = window.document.getElementById(slug);
    if (!target) return;
    target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    // Move focus for keyboard/screen-reader users without re-triggering the
    // scroll a second time (focus() alone can also scroll).
    target.setAttribute('tabIndex', '-1');
    target.focus({ preventScroll: true });
  };

  return (
    <nav className="document-toc" aria-label="문서 네비게이션">
      <div className="document-toc-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={mode === TOC_MODES.TOC}
          className={`document-toc-tab${mode === TOC_MODES.TOC ? ' active' : ''}`}
          onClick={() => onModeChange(TOC_MODES.TOC)}
        >
          목차
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={mode === TOC_MODES.RECENT}
          className={`document-toc-tab${mode === TOC_MODES.RECENT ? ' active' : ''}`}
          onClick={() => onModeChange(TOC_MODES.RECENT)}
        >
          최근 보기
        </button>
      </div>

      {mode === TOC_MODES.TOC ? (
        headings.length > 0 ? (
          <ul className="document-toc-list">
            {headings.map((h) => (
              <li key={h.slug} className={`document-toc-item level-${h.level}`}>
                <a href={`#${h.slug}`} onClick={(e) => handleTocClick(e, h.slug)}>
                  {h.text}
                </a>
              </li>
            ))}
          </ul>
        ) : (
          <div className="document-toc-empty">이 문서에는 제목이 없습니다</div>
        )
      ) : recentLoading ? (
        <div className="document-toc-empty">로딩 중...</div>
      ) : recentError ? (
        <div className="document-toc-empty">{recentError}</div>
      ) : (
        <ul className="document-toc-list document-toc-recent">
          {recentDocuments
            .filter((doc) => String(doc.id) !== String(currentDocumentId))
            .map((doc) => (
              <li key={doc.id}>
                <Link to={`/documents/${doc.id}`}>
                  <span className="document-toc-recent-title">{doc.title}</span>
                  <span className="document-toc-recent-date">{formatDate(doc.created_at)}</span>
                </Link>
              </li>
            ))}
          {recentDocuments.length === 0 && (
            <li className="document-toc-empty">최근 문서가 없습니다</li>
          )}
        </ul>
      )}
    </nav>
  );
};

DocumentToc.propTypes = {
  headings: PropTypes.arrayOf(
    PropTypes.shape({
      level: PropTypes.number.isRequired,
      text: PropTypes.string.isRequired,
      slug: PropTypes.string.isRequired,
    })
  ),
  currentDocumentId: PropTypes.oneOfType([PropTypes.string, PropTypes.number]),
  mode: PropTypes.oneOf(Object.values(TOC_MODES)).isRequired,
  onModeChange: PropTypes.func.isRequired,
};

export default DocumentToc;
