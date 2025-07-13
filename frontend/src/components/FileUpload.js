import React, { useState } from 'react';
import { documentService } from '../services/api';
import './FileUpload.css';

const FileUpload = ({ onUploadSuccess, onUploadError }) => {
  const [uploading, setUploading] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [uploadProgress, setUploadProgress] = useState({ current: 0, total: 0 });

  const handleFileUpload = async (file) => {
    if (!file) return;

    // Validate file type
    if (!file.name.toLowerCase().endsWith('.md')) {
      onUploadError?.(`File "${file.name}" is not a markdown file`);
      return false;
    }

    // Validate file size (10MB max)
    if (file.size > 10 * 1024 * 1024) {
      onUploadError?.(`File "${file.name}" is too large (max 10MB)`);
      return false;
    }

    try {
      const response = await documentService.uploadDocument(file);
      onUploadSuccess?.(response);
      return true;
    } catch (error) {
      const errorMessage = error.response?.data?.error || 'Upload failed. Please try again.';
      onUploadError?.(`${file.name}: ${errorMessage}`);
      return false;
    }
  };

  const handleMultipleFileUpload = async (files) => {
    if (!files || files.length === 0) return;

    const fileArray = Array.from(files);
    const mdFiles = fileArray.filter(file => file.name.toLowerCase().endsWith('.md'));
    
    if (mdFiles.length === 0) {
      onUploadError?.('No markdown files found');
      return;
    }

    if (mdFiles.length !== fileArray.length) {
      onUploadError?.(`Only ${mdFiles.length} of ${fileArray.length} files are markdown files`);
    }

    setUploading(true);
    setUploadProgress({ current: 0, total: mdFiles.length });

    let successCount = 0;
    let failCount = 0;

    for (let i = 0; i < mdFiles.length; i++) {
      setUploadProgress({ current: i + 1, total: mdFiles.length });
      
      const success = await handleFileUpload(mdFiles[i]);
      if (success) {
        successCount++;
      } else {
        failCount++;
      }
      
      // Small delay between uploads to prevent overwhelming the server
      if (i < mdFiles.length - 1) {
        await new Promise(resolve => setTimeout(resolve, 500));
      }
    }

    setUploading(false);
    setUploadProgress({ current: 0, total: 0 });

    if (successCount > 0) {
      onUploadSuccess?.({ 
        message: `Successfully uploaded ${successCount} files${failCount > 0 ? `, ${failCount} failed` : ''}`,
        count: successCount
      });
    }
  };

  const handleFileSelect = (event) => {
    const files = event.target.files;
    if (files && files.length > 0) {
      if (files.length === 1) {
        handleFileUpload(files[0]);
      } else {
        handleMultipleFileUpload(files);
      }
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
    if (files && files.length > 0) {
      if (files.length === 1) {
        handleFileUpload(files[0]);
      } else {
        handleMultipleFileUpload(files);
      }
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
          multiple
          onChange={handleFileSelect}
          disabled={uploading}
          id="file-upload-input"
          className="file-upload-input"
        />
        <label htmlFor="file-upload-input" className="file-upload-label">
          {uploading ? (
            <div className="upload-progress">
              <div className="spinner"></div>
              <span>
                {uploadProgress.total > 1 
                  ? `Uploading ${uploadProgress.current}/${uploadProgress.total} files...`
                  : 'Uploading...'
                }
              </span>
            </div>
          ) : (
            <div className="upload-prompt">
              <div className="upload-icon">📄</div>
              <div className="upload-text">
                <strong>Click to select</strong> or drag and drop your markdown files here
              </div>
              <div className="upload-hint">
                .md files only, max 10MB each, multiple files supported
              </div>
            </div>
          )}
        </label>
      </div>
    </div>
  );
};

export default FileUpload;