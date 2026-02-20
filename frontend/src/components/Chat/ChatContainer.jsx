import React, { useEffect, useRef, useCallback } from 'react';
import PropTypes from 'prop-types';
import ChatMessage from './ChatMessage';
import ChatInput from './ChatInput';
import ChatHistory from './ChatHistory';
import TypingIndicator from './TypingIndicator';
import { useChat } from '../../hooks/useChat';
import './Chat.css';

const EmptyState = () => (
  <div className="chat-empty">
    <div className="chat-empty__icon" aria-hidden="true">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
      </svg>
    </div>
    <h2 className="chat-empty__title">Ask your team's knowledge</h2>
    <p className="chat-empty__subtitle">
      Search across documents, notes, and team conversations using natural language.
    </p>
    <ul className="chat-empty__suggestions">
      <li>"How did we solve the authentication issue last quarter?"</li>
      <li>"What's our approach to database migrations?"</li>
      <li>"Summarize our onboarding process"</li>
    </ul>
  </div>
);

const ChatContainer = ({ className }) => {
  const {
    sessions,
    activeSessionId,
    messages,
    isLoading,
    error,
    sendMessage,
    selectSession,
    createSession,
    deleteSession,
  } = useChat();

  const messagesEndRef = useRef(null);
  const messagesContainerRef = useRef(null);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, isLoading, scrollToBottom]);

  const handleSend = useCallback(
    async (content) => {
      await sendMessage(content);
    },
    [sendMessage]
  );

  return (
    <div className={`chat-container ${className || ''}`}>
      <ChatHistory
        sessions={sessions}
        activeId={activeSessionId}
        onSelect={selectSession}
        onNew={createSession}
        onDelete={deleteSession}
      />

      <div className="chat-main">
        {error && (
          <div className="chat-error" role="alert">
            {error}
          </div>
        )}

        <div
          className="chat-messages"
          ref={messagesContainerRef}
          role="log"
          aria-label="Chat messages"
          aria-live="polite"
        >
          {messages.length === 0 && !isLoading ? (
            <EmptyState />
          ) : (
            messages.map((message) => (
              <ChatMessage key={message.id} message={message} />
            ))
          )}
          {isLoading && <TypingIndicator />}
          <div ref={messagesEndRef} />
        </div>

        <ChatInput onSend={handleSend} disabled={isLoading} />
      </div>
    </div>
  );
};

ChatContainer.propTypes = {
  className: PropTypes.string,
};

ChatContainer.defaultProps = {
  className: '',
};

export default ChatContainer;
