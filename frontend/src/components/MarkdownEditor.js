import React, { useState } from 'react';
import MDEditor from '@uiw/react-md-editor';
import './MarkdownEditor.css';

const MarkdownEditor = ({ value, onChange, placeholder = "Start writing your markdown..." }) => {
  const [previewMode, setPreviewMode] = useState('edit');

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
        <MDEditor
          value={value}
          onChange={onChange}
          preview={previewMode}
          hideToolbar={false}
          visibleDragBar={false}
          data-color-mode="light"
          height={400}
          textareaProps={{
            placeholder,
            style: {
              fontSize: 14,
              lineHeight: 1.5,
              fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace"
            }
          }}
        />
      </div>
    </div>
  );
};

export default MarkdownEditor;