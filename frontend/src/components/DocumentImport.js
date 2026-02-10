import React, { useState, useRef } from 'react';
import api from '../services/api';
import { logError } from '../utils/logger';
import './DocumentImport.css';

const DocumentImport = ({ 
  onDocumentImported, 
  acceptedFileTypes = null, 
  fileExtensions = null,
  title = "Document Import",
  description = "Upload documents to convert them to Markdown format"
}) => {
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
    'text/markdown': '.md',
    'text/csv': '.csv',
    'application/json': '.json',
    'application/xml': '.xml',
    'text/xml': '.xml',
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
    // If specific file types are provided, use those for validation
    if (acceptedFileTypes || fileExtensions) {
      const typeMatches = acceptedFileTypes ? acceptedFileTypes.includes(file.type) : true;
      const extensionMatches = fileExtensions ? 
        fileExtensions.some(ext => file.name.toLowerCase().endsWith(ext.toLowerCase())) : true;
      return typeMatches && extensionMatches;
    }
    
    // Exclude image files - they should use OCR instead
    if (file.type && file.type.startsWith('image/')) {
      return false;
    }
    
    // Otherwise, use the default supported types
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
        if (file.type && file.type.startsWith('image/')) {
          newErrors.push(`${file.name}: Image files should use the OCR Text Extraction tab instead of Document Conversion`);
        } else {
          newErrors.push(`${file.name}: Unsupported file type (${file.type || 'unknown'})`);
        }
        continue;
      }

      try {
        const formData = new FormData();
        formData.append('file', file);
        formData.append('auto_tag', 'true');

        const response = await api.post('/documents/import', formData, {
          timeout: 120000, // 2 minutes timeout for large files
        });
        
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
        logError('DocumentImport', error, {
          status: error.response?.status,
          data: error.response?.data
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
              <h3>{title}</h3>
              <p>
                {description}
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