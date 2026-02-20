import React from 'react';
import './Chat.css';

const TypingIndicator = () => (
  <div className="chat-message chat-message--ai" aria-live="polite" aria-label="AI is typing">
    <div className="chat-message__avatar" aria-hidden="true">AI</div>
    <div className="chat-message__body">
      <div className="chat-typing">
        <span className="chat-typing__dot" />
        <span className="chat-typing__dot" />
        <span className="chat-typing__dot" />
      </div>
    </div>
  </div>
);

export default TypingIndicator;
