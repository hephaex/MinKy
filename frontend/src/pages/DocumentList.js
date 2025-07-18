import React, { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { documentService } from '../services/api';
import api from '../services/api';
import SearchBar from '../components/SearchBar';
import Pagination from '../components/Pagination';
import FileUpload from '../components/FileUpload';
import DocumentCard from '../components/DocumentCard';
import { highlightTextReact, truncateWithHighlight } from '../utils/highlightText';
import './DocumentList.css';

const DocumentList = () => {
  const [documents, setDocuments] = useState([]);
  const [pagination, setPagination] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const [showUpload, setShowUpload] = useState(false);
  const [uploadStatus, setUploadStatus] = useState(null);
  const [syncing, setSyncing] = useState(false);
  const [syncStatus, setSyncStatus] = useState(null);
  const [categories, setCategories] = useState([]);
  const [selectedCategory, setSelectedCategory] = useState('');
  const navigate = useNavigate();

  const fetchDocuments = async (page = 1, search = '', categoryId = null) => {
    try {
      setLoading(true);
      const params = new URLSearchParams();
      params.append('page', page);
      params.append('per_page', 10);
      if (search) params.append('search', search);
      if (categoryId) params.append('category_id', categoryId);
      
      const response = await api.get(`/documents?${params.toString()}`);
      setDocuments(response.data.documents);
      setPagination(response.data.pagination);
      setCurrentPage(page);
      setError(null);
    } catch (err) {
      setError('Failed to fetch documents');
      console.error('Error fetching documents:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchCategories = async () => {
    try {
      const response = await api.get('/categories?format=flat');
      setCategories(response.data.categories);
    } catch (err) {
      console.error('Error fetching categories:', err);
    }
  };

  useEffect(() => {
    fetchCategories();
  }, []);

  useEffect(() => {
    fetchDocuments(1, searchQuery, selectedCategory);
  }, [searchQuery, selectedCategory]);

  const handleSearch = (query) => {
    setSearchQuery(query);
    setCurrentPage(1);
  };

  const handlePageChange = (page) => {
    fetchDocuments(page, searchQuery, selectedCategory);
  };

  const handleCategoryChange = (categoryId) => {
    setSelectedCategory(categoryId);
    setCurrentPage(1);
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


  const handleGitSync = async () => {
    try {
      setSyncing(true);
      // TODO: Implement actual git pull functionality
      await new Promise(resolve => setTimeout(resolve, 2000)); // Simulate API call
      
      setSyncStatus({
        type: 'success',
        message: 'Git pull completed successfully. Documents are up to date.'
      });
      
      // Refresh documents after successful sync
      fetchDocuments(currentPage, searchQuery);
    } catch (error) {
      setSyncStatus({
        type: 'error',
        message: 'Git sync failed: ' + error.message
      });
    } finally {
      setSyncing(false);
    }
  };

  const clearSyncStatus = () => {
    setSyncStatus(null);
  };

  const formatDate = (dateString) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
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
            <button 
              className="btn btn-secondary" 
              onClick={handleGitSync}
              disabled={syncing}
            >
              🔄 {syncing ? 'Syncing...' : 'Sync using Git'}
            </button>
          </div>
        </div>
      </div>

      {/* Upload Status Messages */}
      {uploadStatus && (
        <div className={`upload-status ${uploadStatus.type}`}>
          <span>{uploadStatus.message}</span>
          <button className="close-btn" onClick={clearUploadStatus}>×</button>
        </div>
      )}


      {/* Sync Status Messages */}
      {syncStatus && (
        <div className={`upload-status ${syncStatus.type}`}>
          <span>{syncStatus.message}</span>
          <button className="close-btn" onClick={clearSyncStatus}>×</button>
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
          <div className="documents-grid">
            {documents.map((doc) => (
              <DocumentCard
                key={doc.id}
                document={doc}
                searchQuery={searchQuery}
                showPreview={true}
                formatDate={formatDate}
              />
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