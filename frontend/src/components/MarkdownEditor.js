import React, { useState, useRef, useCallback } from 'react';
import MDEditor from '@uiw/react-md-editor';
import AISuggestions from './AISuggestions';
import './MarkdownEditor.css';

const MarkdownEditor = ({ 
  value, 
  onChange, 
  placeholder = "Start writing your markdown...",
  onTitleSuggestion,
  onTagSuggestions,
  showAISuggestions = true
}) => {
  const [previewMode, setPreviewMode] = useState('edit');
  const [cursorPosition, setCursorPosition] = useState(0);
  const editorRef = useRef(null);

  const handleEditorChange = useCallback((val) => {
    onChange(val);
    // Update cursor position when content changes
    if (editorRef.current && editorRef.current.textarea) {
      setCursorPosition(editorRef.current.textarea.selectionStart);
    }
  }, [onChange]);

  const handleSuggestionSelect = useCallback((suggestionText) => {
    if (!value) return;
    
    const currentPos = cursorPosition || value.length;
    const beforeCursor = value.substring(0, currentPos);
    const afterCursor = value.substring(currentPos);
    
    // Insert suggestion at cursor position
    const newValue = beforeCursor + suggestionText + afterCursor;
    onChange(newValue);
    
    // Update cursor position
    setCursorPosition(currentPos + suggestionText.length);
  }, [value, cursorPosition, onChange]);

  return (
    <div className="markdown-editor-container">
      <div className="editor-toolbar">
        <div className="editor-mode-tabs">
          <button
            className={`mode-tab ${previewMode === 'edit' ? 'active' : ''}`}
            onClick={() => setPreviewMode('edit')}
          >
            Edit
          </button>
          <button
            className={`mode-tab ${previewMode === 'preview' ? 'active' : ''}`}
            onClick={() => setPreviewMode('preview')}
          >
            Preview
          </button>
          <button
            className={`mode-tab ${previewMode === 'live' ? 'active' : ''}`}
            onClick={() => setPreviewMode('live')}
          >
            Split
          </button>
        </div>
      </div>
      
      <div className="editor-content">
        <div className="editor-with-ai">
          <MDEditor
            ref={editorRef}
            value={value}
            onChange={handleEditorChange}
            preview={previewMode}
            hideToolbar={false}
            visibleDragBar={false}
            data-color-mode="light"
            height={500}
            textareaProps={{
              placeholder,
              style: {
                fontSize: 14,
                lineHeight: 1.5,
                fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace"
              },
              onSelect: (e) => setCursorPosition(e.target.selectionStart),
              onKeyUp: (e) => setCursorPosition(e.target.selectionStart)
            }}
          />
          
          {showAISuggestions && (
            <AISuggestions
              content={value}
              cursorPosition={cursorPosition}
              onSuggestionSelect={handleSuggestionSelect}
              onTitleSuggestion={onTitleSuggestion}
              onTagSuggestions={onTagSuggestions}
              isVisible={value && value.length > 10}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default MarkdownEditor;