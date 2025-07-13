import React, { useState, useEffect } from 'react';
import { useParams, Link, useNavigate } from 'react-router-dom';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { documentService } from '../services/api';
import { extractFrontmatter, processInternalLinks, processHashtags } from '../utils/obsidianRenderer';
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
        
        setError(null);
      } catch (err) {
        setError('Failed to fetch document');
        console.error('Error fetching document:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchDocument();
  }, [id]);

  const handleDelete = async () => {
    if (window.confirm('Are you sure you want to delete this document?')) {
      try {
        await documentService.deleteDocument(id);
        navigate('/');
      } catch (err) {
        setError('Failed to delete document');
        console.error('Error deleting document:', err);
      }
    }
  };

  const formatDate = (dateString) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  if (loading) {
    return <div className="loading">Loading document...</div>;
  }

  if (error) {
    return (
      <div className="error">
        {error}
        <Link to="/" className="btn btn-secondary">Back to Documents</Link>
      </div>
    );
  }

  if (!document) {
    return (
      <div className="error">
        Document not found
        <Link to="/" className="btn btn-secondary">Back to Documents</Link>
      </div>
    );
  }

  return (
    <div className="document-view">
      <div className="document-header">
        <div className="document-nav">
          <Link to="/" className="back-link">← Back to Documents</Link>
        </div>
        
        <div className="document-title-section">
          <h1 className="document-title">{document.title}</h1>
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
          <div className="document-meta">
            {document.author && (
              <span className="document-author">By {document.author}</span>
            )}
            <span className="document-dates">
              Created: {formatDate(document.created_at)}
              {document.updated_at !== document.created_at && (
                <span> • Updated: {formatDate(document.updated_at)}</span>
              )}
            </span>
          </div>
        </div>

        <div className="document-actions">
          <button
            className="btn btn-secondary"
            onClick={() => setShowMarkdown(!showMarkdown)}
          >
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
                // 텍스트 노드에서 이미 처리된 HTML을 렌더링
                text({ children }) {
                  if (typeof children === 'string' && 
                      (children.includes('<a') || children.includes('<span'))) {
                    return <span dangerouslySetInnerHTML={{ __html: children }} />;
                  }
                  return children;
                },
                code({ node, inline, className, children, ...props }) {
                  const match = /language-(\w+)/.exec(className || '');
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
                }
              }}
            >
              {processedContent || document.markdown_content}
            </ReactMarkdown>
          </div>
        )}
      </div>
    </div>
  );
};

export default DocumentView;