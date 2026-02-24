import React, { useState } from 'react';
import PropTypes from 'prop-types';
import { Link } from 'react-router-dom';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';
import './Chat.css';

const CodeBlock = ({ language, children }) => {
  const [copied, setCopied] = useState(false);
  const code = String(children).replace(/\n$/, '');

  const handleCopy = () => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div className="chat-code-block">
      <div className="chat-code-header">
        <span className="chat-code-lang">{language || 'text'}</span>
        <button
          className="chat-code-copy"
          onClick={handleCopy}
          aria-label="Copy code"
        >
          {copied ? 'Copied!' : 'Copy'}
        </button>
      </div>
      <SyntaxHighlighter
        style={oneDark}
        language={language || 'text'}
        PreTag="div"
        customStyle={{ margin: 0, borderRadius: '0 0 6px 6px' }}
      >
        {code}
      </SyntaxHighlighter>
    </div>
  );
};

CodeBlock.propTypes = {
  language: PropTypes.string,
  children: PropTypes.node.isRequired,
};

CodeBlock.defaultProps = {
  language: '',
};

const markdownComponents = {
  code({ node, inline, className, children, ...props }) {
    const match = /language-(\w+)/.exec(className || '');
    if (!inline && match) {
      return <CodeBlock language={match[1]}>{children}</CodeBlock>;
    }
    return (
      <code className="chat-inline-code" {...props}>
        {children}
      </code>
    );
  },
};

const StreamingCursor = () => (
  <span className="chat-streaming-cursor" aria-label="Generating response">▊</span>
);

const SourceCard = ({ source, index }) => {
  const { document_title, document_id, chunk_text, similarity } = source;
  const title = document_title || `Document ${index + 1}`;
  const preview = chunk_text?.slice(0, 100) || '';
  const similarityPercent = Math.round((similarity || 0) * 100);

  const cardContent = (
    <>
      <div className="chat-source-card__header">
        <span className="chat-source-card__number">[{index + 1}]</span>
        <span className="chat-source-card__title">{title}</span>
        <span className="chat-source-card__similarity">{similarityPercent}%</span>
      </div>
      {preview && (
        <p className="chat-source-card__preview">
          {preview}{chunk_text?.length > 100 ? '...' : ''}
        </p>
      )}
    </>
  );

  if (document_id) {
    return (
      <Link to={`/documents/${document_id}`} className="chat-source-card chat-source-card--clickable">
        {cardContent}
      </Link>
    );
  }

  return <div className="chat-source-card">{cardContent}</div>;
};

SourceCard.propTypes = {
  source: PropTypes.shape({
    document_title: PropTypes.string,
    document_id: PropTypes.string,
    chunk_text: PropTypes.string,
    similarity: PropTypes.number,
  }).isRequired,
  index: PropTypes.number.isRequired,
};

const ChatMessage = ({ message, onCopy = null }) => {
  const { role, content, timestamp, sources, isStreaming, isError, tokensUsed, model } = message;
  const isUser = role === 'user';
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(content).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
      if (onCopy) onCopy(message.id);
    });
  };

  const formattedTime = timestamp
    ? new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    : '';

  const messageClass = [
    'chat-message',
    isUser ? 'chat-message--user' : 'chat-message--ai',
    isStreaming ? 'chat-message--streaming' : '',
    isError ? 'chat-message--error' : '',
  ].filter(Boolean).join(' ');

  return (
    <div className={messageClass}>
      <div className="chat-message__avatar" aria-hidden="true">
        {isUser ? 'U' : 'AI'}
      </div>
      <div className="chat-message__body">
        <div className="chat-message__content">
          {isUser ? (
            <p className="chat-message__text">{content}</p>
          ) : (
            <>
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={markdownComponents}
              >
                {content || (isStreaming ? '' : 'Generating response...')}
              </ReactMarkdown>
              {isStreaming && <StreamingCursor />}
            </>
          )}
        </div>
        {sources && sources.length > 0 && (
          <div className="chat-message__sources">
            <details className="chat-message__sources-details" open={!isStreaming}>
              <summary className="chat-message__sources-label">
                Sources ({sources.length})
              </summary>
              <div className="chat-message__sources-list">
                {sources.map((source, i) => (
                  <SourceCard key={source.document_id || i} source={source} index={i} />
                ))}
              </div>
            </details>
          </div>
        )}
        <div className="chat-message__meta">
          {formattedTime && (
            <span className="chat-message__time">{formattedTime}</span>
          )}
          {tokensUsed && (
            <span className="chat-message__tokens" title={`Model: ${model || 'unknown'}`}>
              {tokensUsed} tokens
            </span>
          )}
          {!isStreaming && content && (
            <button
              className={`chat-message__copy ${copied ? 'chat-message__copy--copied' : ''}`}
              onClick={handleCopy}
              aria-label="Copy message"
              title="Copy message"
            >
              {copied ? (
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" aria-hidden="true">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              ) : (
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
                </svg>
              )}
              <span>{copied ? 'Copied!' : 'Copy'}</span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

ChatMessage.propTypes = {
  message: PropTypes.shape({
    id: PropTypes.string,
    role: PropTypes.oneOf(['user', 'assistant']).isRequired,
    content: PropTypes.string,
    timestamp: PropTypes.string,
    sources: PropTypes.arrayOf(
      PropTypes.shape({
        document_title: PropTypes.string,
        document_id: PropTypes.string,
        chunk_text: PropTypes.string,
        similarity: PropTypes.number,
      })
    ),
    isStreaming: PropTypes.bool,
    isError: PropTypes.bool,
    tokensUsed: PropTypes.number,
    model: PropTypes.string,
  }).isRequired,
  onCopy: PropTypes.func,
};

ChatMessage.defaultProps = {
  onCopy: null,
};

export default ChatMessage;
