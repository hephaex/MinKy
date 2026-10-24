import { useMemo, useState, useEffect } from 'react';
import { useParams, Link, useNavigate, useLocation } from 'react-router-dom';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';
import DOMPurify from 'dompurify';
import { logError } from '../utils/logger';
import { formatDateTime } from '../utils/dateUtils';
import { formatAuthor } from '../utils/documentUtils';
import {
  extractFrontmatter,
  processInternalLinks,
  processHashtags,
} from '../utils/obsidianRenderer';
import api, { documentService } from '../services/api';
import DocumentToc, { TOC_MODES } from '../components/DocumentToc';
import { extractHeadings } from '../utils/tocUtils';
import Toast from '../components/Toast';
import useToast from '../hooks/useToast';
import useDocumentStatus from '../hooks/useDocumentStatus';
import MermaidDiagram from '../components/MermaidDiagram';
import '../components/DocumentCard.css';
import './DocumentView.css';

/**
 * ReactMarkdown heading renderer that assigns the `id` matching the DocumentToc
 * panel's Nth-heading anchor (`nextIndex()` — a mutable per-render cursor owned
 * by the caller — not a re-derived slug; see the `headingCursor` comment in
 * DocumentView). `Tag` is a lowercase tag-name string ('h1'..'h6'); JSX renders
 * it as a real DOM element because it's referenced via a capitalized variable.
 */
function renderHeadingWithId(Tag, props, headings, nextIndex) {
  const { node: _node, children, ...rest } = props;
  const heading = headings[nextIndex()];
  return (
    <Tag id={heading?.slug} {...rest}>
      {children}
    </Tag>
  );
}

