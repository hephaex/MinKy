import React from 'react';

const OCRStatusLoading = () => (
  <div className="ocr-upload">
    <div className="loading">Loading OCR capabilities...</div>
  </div>
);

const OCRStatusUnavailable = ({ ocrStatus }) => (
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

const OCRCapabilities = ({ ocrStatus }) => (
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
);

export { OCRStatusLoading, OCRStatusUnavailable, OCRCapabilities };
