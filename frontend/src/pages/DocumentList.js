import React, { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import api from '../services/api';
import SearchBar from '../components/SearchBar';
import Pagination from '../components/Pagination';
import FileUpload from '../components/FileUpload';
import DocumentCard from '../components/DocumentCard';
import useCategories from '../hooks/useCategories';
import useTags from '../hooks/useTags';
import { logError } from '../utils/logger';
import { formatDate } from '../utils/dateUtils';
import './DocumentList.css';

// Sort options configuration
const SORT_OPTIONS = [
  { value: 'updated_desc', label: 'Recently Updated' },
  { value: 'updated_asc', label: 'Oldest Updated' },
  { value: 'created_desc', label: 'Recently Created' },
  { value: 'created_asc', label: 'Oldest Created' },
  { value: 'title_asc', label: 'Title (A-Z)' },
  { value: 'title_desc', label: 'Title (Z-A)' },
];

// View mode options
const VIEW_MODES = {
  GRID: 'grid',
  LIST: 'list',
};

const DocumentList = () => {
  const [documents, setDocuments] = useState([]);
  const [pagination, setPagination] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const [showUpload, setShowUpload] = useState(false);
  const [uploadStatus, setUploadStatus] = useState(null);
  const [selectedCategory, setSelectedCategory] = useState('');
  const [sortBy, setSortBy] = useState('updated_desc');
  const [viewMode, setViewMode] = useState(() => {
    return localStorage.getItem('documentViewMode') || VIEW_MODES.GRID;
  });
  const navigate = useNavigate();

  // Use custom hooks for categories and tags
  const { categories } = useCategories();
  const { tags: availableTags } = useTags({ popular: true });
  const [selectedTags, setSelectedTags] = useState([]);

  const fetchDocuments = async (page = 1, search = '', categoryId = null, sort = sortBy, tags = selectedTags) => {
    try {
      setLoading(true);
      const params = new URLSearchParams();
      params.append('page', page);
      params.append('per_page', 10);
      if (search) params.append('search', search);
      if (categoryId) params.append('category_id', categoryId);
      if (sort) params.append('sort', sort);
      if (tags && tags.length > 0) {
        params.append('tags', tags.join(','));
      }

      const response = await api.get(`/documents?${params.toString()}`);
      setDocuments(response.data.documents);
      setPagination(response.data.pagination);
      setCurrentPage(page);
      setError(null);
    } catch (err) {
      setError('Failed to fetch documents');
      logError('DocumentList.fetchDocuments', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDocuments(1, searchQuery, selectedCategory, sortBy, selectedTags);
  }, [searchQuery, selectedCategory, sortBy, selectedTags]);

  const handleSearch = (query) => {
    setSearchQuery(query);
    setCurrentPage(1);
  };

  const handlePageChange = (page) => {
    fetchDocuments(page, searchQuery, selectedCategory, sortBy, selectedTags);
  };

  const handleTagToggle = (tagSlug) => {
    setSelectedTags(prev => {
      if (prev.includes(tagSlug)) {
        return prev.filter(t => t !== tagSlug);
      }
      return [...prev, tagSlug];
    });
    setCurrentPage(1);
  };

  const clearTagFilters = () => {
    setSelectedTags([]);
    setCurrentPage(1);
  };

  const handleCategoryChange = (categoryId) => {
    setSelectedCategory(categoryId);
    setCurrentPage(1);
  };

  const handleSortChange = (sort) => {
    setSortBy(sort);
    setCurrentPage(1);
  };

  const handleViewModeChange = (mode) => {
    setViewMode(mode);
    localStorage.setItem('documentViewMode', mode);
  };

  const handleUploadSuccess = (response) => {
    if (response.count && response.count > 1) {
      // Multiple files uploaded
      setUploadStatus({
        type: 'success',
        message: response.message
      });
      setShowUpload(false);
      
      // Refresh document list
      fetchDocuments(currentPage, searchQuery);
    } else {
      // Single file uploaded
      setUploadStatus({
        type: 'success',
        message: `File uploaded successfully! Document "${response.document.title}" has been created.`
      });
      setShowUpload(false);
      
      // Refresh document list
      fetchDocuments(currentPage, searchQuery);
      
      // Navigate to the new document after a brief delay
      setTimeout(() => {
        navigate(`/documents/${response.document.id}`);
      }, 1500);
    }
  };

  const handleUploadError = (error) => {
    setUploadStatus({
      type: 'error',
      message: error
    });
  };

  const clearUploadStatus = () => {
    setUploadStatus(null);
  };

  if (loading) {
    return <div className="loading">Loading documents...</div>;
  }

  if (error) {
    return <div className="error">{error}</div>;
  }

  return (
    <div className="document-list">
      <div className="document-list-header">
        <h2>Documents</h2>
        <div className="header-actions">
          <SearchBar onSearch={handleSearch} initialValue={searchQuery} />
          <select
            className="category-filter"
            value={selectedCategory}
            onChange={(e) => handleCategoryChange(e.target.value)}
          >
            <option value="">All Categories</option>
            {categories.map(category => (
              <option key={category.id} value={category.id}>
                {category.path}
              </option>
            ))}
          </select>
          <select
            className="sort-filter"
            value={sortBy}
            onChange={(e) => handleSortChange(e.target.value)}
            aria-label="Sort documents"
          >
            {SORT_OPTIONS.map(option => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <div className="view-toggle">
            <button
              className={`view-toggle-btn${viewMode === VIEW_MODES.GRID ? ' active' : ''}`}
              onClick={() => handleViewModeChange(VIEW_MODES.GRID)}
              aria-label="Grid view"
              title="Grid view"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <rect x="1" y="1" width="6" height="6" rx="1" />
                <rect x="9" y="1" width="6" height="6" rx="1" />
                <rect x="1" y="9" width="6" height="6" rx="1" />
                <rect x="9" y="9" width="6" height="6" rx="1" />
              </svg>
            </button>
            <button
              className={`view-toggle-btn${viewMode === VIEW_MODES.LIST ? ' active' : ''}`}
              onClick={() => handleViewModeChange(VIEW_MODES.LIST)}
              aria-label="List view"
              title="List view"
            >
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <rect x="1" y="1" width="14" height="3" rx="1" />
                <rect x="1" y="6" width="14" height="3" rx="1" />
                <rect x="1" y="11" width="14" height="3" rx="1" />
              </svg>
            </button>
          </div>
          <div className="action-buttons">
            <Link to="/documents/new" className="btn btn-primary">
              + New Document
            </Link>
            <button
              className="btn btn-secondary"
              onClick={() => setShowUpload(!showUpload)}
            >
              📁 Upload *.md
            </button>
            <Link to="/import" className="btn btn-secondary">
              📥 Import
            </Link>
          </div>
        </div>
      </div>

      {/* Tag Filter */}
      {availableTags.length > 0 && (
        <div className="tag-filter-section">
          <div className="tag-filter-header">
            <span className="tag-filter-label">Filter by tags:</span>
            {selectedTags.length > 0 && (
              <button className="tag-filter-clear" onClick={clearTagFilters}>
                Clear all
              </button>
            )}
          </div>
          <div className="tag-filter-chips">
            {availableTags.slice(0, 15).map(tag => (
              <button
                key={tag.slug || tag.name}
                className={`tag-filter-chip${selectedTags.includes(tag.slug || tag.name) ? ' active' : ''}`}
                onClick={() => handleTagToggle(tag.slug || tag.name)}
              >
                {tag.name}
                {tag.count !== undefined && (
                  <span className="tag-filter-count">{tag.count}</span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Upload Status Messages */}
      {uploadStatus && (
        <div className={`upload-status ${uploadStatus.type}`}>
          <span>{uploadStatus.message}</span>
          <button className="close-btn" onClick={clearUploadStatus}>×</button>
        </div>
      )}

      {/* File Upload Area */}
      {showUpload && (
        <div className="upload-section">
          <div className="upload-header">
            <h3>Upload Markdown File</h3>
            <button className="close-btn" onClick={() => setShowUpload(false)}>×</button>
          </div>
          <FileUpload 
            onUploadSuccess={handleUploadSuccess}
            onUploadError={handleUploadError}
          />
        </div>
      )}

      {documents.length === 0 ? (
        <div className="no-documents">
          {searchQuery ? (
            <>
              <h3>No documents found</h3>
              <p>No documents match your search for "{searchQuery}"</p>
              <button className="btn btn-secondary" onClick={() => handleSearch('')}>
                Clear Search
              </button>
            </>
          ) : (
            <>
              <h3>No documents yet</h3>
              <p>Get started by creating your first document</p>
              <Link to="/documents/new" className="btn btn-primary">
                Create Document
              </Link>
            </>
          )}
        </div>
      ) : (
        <>
          <div className={`documents-${viewMode}`}>
            {documents.map((doc) => (
              viewMode === VIEW_MODES.LIST ? (
                <Link key={doc.id} to={`/documents/${doc.id}`} className="document-list-item">
                  <div className="document-list-item-icon">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
                      <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" fill="none" stroke="white" strokeWidth="2"/>
                    </svg>
                  </div>
                  <div className="document-list-item-content">
                    <span className="document-list-item-title">{doc.title}</span>
                    <span className="document-list-item-meta">
                      {doc.author && <span className="document-list-item-author">{doc.author}</span>}
                      <span className="document-list-item-date">{formatDate(doc.updated_at)}</span>
                    </span>
                  </div>
                  {doc.tags && doc.tags.length > 0 && (
                    <div className="document-list-item-tags">
                      {doc.tags.slice(0, 2).map(tag => (
                        <span key={tag.name || tag} className="document-list-item-tag">
                          {tag.name || tag}
                        </span>
                      ))}
                      {doc.tags.length > 2 && (
                        <span className="document-list-item-tag-more">+{doc.tags.length - 2}</span>
                      )}
                    </div>
                  )}
                </Link>
              ) : (
                <DocumentCard
                  key={doc.id}
                  document={doc}
                  searchQuery={searchQuery}
                  showPreview={true}
                  formatDate={formatDate}
                />
              )
            ))}
          </div>

          <Pagination
            pagination={pagination}
            currentPage={currentPage}
            onPageChange={handlePageChange}
          />
        </>
      )}

    </div>
  );
};

export default DocumentList;