import React, { useState, useEffect, useCallback } from 'react';
import PropTypes from 'prop-types';
import api from '../services/api';
import { logError } from '../utils/logger';
import './AISuggestions.css';

const AISuggestions = ({ 
  content, 
  cursorPosition, 
  onSuggestionSelect, 
  onTitleSuggestion,
  onTagSuggestions,
  isVisible = true 
}) => {
  const [suggestions, setSuggestions] = useState([]);
  const [loading, setLoading] = useState(false);
  const [aiEnabled, setAiEnabled] = useState(false);
  const [showWritingSuggestions, setShowWritingSuggestions] = useState(false);
  const [writingSuggestions, setWritingSuggestions] = useState([]);

  // Check AI service status on component mount
  useEffect(() => {
    checkAIStatus();
  }, []);

  const checkAIStatus = async () => {
    try {
      const response = await api.get('/ai/status');
      setAiEnabled(response.data.enabled);
    } catch (error) {
      logError('AISuggestions.checkAIStatus', error);
      setAiEnabled(false);
    }
  };

  // Debounced content suggestions
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const getSuggestions = useCallback(
    debounce(async (currentContent, cursor) => {
      if (!aiEnabled || !currentContent.trim()) {
        setSuggestions([]);
        return;
      }

      try {
        setLoading(true);
        const response = await api.post('/ai/suggestions', {
          content: currentContent,
          cursor_position: cursor,
          max_suggestions: 3
        });

        if (response.data.success) {
          setSuggestions(response.data.suggestions);
        }
      } catch (error) {
        logError('AISuggestions.getSuggestions', error);
        setSuggestions([]);
      } finally {
        setLoading(false);
      }
    }, 1000),
    [aiEnabled]
  );

  // Get suggestions when content changes
  useEffect(() => {
    if (content && content.length > 10) {
      getSuggestions(content, cursorPosition);
    } else {
      setSuggestions([]);
    }
  }, [content, cursorPosition, getSuggestions]);

  const handleSuggestionClick = (suggestion) => {
    if (onSuggestionSelect) {
      onSuggestionSelect(suggestion.text);
    }
  };

  const getAutoCompletion = async () => {
    if (!aiEnabled || !content.trim()) return;

    try {
      const response = await api.post('/ai/autocomplete', {
        content,
        cursor_position: cursorPosition
      });

      if (response.data.success && response.data.completion) {
        if (onSuggestionSelect) {
          onSuggestionSelect(response.data.completion);
        }
      }
    } catch (error) {
      logError('AISuggestions.getAutoCompletion', error);
    }
  };

  const suggestTitle = async () => {
    if (!content.trim()) return;

    try {
      const response = await api.post('/ai/suggest-title', {
        content
      });

      if (response.data.success && response.data.suggested_title) {
        if (onTitleSuggestion) {
          onTitleSuggestion(response.data.suggested_title);
        }
      }
    } catch (error) {
      logError('AISuggestions.suggestTitle', error);
    }
  };

  const suggestTags = async () => {
    if (!content.trim()) return;

    try {
      const response = await api.post('/ai/suggest-tags', {
        content,
        title: '' // Could be passed as prop if needed
      });

      if (response.data.success && response.data.suggested_tags) {
        if (onTagSuggestions) {
          onTagSuggestions(response.data.suggested_tags);
        }
      }
    } catch (error) {
      logError('AISuggestions.suggestTags', error);
    }
  };

  const getWritingSuggestions = async () => {
    if (!aiEnabled || !content.trim()) return;

    try {
      setLoading(true);
      const response = await api.post('/ai/writing-suggestions', {
        content
      });

      if (response.data.success) {
        setWritingSuggestions(response.data.suggestions);
        setShowWritingSuggestions(true);
      }
    } catch (error) {
      logError('AISuggestions.getWritingSuggestions', error);
    } finally {
      setLoading(false);
    }
  };

  if (!isVisible || (!aiEnabled && suggestions.length === 0)) {
    return null;
  }

  return (
    <div className="ai-suggestions">
      <div className="ai-suggestions-header">
        <h4>✨ AI Assistant</h4>
        {!aiEnabled && (
          <span className="ai-status-badge disabled">
            Limited Mode
          </span>
        )}
        {aiEnabled && (
          <span className="ai-status-badge enabled">
            AI Powered
          </span>
        )}
      </div>

      <div className="ai-actions">
        <button 
          className="ai-action-btn"
          onClick={getAutoCompletion}
          disabled={!aiEnabled || loading}
        >
          🔮 Auto Complete
        </button>
        
        <button 
          className="ai-action-btn"
          onClick={suggestTitle}
          disabled={loading}
        >
          📝 Suggest Title
        </button>
        
        <button 
          className="ai-action-btn"
          onClick={suggestTags}
          disabled={loading}
        >
          🏷️ Suggest Tags
        </button>
        
        <button 
          className="ai-action-btn"
          onClick={getWritingSuggestions}
          disabled={!aiEnabled || loading}
        >
          📖 Writing Tips
        </button>
      </div>

      {loading && (
        <div className="ai-loading">
          <div className="loading-spinner"></div>
          <span>Getting AI suggestions...</span>
        </div>
      )}

      {suggestions.length > 0 && (
        <div className="ai-suggestions-list">
          <h5>Content Suggestions</h5>
          {suggestions.map((suggestion, index) => (
            <div 
              key={index}
              className={`suggestion-item ${suggestion.type}`}
              onClick={() => handleSuggestionClick(suggestion)}
            >
              <div className="suggestion-type">
                {suggestion.type === 'completion' ? '💡' : '✏️'}
              </div>
              <div className="suggestion-text">
                {suggestion.text}
              </div>
            </div>
          ))}
        </div>
      )}

      {showWritingSuggestions && writingSuggestions.length > 0 && (
        <div className="writing-suggestions">
          <div className="writing-suggestions-header">
            <h5>Writing Suggestions</h5>
            <button 
              className="close-btn"
              onClick={() => setShowWritingSuggestions(false)}
            >
              ×
            </button>
          </div>
          {writingSuggestions.map((suggestion, index) => (
            <div key={index} className="writing-suggestion-item">
              <div className="suggestion-icon">💡</div>
              <div className="suggestion-content">
                {suggestion.text}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

// Debounce utility function
function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

AISuggestions.propTypes = {
  content: PropTypes.string,
  cursorPosition: PropTypes.number,
  onSuggestionSelect: PropTypes.func,
  onTitleSuggestion: PropTypes.func,
  onTagSuggestions: PropTypes.func,
  isVisible: PropTypes.bool
};

AISuggestions.defaultProps = {
  content: '',
  cursorPosition: 0,
  onSuggestionSelect: null,
  onTitleSuggestion: null,
  onTagSuggestions: null,
  isVisible: true
};

export default AISuggestions;