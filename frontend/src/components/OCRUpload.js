import { useState, useCallback, useEffect } from 'react';
import PropTypes from 'prop-types';
import api from '../services/api';
import {
  OCRDropzone,
  OCRStatusLoading,
  OCRStatusUnavailable,
  OCRCapabilities,
  OCRResult,
  OCRDocumentForm,
} from './ocr';
import './OCRUpload.css';

const OCRUpload = ({
  onTextExtracted = () => {},
  onDocumentCreated = () => {},
  mode = 'extract',
}) => {
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
    is_public: true,
  });

  useEffect(() => {
    const loadOCRInfo = async () => {
      try {
        const statusResponse = await api.get('/ocr/status');
        setOcrStatus(statusResponse.data.status);

        const langResponse = await api.get('/ocr/languages');
        if (langResponse.data.success) {
          setSupportedLanguages(langResponse.data.languages);
        }
      } catch (err) {
        // OCR info loading failed silently
      }
    };

    loadOCRInfo();
  }, []);

  const handleDrag = useCallback((e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.type === 'dragenter' || e.type === 'dragover') {
      setDragActive(true);
    } else if (e.type === 'dragleave') {
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

  const handleFileChange = (newFile) => {
    setFile(newFile);
    setError(null);
    setResult(null);
  };

  const handleFileRemove = () => {
    setFile(null);
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
      'image/gif',
    ];
    return allowedTypes.includes(file.type) || /\.(pdf|png|jpe?g|tiff?|bmp|gif)$/i.test(file.name);
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

      const response = await api.post('/ocr/extract', formDataObj);
      const data = response.data;

      if (data.success) {
        setResult(data);
        if (onTextExtracted) {
          onTextExtracted(data);
        }
      } else {
        handleOCRError(data.error);
      }
    } catch (err) {
      // axios throws on HTTP error statuses; surface the server's message when present
      const serverError = err.response?.data?.error;
      if (serverError) {
        handleOCRError(serverError);
      } else {
        handleNetworkError(err);
      }
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

      const response = await api.post('/ocr/extract-and-create', formDataObj);
      const data = response.data;

      if (data.success) {
        setResult(data);
        if (onDocumentCreated) {
          onDocumentCreated(data.document);
        }
      } else {
        setError(data.error || 'Document creation failed');
      }
    } catch (err) {
      setError(err.response?.data?.error || 'Error creating document: ' + err.message);
    } finally {
      setProcessing(false);
    }
  };

  const handleOCRError = (errorMessage) => {
    const msg = errorMessage || 'OCR extraction failed';
    if (
      msg.toLowerCase().includes('service unavailable') ||
      msg.toLowerCase().includes('not available') ||
      msg.toLowerCase().includes('unavailable')
    ) {
      setError(
        'OCR Service is currently unavailable. Please check that Tesseract is installed or cloud OCR services are configured.'
      );
    } else {
      setError(msg);
    }
  };

  const handleNetworkError = (err) => {
    if (err.message.includes('Failed to fetch') || err.message.includes('Network Error')) {
      setError(
        'Unable to connect to OCR service. Please ensure the server is running and try again.'
      );
    } else {
      setError('Error during OCR processing: ' + err.message);
    }
  };

  if (!ocrStatus) {
    return <OCRStatusLoading />;
  }

  if (!ocrStatus.available) {
    return <OCRStatusUnavailable ocrStatus={ocrStatus} />;
  }

  return (
    <div className="ocr-upload">
      <div className="ocr-header">
        <h3>OCR Text Extraction</h3>
        <p>Upload images or PDFs to extract text using optical character recognition</p>
      </div>

      <OCRCapabilities ocrStatus={ocrStatus} />

      <OCRDropzone
        file={file}
        onFileChange={handleFileChange}
        onFileRemove={handleFileRemove}
        dragActive={dragActive}
        onDrag={handleDrag}
        onDrop={handleDrop}
      />

      {supportedLanguages.length > 0 && (
        <div className="language-selection">
          <label htmlFor="ocr-language">Language:</label>
          <select
            id="ocr-language"
            value={language}
            onChange={(e) => setLanguage(e.target.value)}
            className="form-control"
          >
            {supportedLanguages.map((lang) => (
              <option key={lang.code} value={lang.code}>
                {lang.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {mode === 'create' && <OCRDocumentForm formData={formData} onChange={setFormData} />}

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

      {error && (
        <div className="ocr-error">
          <strong>Error:</strong> {error}
        </div>
      )}

      {result && <OCRResult result={result} mode={mode} />}
    </div>
  );
};

OCRUpload.propTypes = {
  onTextExtracted: PropTypes.func,
  onDocumentCreated: PropTypes.func,
  mode: PropTypes.oneOf(['extract', 'create']),
};

export default OCRUpload;
