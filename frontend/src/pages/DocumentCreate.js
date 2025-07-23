import React, { useState, useEffect } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { documentService } from '../services/api';
import api from '../services/api';
import MarkdownEditor from '../components/MarkdownEditor';
import OCRUpload from '../components/OCRUpload';
import TagInput from '../components/TagInput';
import './DocumentForm.css';

const DocumentCreate = () => {
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    title: '',
    author: '',
    markdown_content: '',
    category_id: null,
    tags: []
  });
  const [categories, setCategories] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [showOCR, setShowOCR] = useState(false);
  const [suggestedTags, setSuggestedTags] = useState([]);

  useEffect(() => {
    fetchCategories();
  }, []);

  const fetchCategories = async () => {
    try {
      const response = await api.get('/categories?format=flat');
      setCategories(response.data.categories);
    } catch (err) {
      console.error('Error fetching categories:', err);
    }
  };

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

  const handleTagSuggestions = (suggestedTagsList) => {
    console.log('Suggested tags:', suggestedTagsList);
    
    // Auto-apply AI suggested tags by merging with existing tags
    if (suggestedTagsList && suggestedTagsList.length > 0) {
      setFormData(prev => {
        const currentTags = prev.tags || [];
        const newTags = [...currentTags];
        
        // Add suggested tags that aren't already present
        suggestedTagsList.forEach(suggestedTag => {
          const normalizedSuggested = suggestedTag.toLowerCase().trim();
          const exists = newTags.some(existingTag => 
            existingTag.toLowerCase().trim() === normalizedSuggested
          );
          
          if (!exists) {
            newTags.push(suggestedTag);
          }
        });
        
        return {
          ...prev,
          tags: newTags
        };
      });
      
      // Also set suggested tags for display (user can still see what was added)
      setSuggestedTags(suggestedTagsList);
    }
  };

  const handleTagsChange = (newTags) => {
    setFormData(prev => ({
      ...prev,
      tags: newTags
    }));
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
        tags: formData.tags
      });
      
      navigate(`/documents/${document.id}`);
    } catch (err) {
      setError('Failed to create document');
      console.error('Error creating document:', err);
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
            tags={formData.tags}
            onChange={handleTagsChange}
            suggestedTags={suggestedTags}
            onSuggestionApply={() => setSuggestedTags([])}
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