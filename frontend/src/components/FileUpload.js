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
      if (response && response.document) {
        onUploadSuccess?.(response);
        return true;
      } else {
        onUploadError?.(`${file.name}: Invalid response format`);
        return false;
      }
    } catch (error) {
      console.error('Upload error:', error);
      
      // Check if it's a successful response that threw an error due to response format
      if (error.response?.status === 201 && error.response?.data?.document) {
        onUploadSuccess?.(error.response.data);
        return true;
      }
      
      // Log detailed error info for debugging
      console.error('Upload error details:', {
        status: error.response?.status,
        data: error.response?.data,
        headers: error.response?.headers,
        config: error.config
      });
      
      // Log the actual error data content
      console.error('Error data:', error.response?.data);
      console.error('Error message from backend:', error.response?.data?.error);
      
      // Show detailed error for debugging
      const errorData = error.response?.data;
      if (errorData) {
        console.error('Full error object:', JSON.stringify(errorData, null, 2));
        console.error('=== UPLOAD ERROR MESSAGE ===');
        console.error('Status:', error.response?.status);
        console.error('Error:', errorData.error || 'No error message');
        console.error('Full data:', errorData);
        console.error('=== END ERROR MESSAGE ===');
        
        // Create a visible error div
        const errorDiv = document.createElement('div');
        errorDiv.style.cssText = 'position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: red; color: white; padding: 20px; z-index: 9999; border-radius: 5px; max-width: 80%; word-wrap: break-word;';
        errorDiv.innerHTML = `<strong>Upload Error ${error.response?.status}:</strong><br>${JSON.stringify(errorData, null, 2)}`;
        document.body.appendChild(errorDiv);
        
        setTimeout(() => {
          document.body.removeChild(errorDiv);
        }, 10000);
      }
      
      const errorMessage = error.response?.data?.error || error.message || 'Upload failed. Please try again.';
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