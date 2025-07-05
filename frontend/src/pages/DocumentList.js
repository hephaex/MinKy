import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { documentService } from '../services/api';
import SearchBar from '../components/SearchBar';
import Pagination from '../components/Pagination';
import { highlightTextReact, truncateWithHighlight } from '../utils/highlightText';
import './DocumentList.css';

const DocumentList = () => {
  const [documents, setDocuments] = useState([]);
  const [pagination, setPagination] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentPage, setCurrentPage] = useState(1);

  const fetchDocuments = async (page = 1, search = '') => {
    try {
      setLoading(true);
      const data = await documentService.getDocuments(page, 10, search);
      setDocuments(data.documents);
      setPagination(data.pagination);
      setCurrentPage(page);
      setError(null);
    } catch (err) {
      setError('Failed to fetch documents');
      console.error('Error fetching documents:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchDocuments(1, searchQuery);
  }, [searchQuery]);

  const handleSearch = (query) => {
    setSearchQuery(query);
    setCurrentPage(1);
  };

  const handlePageChange = (page) => {
    fetchDocuments(page, searchQuery);
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
          <Link to="/documents/new" className="btn btn-primary">
            + New Document
          </Link>
        </div>
      </div>

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
              <div key={doc.id} className="document-card">
                <Link to={`/documents/${doc.id}`} className="document-link">
                  <h3 className="document-title">
                    {searchQuery ? highlightTextReact(doc.title, searchQuery) : doc.title}
                  </h3>
                  <div className="document-meta">
                    {doc.author && (
                      <span className="document-author">
                        By {searchQuery ? highlightTextReact(doc.author, searchQuery) : doc.author}
                      </span>
                    )}
                    <span className="document-date">
                      Updated {formatDate(doc.updated_at)}
                    </span>
                  </div>
                  <div className="document-preview">
                    {searchQuery ? 
                      highlightTextReact(
                        truncateWithHighlight(doc.markdown_content, searchQuery, 150),
                        searchQuery
                      ) : (
                      <>
                        {doc.markdown_content.substring(0, 150)}
                        {doc.markdown_content.length > 150 && '...'}
                      </>
                    )}
                  </div>
                </Link>
              </div>
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