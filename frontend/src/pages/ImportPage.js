import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import OCRUpload from '../components/OCRUpload';
import DocumentImport from '../components/DocumentImport';
import './ImportPage.css';

const ImportPage = () => {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState('ocr');

  const handleDocumentCreated = (document) => {
    // Navigate to the newly created document
    navigate(`/documents/${document.id}`);
  };

  const handleTextExtracted = (result) => {
    // Could show a success message or store extracted text for later use
    console.log('Text extracted:', result);
  };

  const handleDocumentImported = (documents) => {
    // Show success message and optionally navigate to document list
    console.log('Documents imported:', documents);
    if (documents.length === 1) {
      navigate(`/documents/${documents[0].id}`);
    } else {
      navigate('/');
    }
  };

  return (
    <div className="import-page">
      <div className="page-header">
        <div className="header-content">
          <h1>Import Documents</h1>
          <p>
            Import documents from various sources: extract text from images/PDFs using OCR, 
            or convert documents from multiple formats (Office, PDF, HTML, etc.) to Markdown.
          </p>
        </div>
        <button 
          className="btn btn-secondary"
          onClick={() => navigate(-1)}
        >
          ← Back
        </button>
      </div>

      <div className="import-content">
        <div className="import-modes">
          <div className="mode-tabs">
            <div 
              className={`tab ${activeTab === 'ocr' ? 'active' : ''}`}
              onClick={() => setActiveTab('ocr')}
            >
              📷 OCR Text Extraction
            </div>
            <div 
              className={`tab ${activeTab === 'convert' ? 'active' : ''}`}
              onClick={() => setActiveTab('convert')}
            >
              📄 Document Conversion
            </div>
          </div>

          <div className="tab-content">
            {activeTab === 'ocr' && (
              <OCRUpload 
                mode="create"
                onDocumentCreated={handleDocumentCreated}
                onTextExtracted={handleTextExtracted}
              />
            )}
            {activeTab === 'convert' && (
              <DocumentImport 
                onDocumentImported={handleDocumentImported}
              />
            )}
          </div>
        </div>

        <div className="import-info">
          {activeTab === 'ocr' && (
            <>
              <div className="info-section">
                <h3>OCR Text Extraction</h3>
                <ol>
                  <li>Upload an image (PNG, JPEG, TIFF, BMP, GIF) or PDF file</li>
                  <li>Select the appropriate language for better accuracy</li>
                  <li>Our OCR engine extracts text from your document</li>
                  <li>Review the extracted text and create a new document</li>
                </ol>
              </div>

              <div className="info-section">
                <h3>Tips for better OCR results</h3>
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
            </>
          )}

          {activeTab === 'convert' && (
            <>
              <div className="info-section">
                <h3>Document Conversion</h3>
                <ol>
                  <li>Upload a document in any supported format</li>
                  <li>The system automatically detects the file type</li>
                  <li>Document is converted to Markdown format</li>
                  <li>Automatic tags are generated based on content analysis</li>
                  <li>New document is added to your document library</li>
                </ol>
              </div>

              <div className="info-section">
                <h3>Supported file formats</h3>
                <ul>
                  <li><strong>Office Documents:</strong> .docx, .pptx, .xlsx</li>
                  <li><strong>PDF Documents:</strong> .pdf (text extraction and conversion)</li>
                  <li><strong>Web Formats:</strong> .html, .htm</li>
                  <li><strong>Text Formats:</strong> .txt, .csv, .json, .xml</li>
                  <li><strong>Images:</strong> .png, .jpg, .jpeg (with OCR processing)</li>
                  <li><strong>Archives:</strong> .zip (extracts and processes contents)</li>
                </ul>
              </div>

              <div className="info-section">
                <h3>Automatic features</h3>
                <ul>
                  <li>Content-based tag generation using AI analysis</li>
                  <li>Metadata extraction (title, author, creation date)</li>
                  <li>Intelligent document structure preservation</li>
                  <li>Automatic categorization based on content type</li>
                </ul>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

export default ImportPage;