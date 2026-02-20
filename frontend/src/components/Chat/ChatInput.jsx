import React, { useState, useRef, useCallback } from 'react';
import PropTypes from 'prop-types';
import './Chat.css';

const MAX_LENGTH = 4000;

const ChatInput = ({ onSend, disabled = false }) => {
  const [value, setValue] = useState('');
  const textareaRef = useRef(null);

  const resizeTextarea = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = 'auto';
    el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
  }, []);

  const handleChange = (e) => {
    const next = e.target.value;
    if (next.length > MAX_LENGTH) return;
    setValue(next);
    resizeTextarea();
  };

  const handleSubmit = () => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSend(trimmed);
    setValue('');
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
    }
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const remaining = MAX_LENGTH - value.length;
  const isNearLimit = remaining < 200;

  return (
    <div className="chat-input">
      <div className="chat-input__wrapper">
        <textarea
          ref={textareaRef}
          className="chat-input__textarea"
          placeholder="Ask anything about your team's knowledge..."
          value={value}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          disabled={disabled}
          rows={1}
          aria-label="Chat message input"
          aria-describedby="chat-input-hint"
        />
        <button
          className="chat-input__send"
          onClick={handleSubmit}
          disabled={disabled || !value.trim()}
          aria-label="Send message"
        >
          Send
        </button>
      </div>
      <div className="chat-input__footer">
        <span id="chat-input-hint" className="chat-input__hint">
          Enter to send, Shift+Enter for new line
        </span>
        {isNearLimit && (
          <span className={`chat-input__counter ${remaining < 50 ? 'chat-input__counter--warn' : ''}`}>
            {remaining}
          </span>
        )}
      </div>
    </div>
  );
};

ChatInput.propTypes = {
  onSend: PropTypes.func.isRequired,
  disabled: PropTypes.bool,
};

export default ChatInput;
