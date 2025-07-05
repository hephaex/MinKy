import React, { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { documentService } from '../services/api';
import MarkdownEditor from '../components/MarkdownEditor';
import './DocumentForm.css';

const DocumentEdit = () => {
  const { id } = useParams();
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    title: '',
    author: '',
    markdown_content: ''
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(null);
  const [originalDocument, setOriginalDocument] = useState(null);

  useEffect(() => {
    const fetchDocument = async () => {
      try {
        setLoading(true);
        const document = await documentService.getDocument(id);
        setOriginalDocument(document);
        setFormData({
          title: document.title,
          author: document.author || '',
          markdown_content: document.markdown_content
        });
        setError(null);
      } catch (err) {
        setError('Failed to fetch document');
        console.error('Error fetching document:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchDocument();
  }, [id]);

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

  const hasChanges = () => {
    if (!originalDocument) return false;
    return (
      formData.title !== originalDocument.title ||
      formData.author !== (originalDocument.author || '') ||
      formData.markdown_content !== originalDocument.markdown_content
    );
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    
    if (!formData.title.trim() || !formData.markdown_content.trim()) {
      setError('Title and content are required');
      return;
    }

    try {
      setSaving(true);
      setError(null);
      
      const updatedDocument = await documentService.updateDocument(id, {
        title: formData.title.trim(),
        author: formData.author.trim() || null,
        markdown_content: formData.markdown_content.trim()
      });
      
      navigate(`/documents/${updatedDocument.id}`);
    } catch (err) {
      setError('Failed to update document');
      console.error('Error updating document:', err);
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    if (hasChanges()) {
      if (window.confirm('You have unsaved changes. Are you sure you want to leave?')) {
        navigate(`/documents/${id}`);
      }
    } else {
      navigate(`/documents/${id}`);
    }
  };

  if (loading) {
    return <div className="loading">Loading document...</div>;
  }

  if (error && !originalDocument) {
    return (
      <div className="error">
        {error}
        <button className="btn btn-secondary" onClick={() => navigate('/')}>
          Back to Documents
        </button>
      </div>
    );
  }

  return (
    <div className="document-form">
      <div className="document-form-header">
        <h2>Edit Document</h2>
        <div className="form-actions">
          <button 
            type="button" 
            className="btn btn-secondary" 
            onClick={handleCancel}
            disabled={saving}
          >
            Cancel
          </button>
          <button 
            type="submit" 
            form="document-form"
            className="btn btn-primary" 
            disabled={saving || !formData.title.trim() || !formData.markdown_content.trim() || !hasChanges()}
          >
            {saving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>

      {error && <div className="error">{error}</div>}

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
        </div>

        <div className="form-group">
          <label>Content *</label>
          <MarkdownEditor
            value={formData.markdown_content}
            onChange={handleMarkdownChange}
            placeholder="Start writing your markdown content..."
          />
        </div>
      </form>
    </div>
  );
};

export default DocumentEdit;