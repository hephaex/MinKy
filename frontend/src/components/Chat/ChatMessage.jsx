import React, { useState } from 'react';
import PropTypes from 'prop-types';
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

const ChatMessage = ({ message, onCopy = null }) => {
  const { role, content, timestamp, sources } = message;
  const isUser = role === 'user';

  const handleCopy = () => {
    navigator.clipboard.writeText(content).then(() => {
      if (onCopy) onCopy(message.id);
    });
  };

  const formattedTime = timestamp
    ? new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    : '';

  return (
    <div className={`chat-message ${isUser ? 'chat-message--user' : 'chat-message--ai'}`}>
      <div className="chat-message__avatar" aria-hidden="true">
        {isUser ? 'U' : 'AI'}
      </div>
      <div className="chat-message__body">
        <div className="chat-message__content">
          {isUser ? (
            <p className="chat-message__text">{content}</p>
          ) : (
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={markdownComponents}
            >
              {content}
            </ReactMarkdown>
          )}
        </div>
        {sources && sources.length > 0 && (
          <div className="chat-message__sources">
            <span className="chat-message__sources-label">Sources:</span>
            <ul className="chat-message__sources-list">
              {sources.map((source, i) => (
                <li key={i}>
                  <a
                    href={source.url || '#'}
                    className="chat-message__source-link"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    {source.title || source.url}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        )}
        <div className="chat-message__meta">
          {formattedTime && (
            <span className="chat-message__time">{formattedTime}</span>
          )}
          <button
            className="chat-message__copy"
            onClick={handleCopy}
            aria-label="Copy message"
            title="Copy message"
          >
            Copy
          </button>
        </div>
      </div>
    </div>
  );
};

ChatMessage.propTypes = {
  message: PropTypes.shape({
    id: PropTypes.string,
    role: PropTypes.oneOf(['user', 'assistant']).isRequired,
    content: PropTypes.string.isRequired,
    timestamp: PropTypes.string,
    sources: PropTypes.arrayOf(
      PropTypes.shape({
        title: PropTypes.string,
        url: PropTypes.string,
      })
    ),
  }).isRequired,
  onCopy: PropTypes.func,
};

export default ChatMessage;
