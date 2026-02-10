import React, { useState, useEffect, useRef } from 'react';
import PropTypes from 'prop-types';
import { tagService } from '../services/api';
import { logError } from '../utils/logger';
import './TagInput.css';

const TagInput = ({ tags = [], onChange, suggestedTags = [], onSuggestionApply }) => {
  const [inputValue, setInputValue] = useState('');
  const [suggestions, setSuggestions] = useState([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [showAISuggestions, setShowAISuggestions] = useState(false);
  const inputRef = useRef(null);

  useEffect(() => {
    if (suggestedTags.length > 0) {
      setShowAISuggestions(true);
    }
  }, [suggestedTags]);

  const searchTags = async (query) => {
    if (query.length < 2) {
      setSuggestions([]);
      return;
    }

    try {
      const response = await tagService.suggestTags(query);
      setSuggestions(response.suggestions || []);
    } catch (error) {
      logError('TagInput.searchTags', error);
      setSuggestions([]);
    }
  };

  const handleInputChange = (e) => {
    const value = e.target.value;
    setInputValue(value);
    setShowSuggestions(true);
    searchTags(value);
  };

  const handleInputKeyDown = (e) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      addTag(inputValue.trim());
    } else if (e.key === 'Backspace' && inputValue === '' && tags.length > 0) {
      removeTag(tags.length - 1);
    }
  };

  const addTag = (tagName) => {
    if (!tagName) return;
    
    const normalizedTagName = tagName.toLowerCase().trim();
    const existingTag = tags.find(tag => tag.toLowerCase() === normalizedTagName);
    
    if (!existingTag) {
      onChange([...tags, tagName]);
    }
    
    setInputValue('');
    setShowSuggestions(false);
    inputRef.current?.focus();
  };

  const removeTag = (index) => {
    const newTags = tags.filter((_, i) => i !== index);
    onChange(newTags);
  };

  const applySuggestedTag = (tagName) => {
    addTag(tagName);
    if (onSuggestionApply) {
      onSuggestionApply(tagName);
    }
  };

  const applyAllSuggestedTags = () => {
    const newTags = [...tags];
    suggestedTags.forEach(tag => {
      const normalizedTagName = tag.toLowerCase().trim();
      const exists = newTags.some(existingTag => existingTag.toLowerCase() === normalizedTagName);
      if (!exists) {
        newTags.push(tag);
      }
    });
    onChange(newTags);
    setShowAISuggestions(false);
    if (onSuggestionApply) {
      onSuggestionApply('all');
    }
  };

  return (
    <div className="tag-input-container">
      <div className="tag-input-wrapper">
        <div className="tag-chips">
          {tags.map((tag, index) => (
            <span key={index} className="tag-chip">
              {tag}
              <button
                type="button"
                className="tag-remove"
                onClick={() => removeTag(index)}
                aria-label={`Remove tag ${tag}`}
              >
                ×
              </button>
            </span>
          ))}
          <input
            ref={inputRef}
            id="tags"
            type="text"
            value={inputValue}
            onChange={handleInputChange}
            onKeyDown={handleInputKeyDown}
            onFocus={() => setShowSuggestions(true)}
            onBlur={() => setTimeout(() => setShowSuggestions(false), 200)}
            placeholder={tags.length === 0 ? "Add tags..." : ""}
            className="tag-input"
          />
        </div>
        
        {showSuggestions && suggestions.length > 0 && (
          <div className="tag-suggestions">
            {suggestions.map((suggestion, index) => (
              <button
                key={index}
                type="button"
                className="tag-suggestion"
                onMouseDown={() => addTag(suggestion.name)}
              >
                {suggestion.name}
              </button>
            ))}
          </div>
        )}
      </div>

      {showAISuggestions && suggestedTags.length > 0 && (
        <div className="ai-suggestions-panel">
          <div className="ai-suggestions-header">
            <span className="ai-suggestions-title">🤖 AI Tags Auto-Applied</span>
            <div className="ai-suggestions-actions">
              <span className="auto-applied-note">Tags automatically added based on content</span>
              <button
                type="button"
                className="btn-dismiss"
                onClick={() => setShowAISuggestions(false)}
              >
                ×
              </button>
            </div>
          </div>
          <div className="ai-suggested-tags">
            {suggestedTags.map((tag, index) => {
              const isAlreadyAdded = tags.some(existingTag => 
                existingTag.toLowerCase() === tag.toLowerCase()
              );
              return (
                <div
                  key={index}
                  className={`ai-suggested-tag ${isAlreadyAdded ? 'already-added' : 'not-applied'}`}
                >
                  {tag}
                  {isAlreadyAdded ? (
                    <span className="already-added-indicator">✓ Applied</span>
                  ) : (
                    <button
                      type="button"
                      className="btn-add-individual"
                      onClick={() => applySuggestedTag(tag)}
                    >
                      + Add
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};

TagInput.propTypes = {
  tags: PropTypes.arrayOf(PropTypes.string),
  onChange: PropTypes.func.isRequired,
  suggestedTags: PropTypes.arrayOf(PropTypes.string),
  onSuggestionApply: PropTypes.func
};

TagInput.defaultProps = {
  tags: [],
  suggestedTags: [],
  onSuggestionApply: null
};

export default TagInput;