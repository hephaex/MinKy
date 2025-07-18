import React, { useState, useEffect } from 'react';
import api from '../services/api';
import './CategoryManager.css';

const CategoryManager = () => {
  const [categories, setCategories] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingCategory, setEditingCategory] = useState(null);
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    parent_id: null,
    color: '#007bff'
  });

  useEffect(() => {
    fetchCategories();
  }, []);

  const fetchCategories = async () => {
    try {
      setLoading(true);
      const response = await api.get('/categories?format=tree');
      setCategories(response.data.tree);
      setError(null);
    } catch (err) {
      setError('Failed to load categories');
      console.error('Error fetching categories:', err);
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async (e) => {
    e.preventDefault();
    try {
      if (editingCategory) {
        await api.put(`/categories/${editingCategory.id}`, formData);
      } else {
        await api.post('/categories', formData);
      }
      
      setFormData({ name: '', description: '', parent_id: null, color: '#007bff' });
      setEditingCategory(null);
      setShowCreateForm(false);
      await fetchCategories();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to save category');
    }
  };

  const handleEdit = (category) => {
    setEditingCategory(category);
    setFormData({
      name: category.name,
      description: category.description || '',
      parent_id: category.parent_id,
      color: category.color || '#007bff'
    });
    setShowCreateForm(true);
  };

  const handleDelete = async (categoryId) => {
    if (!window.confirm('Are you sure you want to delete this category?')) {
      return;
    }

    try {
      await api.delete(`/categories/${categoryId}`);
      await fetchCategories();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to delete category');
    }
  };

  const handleCancel = () => {
    setFormData({ name: '', description: '', parent_id: null, color: '#007bff' });
    setEditingCategory(null);
    setShowCreateForm(false);
  };

  const renderCategoryTree = (nodes, level = 0) => {
    return nodes.map(node => (
      <div key={node.category.id} className="category-item" style={{ marginLeft: `${level * 20}px` }}>
        <div className="category-content">
          <div className="category-info">
            <div className="category-header">
              <span 
                className="category-color" 
                style={{ backgroundColor: node.category.color }}
              ></span>
              <h3>{node.category.name}</h3>
              <span className="category-count">
                ({node.category.document_count} documents)
              </span>
            </div>
            {node.category.description && (
              <p className="category-description">{node.category.description}</p>
            )}
          </div>
          <div className="category-actions">
            <button 
              className="btn btn-sm btn-secondary"
              onClick={() => handleEdit(node.category)}
            >
              Edit
            </button>
            <button 
              className="btn btn-sm btn-danger"
              onClick={() => handleDelete(node.category.id)}
            >
              Delete
            </button>
          </div>
        </div>
        {node.children && node.children.length > 0 && (
          <div className="category-children">
            {renderCategoryTree(node.children, level + 1)}
          </div>
        )}
      </div>
    ));
  };

  const renderParentOptions = (nodes, level = 0) => {
    let options = [];
    
    nodes.forEach(node => {
      // Don't show the category being edited as a parent option
      if (editingCategory && node.category.id === editingCategory.id) {
        return;
      }
      
      options.push(
        <option key={node.category.id} value={node.category.id}>
          {'—'.repeat(level)} {node.category.name}
        </option>
      );
      
      if (node.children && node.children.length > 0) {
        options = options.concat(renderParentOptions(node.children, level + 1));
      }
    });
    
    return options;
  };

  if (loading) {
    return <div className="category-manager loading">Loading categories...</div>;
  }

  return (
    <div className="category-manager">
      <div className="category-header">
        <h1>Category Management</h1>
        <button 
          className="btn btn-primary"
          onClick={() => setShowCreateForm(true)}
        >
          Create New Category
        </button>
      </div>

      {error && (
        <div className="alert alert-danger">
          {error}
          <button onClick={() => setError(null)} className="alert-close">×</button>
        </div>
      )}

      {showCreateForm && (
        <div className="category-form-modal">
          <div className="category-form-overlay" onClick={handleCancel}></div>
          <div className="category-form">
            <h2>{editingCategory ? 'Edit Category' : 'Create New Category'}</h2>
            <form onSubmit={handleSubmit}>
              <div className="form-group">
                <label htmlFor="name">Category Name *</label>
                <input
                  id="name"
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({...formData, name: e.target.value})}
                  required
                />
              </div>

              <div className="form-group">
                <label htmlFor="description">Description</label>
                <textarea
                  id="description"
                  value={formData.description}
                  onChange={(e) => setFormData({...formData, description: e.target.value})}
                  rows="3"
                />
              </div>

              <div className="form-group">
                <label htmlFor="parent_id">Parent Category</label>
                <select
                  id="parent_id"
                  value={formData.parent_id || ''}
                  onChange={(e) => setFormData({...formData, parent_id: e.target.value || null})}
                >
                  <option value="">None (Root Category)</option>
                  {renderParentOptions(categories)}
                </select>
              </div>

              <div className="form-group">
                <label htmlFor="color">Color</label>
                <input
                  id="color"
                  type="color"
                  value={formData.color}
                  onChange={(e) => setFormData({...formData, color: e.target.value})}
                />
              </div>

              <div className="form-actions">
                <button type="button" className="btn btn-secondary" onClick={handleCancel}>
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary">
                  {editingCategory ? 'Update' : 'Create'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      <div className="category-tree">
        {categories.length > 0 ? (
          renderCategoryTree(categories)
        ) : (
          <div className="no-categories">
            <p>No categories found. Create your first category to get started!</p>
          </div>
        )}
      </div>
    </div>
  );
};

export default CategoryManager;