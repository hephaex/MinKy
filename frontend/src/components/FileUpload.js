import React, { useState } from 'react';
import { documentService } from '../services/api';
import './FileUpload.css';

const FileUpload = ({ onUploadSuccess, onUploadError }) => {
  const [uploading, setUploading] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  const handleFileUpload = async (file) => {
    if (!file) return;

    // Validate file type
    if (!file.name.toLowerCase().endsWith('.md')) {
      onUploadError?.('Please select a markdown (.md) file');
      return;
    }

    // Validate file size (10MB max)
    if (file.size > 10 * 1024 * 1024) {
      onUploadError?.('File size must be less than 10MB');
      return;
    }

    setUploading(true);
    try {
      const response = await documentService.uploadDocument(file);
      onUploadSuccess?.(response);
    } catch (error) {
      const errorMessage = error.response?.data?.error || 'Upload failed. Please try again.';
      onUploadError?.(errorMessage);
    } finally {
      setUploading(false);
    }
  };

  const handleFileSelect = (event) => {
    const file = event.target.files[0];
    if (file) {
      handleFileUpload(file);
    }
  };

  const handleDragOver = (e) => {
    e.preventDefault();
    setDragOver(true);
  };

  const handleDragLeave = (e) => {
    e.preventDefault();
    setDragOver(false);
  };

  const handleDrop = (e) => {
    e.preventDefault();
    setDragOver(false);
    
    const files = e.dataTransfer.files;
    if (files.length > 0) {
      handleFileUpload(files[0]);
    }
  };

  return (
    <div className={`file-upload-container ${dragOver ? 'drag-over' : ''}`}>
      <div
        className="file-upload-dropzone"
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        <input
          type="file"
          accept=".md"
          onChange={handleFileSelect}
          disabled={uploading}
          id="file-upload-input"
          className="file-upload-input"
        />
        <label htmlFor="file-upload-input" className="file-upload-label">
          {uploading ? (
            <div className="upload-progress">
              <div className="spinner"></div>
              <span>Uploading...</span>
            </div>
          ) : (
            <div className="upload-prompt">
              <div className="upload-icon">📄</div>
              <div className="upload-text">
                <strong>Click to select</strong> or drag and drop your markdown file here
              </div>
              <div className="upload-hint">
                .md files only, max 10MB
              </div>
            </div>
          )}
        </label>
      </div>
    </div>
  );
};

export default FileUpload;