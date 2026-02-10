import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { documentService } from '../services/api';
import MarkdownEditor from '../components/MarkdownEditor';
import OCRUpload from '../components/OCRUpload';
import TagInput from '../components/TagInput';
import useTagSuggestions from '../hooks/useTagSuggestions';
import useCategories from '../hooks/useCategories';
import { logError } from '../utils/logger';
import './DocumentForm.css';

const DocumentCreate = () => {
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    title: '',
    author: '',
    markdown_content: '',
    category_id: null
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [showOCR, setShowOCR] = useState(false);

  // Use custom hooks
  const { categories } = useCategories();
  const {
    tags,
    suggestedTags,
    handleTagSuggestions,
    handleTagsChange,
    clearSuggestedTags
  } = useTagSuggestions();

  const handleInputChange = (e) => {
    const { name, value } = e.target;
    setFormData(prev => ({
      ...prev,
      [name]: value
    }));
  };

  const handleMarkdownChange = (value) => {
    setFormData(prev => ({
      ...prev,
      markdown_content: value || ''
    }));
  };

  const handleTitleSuggestion = (suggestedTitle) => {
    if (suggestedTitle && !formData.title.trim()) {
      setFormData(prev => ({
        ...prev,
        title: suggestedTitle
      }));
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    
    if (!formData.title.trim() || !formData.markdown_content.trim()) {
      setError('Title and content are required');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      
      const document = await documentService.createDocument({
        title: formData.title.trim(),
        author: formData.author.trim() || null,
        markdown_content: formData.markdown_content.trim(),
        category_id: formData.category_id || null,
        tags: tags
      });
      
      navigate(`/documents/${document.id}`);
    } catch (err) {
      setError('Failed to create document');
      logError('DocumentCreate.handleSubmit', err);
    } finally {
      setLoading(false);
    }
  };

  const handleCancel = () => {
    navigate('/');
  };

  const handleOCRTextExtracted = (result) => {
    // Pre-fill form with OCR results
    setFormData(prev => ({
      ...prev,
      markdown_content: result.text || '',
      title: prev.title || `OCR Extract from ${result.filename}`,
    }));
    setShowOCR(false);
  };

  return (
    <div className="document-form">
      <div className="document-form-header">
        <h2>Create New Document</h2>
        <div className="form-actions">
          <button 
            type="button" 
            className="btn btn-outline-primary" 
            onClick={() => setShowOCR(!showOCR)}
            disabled={loading}
            title="Extract text from images or PDFs"
          >
            📄 OCR
          </button>
          <Link 
            to="/ocr" 
            className="btn btn-outline-secondary"
            title="Full OCR page with document creation"
          >
            🔍 Advanced OCR
          </Link>
          <button 
            type="button" 
            className="btn btn-secondary" 
            onClick={handleCancel}
            disabled={loading}
          >
            Cancel
          </button>
          <button 
            type="submit" 
            form="document-form"
            className="btn btn-primary" 
            disabled={loading || !formData.title.trim() || !formData.markdown_content.trim()}
          >
            {loading ? 'Creating...' : 'Create Document'}
          </button>
        </div>
      </div>

      {error && <div className="error">{error}</div>}

      {showOCR && (
        <div className="ocr-section">
          <OCRUpload 
            mode="extract"
            onTextExtracted={handleOCRTextExtracted}
          />
        </div>
      )}

      <form id="document-form" onSubmit={handleSubmit} className="document-form-content">
        <div className="form-row">
          <div className="form-group">
            <label htmlFor="title">Title *</label>
            <input
              type="text"
              id="title"
              name="title"
              className="form-control"
              value={formData.title}
              onChange={handleInputChange}
              placeholder="Enter document title"
              required
            />
          </div>
          
          <div className="form-group">
            <label htmlFor="author">Author</label>
            <input
              type="text"
              id="author"
              name="author"
              className="form-control"
              value={formData.author}
              onChange={handleInputChange}
              placeholder="Enter author name (optional)"
            />
          </div>
          
          <div className="form-group">
            <label htmlFor="category_id">Category</label>
            <select
              id="category_id"
              name="category_id"
              className="form-control"
              value={formData.category_id || ''}
              onChange={handleInputChange}
            >
              <option value="">Select a category (optional)</option>
              {categories.map(category => (
                <option key={category.id} value={category.id}>
                  {category.path}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="form-group">
          <label htmlFor="tags">Tags</label>
          <TagInput
            tags={tags}
            onChange={handleTagsChange}
            suggestedTags={suggestedTags}
            onSuggestionApply={clearSuggestedTags}
          />
        </div>

        <div className="form-group">
          <label>Content *</label>
          <MarkdownEditor
            value={formData.markdown_content}
            onChange={handleMarkdownChange}
            onTitleSuggestion={handleTitleSuggestion}
            onTagSuggestions={handleTagSuggestions}
            placeholder="Start writing your markdown content..."
            showAISuggestions={true}
          />
        </div>
      </form>
    </div>
  );
};

export default DocumentCreate;