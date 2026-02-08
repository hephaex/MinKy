import React, { useCallback } from 'react';

const OCRDropzone = ({ file, onFileChange, onFileRemove, dragActive, onDrag, onDrop }) => {
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

  const handleFileChange = (e) => {
    if (e.target.files && e.target.files[0]) {
      onFileChange(e.target.files[0]);
    }
  };

  return (
    <div
      className={`file-upload-area ${dragActive ? 'drag-active' : ''} ${!isValidFileType(file) && file ? 'invalid-file' : ''}`}
      onDragEnter={onDrag}
      onDragLeave={onDrag}
      onDragOver={onDrag}
      onDrop={onDrop}
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
            onClick={onFileRemove}
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
  );
};

export default OCRDropzone;
