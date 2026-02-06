import React from 'react';
import { useNavigate } from 'react-router-dom';
import OCRUpload from '../components/OCRUpload';
import './OCRPage.css';

const OCRPage = () => {
  const navigate = useNavigate();

  const handleDocumentCreated = (document) => {
    // Navigate to the newly created document
    navigate(`/documents/${document.id}`);
  };

  const handleTextExtracted = (result) => {
    // Could show a success message or store extracted text for later use
    // text extracted
  };

  return (
    <div className="ocr-page">
      <div className="page-header">
        <div className="header-content">
          <h1>OCR Text Extraction</h1>
          <p>
            Extract text from images and PDF documents using optical character recognition.
            Create new documents from extracted text or use the text for other purposes.
          </p>
        </div>
        <button 
          className="btn btn-secondary"
          onClick={() => navigate(-1)}
        >
          ← Back
        </button>
      </div>

      <div className="ocr-content">
        <div className="ocr-modes">
          <div className="mode-tabs">
            <div className="tab active">
              📄 Create Document from OCR
            </div>
          </div>

          <div className="tab-content">
            <OCRUpload 
              mode="create"
              onDocumentCreated={handleDocumentCreated}
              onTextExtracted={handleTextExtracted}
            />
          </div>
        </div>

        <div className="ocr-info">
          <div className="info-section">
            <h3>How it works</h3>
            <ol>
              <li>Upload an image (PNG, JPEG, TIFF, BMP, GIF) or PDF file</li>
              <li>Select the appropriate language for better accuracy</li>
              <li>Our OCR engine extracts text from your document</li>
              <li>Review the extracted text and create a new document</li>
            </ol>
          </div>

          <div className="info-section">
            <h3>Tips for better results</h3>
            <ul>
              <li>Use high-resolution images with clear, readable text</li>
              <li>Ensure good contrast between text and background</li>
              <li>Avoid skewed or rotated documents when possible</li>
              <li>Select the correct language for your document</li>
              <li>For PDFs, text-based PDFs will yield better results than scanned images</li>
            </ul>
          </div>

          <div className="info-section">
            <h3>Supported languages</h3>
            <p>
              The OCR service supports multiple languages including English, Korean, Japanese, 
              Chinese (Simplified/Traditional), French, German, Spanish, Italian, Portuguese, 
              Russian, Arabic, Hindi, Thai, and Vietnamese.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default OCRPage;