const DocumentView = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const location = useLocation();

  // Prev / Next from the last-visited document list.
  const { prevId, nextId } = (() => {
    try {
      const ctx = JSON.parse(sessionStorage.getItem('docListContext'));
      if (!ctx?.ids) return { prevId: null, nextId: null };
      const idx = ctx.ids.indexOf(id);
      return {
        prevId: idx > 0 ? ctx.ids[idx - 1] : null,
        nextId: idx !== -1 && idx < ctx.ids.length - 1 ? ctx.ids[idx + 1] : null,
      };
    } catch (_) {
      return { prevId: null, nextId: null };
    }
  })();

  // Return to the exact previous view (its list page/filters are restored from
  // sessionStorage) when the document was opened from within the app; fall back
  // to all documents when it was opened directly (no in-app history).
  const goToPreviousView = () => {
    if (location.key !== 'default') {
      navigate(-1);
    } else {
      navigate('/');
    }
  };

  const handleBack = (e) => {
    // Let direct loads fall through to the link's '/' href.
    if (location.key !== 'default') {
      e.preventDefault();
      navigate(-1);
    }
  };

  const [document, setDocument] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [showMarkdown, setShowMarkdown] = useState(false);
  const [frontmatter, setFrontmatter] = useState({});
  const [processedContent, setProcessedContent] = useState('');
  const [tocMode, setTocMode] = useState(TOC_MODES.TOC);
  const [tocVisible, setTocVisible] = useState(true);
  const [autoTaggingInProgress, setAutoTaggingInProgress] = useState(false);
  const [suggestedTags, setSuggestedTags] = useState([]);
  const { toast, showToast, dismissToast } = useToast();
  const {
    status: liveStatus,
    queuePosition,
    errorMessage: statusError,
    isPolling,
    startPolling,
  } = useDocumentStatus(id);

  const processingStatus = liveStatus || document?.processing_status;

  // Extracted from the exact string ReactMarkdown renders (post obsidian
  // link/hashtag processing) so heading ids assigned during render (see the
  // `components` prop below) line up 1:1, in order, with this list — the
  // DocumentToc panel's anchors always resolve to the right heading.
  const headings = useMemo(
    () => extractHeadings(processedContent || document?.markdown_content),
    [processedContent, document]
  );

  const generateAndApplyTags = async (documentData) => {
    try {
      setAutoTaggingInProgress(true);
      // Get AI tag suggestions
      const response = await api.post('/ai/suggest-tags', {
        title: documentData.title,
        content: documentData.markdown_content,
      });

      if (response.data.success && response.data.suggested_tags?.length > 0) {
        const suggestedTags = response.data.suggested_tags;
        setSuggestedTags(suggestedTags);

        // Automatically apply the suggested tags to the document
        const updateResponse = await api.put(`/documents/${documentData.id}`, {
          title: documentData.title,
          author: documentData.author,
          markdown_content: documentData.markdown_content,
          tags: suggestedTags,
        });

        // Update the document state with the new tags
        setDocument((prevDoc) => ({
          ...prevDoc,
          tags: updateResponse.data.tags || suggestedTags.map((tagName) => ({ name: tagName })),
        }));

        // tags applied
      }
    } catch (error) {
      logError('DocumentView.generateAndApplyTags', error);
    } finally {
      setAutoTaggingInProgress(false);
    }
  };

  useEffect(() => {
    const fetchDocument = async () => {
      try {
        setLoading(true);
        const data = await documentService.getDocument(id);
        setDocument(data);

        // 옵시디언 스타일 콘텐츠 처리
        if (data.markdown_content) {
          const { metadata, content } = extractFrontmatter(data.markdown_content);
          setFrontmatter(metadata);

          // 내부 링크와 해시태그 처리
          let processed = processInternalLinks(content, navigate);
          processed = processHashtags(processed);
          setProcessedContent(processed);
        }

        // Auto-generate tags if document has no tags. Rust's single-document
        // API returns `tag_names` (plain strings), not `tags` — check both so
        // an already-tagged document isn't misread as untagged on every load.
        const existingTags = data?.tags ?? data?.tag_names;
        if (data && (!existingTags || existingTags.length === 0) && data.markdown_content) {
          await generateAndApplyTags(data);
        }

        if (data.processing_status === 'pending') {
          startPolling();
        }

        setError(null);
      } catch (err) {
        setError('Failed to fetch document');
        logError('DocumentView.fetchDocument', err);
      } finally {
        setLoading(false);
      }
    };

    fetchDocument();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);

  useEffect(() => {
    if (statusError) showToast(statusError, 'error');
  }, [statusError, showToast]);

  const handleDelete = async () => {
    if (window.confirm('Are you sure you want to delete this document?')) {
      try {
        await api.delete(`/documents/${id}`);
        goToPreviousView();
      } catch (err) {
        setError('Failed to delete document');
        logError('DocumentView.handleDelete', err);
      }
    }
  };

  const handleReprocess = async () => {
    const previousStatus = document?.processing_status;
    setDocument((prev) => (prev ? { ...prev, processing_status: 'pending' } : prev));
    try {
      await documentService.reprocessDocument(id);
      showToast('Document queued for reprocessing', 'success');
      startPolling();
    } catch (err) {
      setDocument((prev) => (prev ? { ...prev, processing_status: previousStatus } : prev));
      showToast('Failed to reprocess document', 'error');
      logError('DocumentView.handleReprocess', err);
    }
  };

  if (loading) {
    return <div className="loading">Loading document...</div>;
  }

  if (error) {
    return (
      <div className="error">
        {error}
        <Link to="/" className="btn btn-secondary" onClick={handleBack}>
          Back to Documents
        </Link>
      </div>
    );
  }

  if (!document) {
    return (
      <div className="error">
        Document not found
        <Link to="/" className="btn btn-secondary" onClick={handleBack}>
          Back to Documents
        </Link>
      </div>
    );
  }

  // Positional cursor consumed by the h1-h6 renderers below: React invokes
  // them synchronously, in document order, once per render — so "the Nth
  // heading rendered so far" always matches `headings[Nth]` from the useMemo
  // above without the renderer needing to re-derive (and potentially
  // diverge on) its own slug from React's processed `children` nodes.
  let headingCursor = 0;

  return (
    <div className="document-view">
      <div className="document-header">
        <div className="document-nav">
          <Link to="/" className="back-link" onClick={handleBack}>
            ← Back to Documents
          </Link>
        </div>

        <div className="document-title-section">
          <h1 className="document-title">{document.title}</h1>
          {/* Auto-tagging status indicator */}
          {autoTaggingInProgress && (
            <div className="auto-tagging-status">🤖 Analyzing content and generating tags...</div>
          )}

          {/* Tags display in 8pt font below title. Rust's single-document API
              returns `tag_names` (plain strings); legacy sent `tags`. */}
          {(() => {
            const tags = document.tags ?? document.tag_names;
            return (
              tags &&
              tags.length > 0 && (
                <div className="document-tags">
                  {tags.map((tag, index) => (
                    <span key={index} className="document-tag">
                      {tag.name || tag}
                    </span>
                  ))}
                </div>
              )
            );
          })()}

          {/* Show recently applied AI tags */}
          {suggestedTags.length > 0 && !autoTaggingInProgress && (
            <div className="ai-tags-applied">
              ✨ AI automatically added tags: {suggestedTags.join(', ')}
            </div>
          )}
          <div className="document-meta">
            {document.author && (
              <span className="document-author">By {formatAuthor(document.author)}</span>
            )}
            <span className="document-dates">
              Created: {formatDateTime(document.created_at)}
              {document.updated_at !== document.created_at && (
                <span> • Updated: {formatDateTime(document.updated_at)}</span>
              )}
            </span>
            {processingStatus === 'pending' && (
              <span
                className="processing-badge processing-badge--pending"
                aria-label="Processing pending"
              >
                {isPolling ? 'Processing' : 'Pending'}
                {queuePosition != null && ` (#${queuePosition})`}
              </span>
            )}
            {processingStatus === 'failed' && (
              <button
                className="processing-badge processing-badge--failed processing-badge--clickable"
                aria-label="Processing failed. Click to retry."
                onClick={handleReprocess}
              >
                Failed — Retry
              </button>
            )}
            {processingStatus === 'completed' && (
              <span
                className="processing-badge processing-badge--completed"
                aria-label="Processing completed"
              >
                Completed
              </span>
            )}
          </div>
        </div>

        <div className="document-actions">
          <button className="btn btn-secondary no-print" onClick={() => setTocVisible(!tocVisible)}>
            {tocVisible ? '목차 숨기기' : '목차 보기'}
          </button>
          <button className="btn btn-secondary" onClick={() => setShowMarkdown(!showMarkdown)}>
            {showMarkdown ? 'Show Rendered' : 'Show Markdown'}
          </button>
          <button className="btn btn-secondary no-print" onClick={() => window.print()}>
            🖨️ PDF 출력
          </button>
          <Link to={`/documents/${id}/edit`} className="btn btn-primary no-print">
            Edit
          </Link>
          <button className="btn btn-danger no-print" onClick={handleDelete}>
            Delete
          </button>
        </div>
      </div>

      <div className="document-body">
        {!showMarkdown && tocVisible && (
          <DocumentToc
            headings={headings}
            currentDocumentId={id}
            mode={tocMode}
            onModeChange={setTocMode}
          />
        )}

        <div className="document-content">
          {showMarkdown ? (
            <div className="markdown-source">
              <h3>Markdown Source</h3>
              <pre className="markdown-code">{document.markdown_content}</pre>
            </div>
          ) : (
            <div className="markdown-rendered">
              {/* 프론트매터 표시 */}
              {Object.keys(frontmatter).length > 0 && (
                <div className="frontmatter-display">
                  <h4>메타데이터</h4>
                  <div className="metadata-grid">
                    {Object.entries(frontmatter).map(([key, value]) => (
                      <div key={key} className="metadata-item">
                        <strong>{key}:</strong>
                        <span>{Array.isArray(value) ? value.join(', ') : String(value)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  // 텍스트 노드에서 이미 처리된 HTML을 DOMPurify로 살균 후 렌더링
                  text({ children }) {
                    if (
                      typeof children === 'string' &&
                      (children.includes('<a') || children.includes('<span'))
                    ) {
                      const sanitized = DOMPurify.sanitize(children, {
                        ALLOWED_TAGS: ['a', 'span'],
                        ALLOWED_ATTR: ['href', 'class', 'data-target', 'title'],
                      });
                      return <span dangerouslySetInnerHTML={{ __html: sanitized }} />;
                    }
                    return children;
                  },
                  // 이미지 컴포넌트 - 반응형 스타일링 보장
                  img({ src, alt, title, ...props }) {
                    return (
                      <img
                        src={src}
                        alt={alt}
                        title={title}
                        style={{
                          maxWidth: '100%',
                          height: 'auto',
                          display: 'block',
                          margin: '1em auto',
                          borderRadius: '4px',
                          boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
                        }}
                        {...props}
                      />
                    );
                  },
                  code({ inline, className, children, ...props }) {
                    const match = /language-(\w+)/.exec(className || '');
                    if (!inline && match && match[1] === 'mermaid') {
                      return <MermaidDiagram code={String(children).replace(/\n$/, '')} />;
                    }
                    return !inline && match ? (
                      <SyntaxHighlighter
                        style={tomorrow}
                        language={match[1]}
                        PreTag="div"
                        {...props}
                      >
                        {String(children).replace(/\n$/, '')}
                      </SyntaxHighlighter>
                    ) : (
                      <code className={className} {...props}>
                        {children}
                      </code>
                    );
                  },
                  // h1-h6: assign the id the DocumentToc panel links to. See
                  // the `headingCursor` comment above `return (` — position,
                  // not re-derived text, is the source of truth for which
                  // `headings[]` entry a given rendered heading corresponds to.
                  h1: (props) => renderHeadingWithId('h1', props, headings, () => headingCursor++),
                  h2: (props) => renderHeadingWithId('h2', props, headings, () => headingCursor++),
                  h3: (props) => renderHeadingWithId('h3', props, headings, () => headingCursor++),
                  h4: (props) => renderHeadingWithId('h4', props, headings, () => headingCursor++),
                  h5: (props) => renderHeadingWithId('h5', props, headings, () => headingCursor++),
                  h6: (props) => renderHeadingWithId('h6', props, headings, () => headingCursor++),
                }}
              >
                {processedContent || document.markdown_content}
              </ReactMarkdown>
            </div>
          )}
        </div>
      </div>
      {/* Bottom navigation: list / prev / next */}
      <div className="document-bottom-nav">
        <Link to="/" className="doc-nav-btn doc-nav-list" onClick={handleBack}>
          ← 목록으로
        </Link>
        <div className="doc-nav-adjacent">
          {prevId ? (
            <Link to={`/documents/${prevId}`} className="doc-nav-btn doc-nav-prev">
              ← 이전 문서
            </Link>
          ) : (
            <span className="doc-nav-btn doc-nav-disabled">← 이전 문서</span>
          )}
          {nextId ? (
            <Link to={`/documents/${nextId}`} className="doc-nav-btn doc-nav-next">
              다음 문서 →
            </Link>
          ) : (
            <span className="doc-nav-btn doc-nav-disabled">다음 문서 →</span>
          )}
        </div>
      </div>

      {toast && <Toast message={toast.message} type={toast.type} onDismiss={dismissToast} />}
    </div>
  );
};

export default DocumentView;
