import React from 'react';
import PropTypes from 'prop-types';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import SourceDocuments from './SourceDocuments';
import './AnswerDisplay.css';

const AnswerDisplay = ({
  answer,
  sources = [],
  question = '',
  streaming = false,
}) => {
  if (!answer && !streaming) return null;

  return (
    <div className="kb-answer" role="region" aria-label="AI 답변">
      <div className="kb-answer-header">
        <div className="kb-answer-badge">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
            <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z" />
            <path d="M12 6v6l4 2" />
          </svg>
          AI 답변
        </div>
        {question && (
          <p className="kb-answer-question" aria-label="질문">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" aria-hidden="true">
              <circle cx="12" cy="12" r="10" />
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            {question}
          </p>
        )}
      </div>

      <div className={`kb-answer-body ${streaming ? 'kb-answer-body--streaming' : ''}`}>
        {answer ? (
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            className="kb-answer-markdown"
            components={{
              a: ({ href, children }) => (
                <a href={href} target="_blank" rel="noopener noreferrer">
                  {children}
                </a>
              ),
            }}
          >
            {answer}
          </ReactMarkdown>
        ) : (
          <div className="kb-answer-generating" aria-live="polite">
            <span className="kb-answer-dot" />
            <span className="kb-answer-dot" />
            <span className="kb-answer-dot" />
            <span className="sr-only">답변 생성 중...</span>
          </div>
        )}
        {streaming && answer && (
          <span className="kb-answer-cursor" aria-hidden="true" />
        )}
      </div>

      {sources.length > 0 && !streaming && (
        <div className="kb-answer-sources">
          <SourceDocuments sources={sources} />
        </div>
      )}
    </div>
  );
};

AnswerDisplay.propTypes = {
  answer: PropTypes.string,
  sources: PropTypes.array,
  question: PropTypes.string,
  streaming: PropTypes.bool,
};

export default AnswerDisplay;
