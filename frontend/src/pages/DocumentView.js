import { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
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
import Toast from '../components/Toast';
import useToast from '../hooks/useToast';
import useDocumentStatus from '../hooks/useDocumentStatus';
import '../components/DocumentCard.css';
import './DocumentView.css';

const DocumentView = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const [document, setDocument] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [showMarkdown, setShowMarkdown] = useState(false);
  const [frontmatter, setFrontmatter] = useState({});
  const [processedContent, setProcessedContent] = useState('');
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

        // Auto-generate tags if document has no tags
        if (data && (!data.tags || data.tags.length === 0) && data.markdown_content) {
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
        navigate('/');
      } catch (err) {
        setError('Failed to delete document');
        logError('DocumentView.handleDelete', err);
      }
    }
  };

  const handleReprocess = async () => {
    try {
      await documentService.reprocessDocument(id);
      showToast('Document queued for reprocessing', 'success');
      const data = await documentService.getDocument(id);
      setDocument(data);
      startPolling();
    } catch (err) {
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
        <Link to="/" className="btn btn-secondary">
          Back to Documents
        </Link>
      </div>
    );
  }

  if (!document) {
    return (
      <div className="error">
        Document not found
        <Link to="/" className="btn btn-secondary">
          Back to Documents
        </Link>
      </div>
    );
  }

  return (
    <div className="document-view">
      <div className="document-header">
        <div className="document-nav">
          <Link to="/" className="back-link">
            ← Back to Documents
          </Link>
        </div>

        <div className="document-title-section">
          <h1 className="document-title">{document.title}</h1>
          {/* Auto-tagging status indicator */}
          {autoTaggingInProgress && (
            <div className="auto-tagging-status">🤖 Analyzing content and generating tags...</div>
          )}

          {/* Tags display in 8pt font below title */}
          {document.tags && document.tags.length > 0 && (
            <div className="document-tags">
              {document.tags.map((tag, index) => (
                <span key={index} className="document-tag">
                  {tag.name || tag}
                </span>
              ))}
            </div>
          )}

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
              <span className="processing-badge processing-badge--pending" aria-label="Processing pending">
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
              <span className="processing-badge processing-badge--completed" aria-label="Processing completed">
                Completed
              </span>
            )}
          </div>
        </div>

        <div className="document-actions">
          <button className="btn btn-secondary" onClick={() => setShowMarkdown(!showMarkdown)}>
            {showMarkdown ? 'Show Rendered' : 'Show Markdown'}
          </button>
          <Link to={`/documents/${id}/edit`} className="btn btn-primary">
            Edit
          </Link>
          <button className="btn btn-danger" onClick={handleDelete}>
            Delete
          </button>
        </div>
      </div>

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
                  return !inline && match ? (
                    <SyntaxHighlighter style={tomorrow} language={match[1]} PreTag="div" {...props}>
                      {String(children).replace(/\n$/, '')}
                    </SyntaxHighlighter>
                  ) : (
                    <code className={className} {...props}>
                      {children}
                    </code>
                  );
                },
              }}
            >
              {processedContent || document.markdown_content}
            </ReactMarkdown>
          </div>
        )}
      </div>
      {toast && (
        <Toast message={toast.message} type={toast.type} onDismiss={dismissToast} />
      )}
    </div>
  );
};

export default DocumentView;
