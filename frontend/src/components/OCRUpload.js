import React, { useState, useCallback } from 'react';
import { authService } from '../services/api';
import './OCRUpload.css';

const OCRUpload = ({ onTextExtracted, onDocumentCreated, mode = 'extract' }) => {
  const [file, setFile] = useState(null);
  const [dragActive, setDragActive] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);
  const [language, setLanguage] = useState('eng');
  const [supportedLanguages, setSupportedLanguages] = useState([]);
  const [ocrStatus, setOcrStatus] = useState(null);
  const [formData, setFormData] = useState({
    title: '',
    author: '',
    is_public: true
  });

  // Load OCR status and supported languages on mount
  React.useEffect(() => {
    const loadOCRInfo = async () => {
      try {
        // Get OCR status
        const statusResponse = await fetch('/api/ocr/status');
        const statusData = await statusResponse.json();
        setOcrStatus(statusData.status);

        // Get supported languages
        const langResponse = await fetch('/api/ocr/languages');
        const langData = await langResponse.json();
        if (langData.success) {
          setSupportedLanguages(langData.languages);
        }
      } catch (err) {
        console.error('Error loading OCR info:', err);
      }
    };

    loadOCRInfo();
  }, []);

  const handleDrag = useCallback((e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true);
    } else if (e.type === "dragleave") {
      setDragActive(false);
    }
  }, []);

  const handleDrop = useCallback((e) => {
    e.preventDefault();
    e.stopPropagation();
    setDragActive(false);
    
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      setFile(e.dataTransfer.files[0]);
      setError(null);
      setResult(null);
    }
  }, []);

  const handleFileChange = (e) => {
    if (e.target.files && e.target.files[0]) {
      setFile(e.target.files[0]);
      setError(null);
      setResult(null);
    }
  };

  const handleFormChange = (e) => {
    const { name, value, type, checked } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value
    }));
  };

  const extractText = async () => {
    if (!file) {
      setError('Please select a file first');
      return;
    }

    setProcessing(true);
    setError(null);
    setResult(null);

    try {
      const formDataObj = new FormData();
      formDataObj.append('file', file);
      formDataObj.append('language', language);

      const token = authService.getToken();
      const headers = {};
      if (token && token !== 'null' && token !== 'undefined') {
        headers['Authorization'] = `Bearer ${token}`;
      }

      const response = await fetch('/api/ocr/extract', {
        method: 'POST',
        headers,
        body: formDataObj
      });

      const data = await response.json();

      if (data.success) {
        setResult(data);
        if (onTextExtracted) {
          onTextExtracted(data);
        }
      } else {
        const errorMessage = data.error || 'OCR extraction failed';
        if (errorMessage.toLowerCase().includes('service unavailable') || 
            errorMessage.toLowerCase().includes('not available') ||
            errorMessage.toLowerCase().includes('unavailable')) {
          setError('OCR Service is currently unavailable. Please check that Tesseract is installed or cloud OCR services are configured.');
        } else {
          setError(errorMessage);
        }
      }
    } catch (err) {
      if (err.message.includes('Failed to fetch') || err.message.includes('Network Error')) {
        setError('Unable to connect to OCR service. Please ensure the server is running and try again.');
      } else {
        setError('Error during OCR processing: ' + err.message);
      }
      console.error('OCR error:', err);
    } finally {
      setProcessing(false);
    }
  };

  const createDocument = async () => {
    if (!file) {
      setError('Please select a file first');
      return;
    }

    setProcessing(true);
    setError(null);
    setResult(null);

    try {
      const formDataObj = new FormData();
      formDataObj.append('file', file);
      formDataObj.append('language', language);
      formDataObj.append('title', formData.title);
      formDataObj.append('author', formData.author);
      formDataObj.append('is_public', formData.is_public.toString());

      const token = authService.getToken();
      const headers = {};
      if (token && token !== 'null' && token !== 'undefined') {
        headers['Authorization'] = `Bearer ${token}`;
      }
      
      const response = await fetch('/api/ocr/extract-and-create', {
        method: 'POST',
        headers,
        body: formDataObj
      });

      const data = await response.json();

      if (data.success) {
        setResult(data);
        if (onDocumentCreated) {
          onDocumentCreated(data.document);
        }
      } else {
        setError(data.error || 'Document creation failed');
      }
    } catch (err) {
      setError('Error creating document: ' + err.message);
      console.error('Document creation error:', err);
    } finally {
      setProcessing(false);
    }
  };

  const isValidFileType = (file) => {
    if (!file) return false;
    const allowedTypes = [
      'application/pdf',
      'image/png',
      'image/jpeg',
      'image/jpg',
      'image/tiff',
      'image/bmp',
      'image/gif'
    ];
    return allowedTypes.includes(file.type) || /\.(pdf|png|jpe?g|tiff?|bmp|gif)$/i.test(file.name);
  };

  if (!ocrStatus) {
    return (
      <div className="ocr-upload">
        <div className="loading">Loading OCR capabilities...</div>
      </div>
    );
  }

  if (!ocrStatus.available) {
    return (
      <div className="ocr-upload">
        <div className="ocr-unavailable">
          <h3>OCR Service Unavailable</h3>
          <p>
            OCR functionality is not available. Please ensure Tesseract is installed 
            or cloud OCR services are configured.
          </p>
          <div className="ocr-status">
            <div className="status-item">
              <span className="label">Tesseract:</span>
              <span className={`status ${ocrStatus.tesseract ? 'available' : 'unavailable'}`}>
                {ocrStatus.tesseract ? 'Available' : 'Not Available'}
              </span>
            </div>
            <div className="status-item">
              <span className="label">Cloud OCR:</span>
              <span className={`status ${ocrStatus.cloud_ocr ? 'available' : 'unavailable'}`}>
                {ocrStatus.cloud_ocr ? 'Available' : 'Not Available'}
              </span>
            </div>
            <div className="status-item">
              <span className="label">PDF Tools:</span>
              <span className={`status ${ocrStatus.pdf_tools ? 'available' : 'unavailable'}`}>
                {ocrStatus.pdf_tools ? 'Available' : 'Not Available'}
              </span>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="ocr-upload">
      <div className="ocr-header">
        <h3>OCR Text Extraction</h3>
        <p>Upload images or PDFs to extract text using optical character recognition</p>
      </div>

      {/* OCR Status Info */}
      <div className="ocr-capabilities">
        <div className="capability-item">
          <span className="capability-label">Supported formats:</span>
          <span className="capability-value">
            {ocrStatus.supported_formats.join(', ')}
          </span>
        </div>
        <div className="capability-item">
          <span className="capability-label">Max file size:</span>
          <span className="capability-value">{ocrStatus.max_file_size}</span>
        </div>
        <div className="capability-item">
          <span className="capability-label">Available features:</span>
          <span className="capability-value">
            {Object.entries(ocrStatus.features)
              .filter(([, enabled]) => enabled)
              .map(([feature]) => feature.replace(/_/g, ' '))
              .join(', ')}
          </span>
        </div>
      </div>

      {/* File Upload Area */}
      <div
        className={`file-upload-area ${dragActive ? 'drag-active' : ''} ${!isValidFileType(file) && file ? 'invalid-file' : ''}`}
        onDragEnter={handleDrag}
        onDragLeave={handleDrag}
        onDragOver={handleDrag}
        onDrop={handleDrop}
      >
        <input
          type="file"
          id="ocr-file-input"
          onChange={handleFileChange}
          accept=".pdf,.png,.jpg,.jpeg,.tiff,.bmp,.gif"
          style={{ display: 'none' }}
        />
        
        {file ? (
          <div className="file-selected">
            <div className="file-info">
              <span className="file-name">{file.name}</span>
              <span className="file-size">
                ({(file.size / 1024 / 1024).toFixed(2)} MB)
              </span>
            </div>
            {!isValidFileType(file) && (
              <div className="file-error">
                Unsupported file type. Please select a PDF or image file.
              </div>
            )}
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              onClick={() => setFile(null)}
            >
              Remove
            </button>
          </div>
        ) : (
          <div className="upload-prompt">
            <div className="upload-icon">📄</div>
            <p>Drag and drop a file here, or</p>
            <label htmlFor="ocr-file-input" className="btn btn-secondary">
              Choose File
            </label>
          </div>
        )}
      </div>

      {/* Language Selection */}
      {supportedLanguages.length > 0 && (
        <div className="language-selection">
          <label htmlFor="ocr-language">Language:</label>
          <select
            id="ocr-language"
            value={language}
            onChange={(e) => setLanguage(e.target.value)}
            className="form-control"
          >
            {supportedLanguages.map(lang => (
              <option key={lang.code} value={lang.code}>
                {lang.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Document Creation Form (for create mode) */}
      {mode === 'create' && (
        <div className="document-form-fields">
          <div className="form-group">
            <label htmlFor="doc-title">Document Title:</label>
            <input
              type="text"
              id="doc-title"
              name="title"
              value={formData.title}
              onChange={handleFormChange}
              placeholder="Auto-generated from filename if empty"
              className="form-control"
            />
          </div>
          
          <div className="form-group">
            <label htmlFor="doc-author">Author:</label>
            <input
              type="text"
              id="doc-author"
              name="author"
              value={formData.author}
              onChange={handleFormChange}
              placeholder="Optional"
              className="form-control"
            />
          </div>
          
          <div className="form-group checkbox-group">
            <label>
              <input
                type="checkbox"
                name="is_public"
                checked={formData.is_public}
                onChange={handleFormChange}
              />
              Make document public
            </label>
          </div>
        </div>
      )}

      {/* Action Buttons */}
      <div className="ocr-actions">
        {mode === 'extract' ? (
          <button
            onClick={extractText}
            disabled={!file || !isValidFileType(file) || processing}
            className="btn btn-primary"
          >
            {processing ? 'Extracting Text...' : 'Extract Text'}
          </button>
        ) : (
          <button
            onClick={createDocument}
            disabled={!file || !isValidFileType(file) || processing}
            className="btn btn-primary"
          >
            {processing ? 'Creating Document...' : 'Create Document from OCR'}
          </button>
        )}
      </div>

      {/* Error Display */}
      {error && (
        <div className="ocr-error">
          <strong>Error:</strong> {error}
        </div>
      )}

      {/* Results Display */}
      {result && (
        <div className="ocr-result">
          <div className="result-header">
            <h4>
              {mode === 'create' ? 'Document Created' : 'Text Extracted'} Successfully
            </h4>
            <div className="result-meta">
              <span className="meta-item">
                Method: <strong>{result.ocr_result?.method || result.method}</strong>
              </span>
              <span className="meta-item">
                Confidence: <strong>{result.ocr_result?.confidence || result.confidence}%</strong>
              </span>
              <span className="meta-item">
                Words: <strong>{result.ocr_result?.word_count || result.word_count}</strong>
              </span>
            </div>
          </div>
          
          {mode === 'create' && result.document && (
            <div className="document-info">
              <p>
                <strong>Document created:</strong>{' '}
                <a href={`/documents/${result.document.id}`} target="_blank" rel="noopener noreferrer">
                  {result.document.title}
                </a>
              </p>
            </div>
          )}
          
          <div className="extracted-text">
            <div className="text-header">
              <strong>Extracted Text:</strong>
              <button
                onClick={() => navigator.clipboard.writeText(result.ocr_result?.text || result.text)}
                className="btn btn-sm btn-secondary"
                title="Copy to clipboard"
              >
                📋 Copy
              </button>
            </div>
            <pre className="text-content">
              {result.ocr_result?.text || result.text}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
};

export default OCRUpload;