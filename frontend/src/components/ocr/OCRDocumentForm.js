import React from 'react';

const OCRDocumentForm = ({ formData, onChange }) => {
  const handleChange = (e) => {
    const { name, value, type, checked } = e.target;
    onChange({
      ...formData,
      [name]: type === 'checkbox' ? checked : value
    });
  };

  return (
    <div className="document-form-fields">
      <div className="form-group">
        <label htmlFor="doc-title">Document Title:</label>
        <input
          type="text"
          id="doc-title"
          name="title"
          value={formData.title}
          onChange={handleChange}
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
          onChange={handleChange}
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
            onChange={handleChange}
          />
          Make document public
        </label>
      </div>
    </div>
  );
};

export default OCRDocumentForm;
