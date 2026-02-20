import React from 'react';
import PropTypes from 'prop-types';
import './Chat.css';

const ChatHistory = ({ sessions, activeId = null, onSelect, onNew, onDelete = null }) => {
  if (!sessions.length) {
    return (
      <aside className="chat-history chat-history--empty">
        <button className="chat-history__new" onClick={onNew}>
          New Chat
        </button>
        <p className="chat-history__empty-text">No conversations yet</p>
      </aside>
    );
  }

  return (
    <aside className="chat-history">
      <div className="chat-history__header">
        <span className="chat-history__title">History</span>
        <button className="chat-history__new" onClick={onNew}>
          + New
        </button>
      </div>
      <ul className="chat-history__list">
        {sessions.map((session) => (
          <li
            key={session.id}
            className={`chat-history__item ${session.id === activeId ? 'chat-history__item--active' : ''}`}
          >
            <button
              className="chat-history__item-btn"
              onClick={() => onSelect(session.id)}
              aria-current={session.id === activeId ? 'true' : undefined}
            >
              <span className="chat-history__item-title">{session.title || 'Untitled'}</span>
              <span className="chat-history__item-date">
                {session.updatedAt
                  ? new Date(session.updatedAt).toLocaleDateString()
                  : ''}
              </span>
            </button>
            {onDelete && (
              <button
                className="chat-history__item-delete"
                onClick={() => onDelete(session.id)}
                aria-label={`Delete "${session.title || 'Untitled'}"`}
              >
                x
              </button>
            )}
          </li>
        ))}
      </ul>
    </aside>
  );
};

ChatHistory.propTypes = {
  sessions: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.string.isRequired,
      title: PropTypes.string,
      updatedAt: PropTypes.string,
    })
  ).isRequired,
  activeId: PropTypes.string,
  onSelect: PropTypes.func.isRequired,
  onNew: PropTypes.func.isRequired,
  onDelete: PropTypes.func,
};

export default ChatHistory;
