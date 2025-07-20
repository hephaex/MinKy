import React, { useState, useRef } from 'react';
import api from '../services/api';
import './DocumentImport.css';

const DocumentImport = ({ onDocumentImported }) => {
  const [files, setFiles] = useState([]);
  const [uploading, setUploading] = useState(false);
  const [results, setResults] = useState([]);
  const [errors, setErrors] = useState([]);
  const fileInputRef = useRef(null);

  const supportedTypes = {
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document': '.docx',
    'application/vnd.openxmlformats-officedocument.presentationml.presentation': '.pptx',
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': '.xlsx',
    'application/pdf': '.pdf',
    'text/html': '.html',
    'text/plain': '.txt',
    'text/csv': '.csv',
    'application/json': '.json',
    'application/xml': '.xml',
    'text/xml': '.xml',
    'image/png': '.png',
    'image/jpeg': '.jpg',
    'image/jpg': '.jpg',
    'application/zip': '.zip'
  };

  const handleFileSelect = (event) => {
    const selectedFiles = Array.from(event.target.files);
    setFiles(selectedFiles);
    setResults([]);
    setErrors([]);
  };

  const handleDrop = (event) => {
    event.preventDefault();
    const droppedFiles = Array.from(event.dataTransfer.files);
    setFiles(droppedFiles);
    setResults([]);
    setErrors([]);
  };

  const handleDragOver = (event) => {
    event.preventDefault();
  };

  const isFileSupported = (file) => {
    return Object.keys(supportedTypes).includes(file.type) || 
           Object.values(supportedTypes).some(ext => file.name.toLowerCase().endsWith(ext));
  };

  const handleImport = async () => {
    if (files.length === 0) return;

    setUploading(true);
    setResults([]);
    setErrors([]);

    const newResults = [];
    const newErrors = [];

    for (const file of files) {
      if (!isFileSupported(file)) {
        newErrors.push(`${file.name}: Unsupported file type`);
        continue;
      }

      try {
        console.log('DocumentImport: Starting upload for file:', file.name, 'size:', file.size);
        const formData = new FormData();
        formData.append('file', file);
        formData.append('auto_tag', 'true');

        console.log('DocumentImport: FormData prepared, making API call to /documents/import');
        const response = await api.post('/documents/import', formData, {
          timeout: 120000, // 2 minutes timeout for large files
        });
        
        console.log('DocumentImport: API response received:', response.status, response.data);

        if (response.data.success) {
          newResults.push({
            filename: file.name,
            document: response.data.document,
            tags: response.data.tags || [],
            message: response.data.message
          });
        } else {
          newErrors.push(`${file.name}: ${response.data.error || 'Import failed'}`);
        }
      } catch (error) {
        console.error('Import error details:', {
          message: error.message,
          status: error.response?.status,
          statusText: error.response?.statusText,
          data: error.response?.data,
          config: {
            url: error.config?.url,
            method: error.config?.method,
            baseURL: error.config?.baseURL
          }
        });
        newErrors.push(`${file.name}: ${error.response?.data?.error || error.message || 'Import failed'}`);
      }
    }

    setResults(newResults);
    setErrors(newErrors);
    setUploading(false);

    // Call callback with successful imports
    if (newResults.length > 0 && onDocumentImported) {
      onDocumentImported(newResults.map(r => r.document));
    }
  };

  const clearFiles = () => {
    setFiles([]);
    setResults([]);
    setErrors([]);
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  const getFileIcon = (file) => {
    if (file.type.startsWith('image/')) return '🖼️';
    if (file.type === 'application/pdf') return '📄';
    if (file.type.includes('word')) return '📝';
    if (file.type.includes('presentation')) return '📊';
    if (file.type.includes('spreadsheet')) return '📈';
    if (file.type.includes('html')) return '🌐';
    if (file.type.includes('text')) return '📄';
    if (file.type.includes('json')) return '🔧';
    if (file.type.includes('xml')) return '🔧';
    if (file.type.includes('zip')) return '📦';
    return '📄';
  };

  return (
    <div className="document-import">
      <div className="import-section">
        <div className="upload-area">
          <div 
            className="drop-zone"
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            onClick={() => fileInputRef.current?.click()}
          >
            <div className="drop-zone-content">
              <div className="upload-icon">📁</div>
              <h3>Drop files here or click to browse</h3>
              <p>
                Supports Office documents, PDFs, HTML, text files, images, and more
              </p>
              <div className="supported-formats">
                <span>.docx</span>
                <span>.pptx</span>
                <span>.xlsx</span>
                <span>.pdf</span>
                <span>.html</span>
                <span>.txt</span>
                <span>.csv</span>
                <span>.json</span>
                <span>.xml</span>
                <span>.png</span>
                <span>.jpg</span>
                <span>.zip</span>
              </div>
            </div>
          </div>
          
          <input
            ref={fileInputRef}
            type="file"
            multiple
            accept=".docx,.pptx,.xlsx,.pdf,.html,.htm,.txt,.csv,.json,.xml,.png,.jpg,.jpeg,.zip"
            onChange={handleFileSelect}
            style={{ display: 'none' }}
          />
        </div>

        {files.length > 0 && (
          <div className="selected-files">
            <div className="files-header">
              <h4>Selected Files ({files.length})</h4>
              <button className="btn btn-link" onClick={clearFiles}>
                Clear All
              </button>
            </div>
            <div className="files-list">
              {files.map((file, index) => (
                <div key={index} className={`file-item ${!isFileSupported(file) ? 'unsupported' : ''}`}>
                  <span className="file-icon">{getFileIcon(file)}</span>
                  <div className="file-info">
                    <div className="file-name">{file.name}</div>
                    <div className="file-size">{(file.size / 1024 / 1024).toFixed(2)} MB</div>
                  </div>
                  {!isFileSupported(file) && (
                    <span className="unsupported-badge">Unsupported</span>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="import-actions">
          <button 
            className="btn btn-primary"
            onClick={handleImport}
            disabled={files.length === 0 || uploading}
          >
            {uploading ? (
              <>
                <span className="spinner"></span>
                Importing...
              </>
            ) : (
              <>
                📥 Import Documents
              </>
            )}
          </button>
        </div>
      </div>

      {/* Results Section */}
      {(results.length > 0 || errors.length > 0) && (
        <div className="import-results">
          <h4>Import Results</h4>
          
          {results.length > 0 && (
            <div className="success-results">
              <h5>✅ Successfully Imported ({results.length})</h5>
              {results.map((result, index) => (
                <div key={index} className="result-item success">
                  <div className="result-header">
                    <span className="result-filename">{result.filename}</span>
                    <span className="result-status">Success</span>
                  </div>
                  <div className="result-details">
                    <div className="document-title">
                      📄 <strong>{result.document.title}</strong>
                    </div>
                    {result.tags && result.tags.length > 0 && (
                      <div className="auto-tags">
                        <span className="tags-label">Auto-generated tags:</span>
                        {result.tags.map((tag, tagIndex) => (
                          <span key={tagIndex} className="tag">
                            {tag}
                          </span>
                        ))}
                      </div>
                    )}
                    {result.message && (
                      <div className="result-message">{result.message}</div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {errors.length > 0 && (
            <div className="error-results">
              <h5>❌ Failed Imports ({errors.length})</h5>
              {errors.map((error, index) => (
                <div key={index} className="result-item error">
                  {error}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default DocumentImport;