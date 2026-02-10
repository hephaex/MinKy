import React, { useState, useEffect } from 'react';
import { useParams, Link } from 'react-router-dom';
import { tagService } from '../services/api';
import Pagination from '../components/Pagination';
import { highlightTextReact, truncateWithHighlight } from '../utils/highlightText';
import { logError } from '../utils/logger';
import { formatDateTime } from '../utils/dateUtils';
import './TagDetail.css';

const TagDetail = () => {
  const { slug } = useParams();
  const [tagData, setTagData] = useState(null);
  const [documents, setDocuments] = useState([]);
  const [pagination, setPagination] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [currentPage, setCurrentPage] = useState(1);

  useEffect(() => {
    fetchTagData(1);
  }, [slug]);

  const fetchTagData = async (page = 1) => {
    try {
      setLoading(true);
      const data = await tagService.getTag(slug, page, 10);
      setTagData(data.tag);
      setDocuments(data.documents);
      setPagination(data.pagination);
      setCurrentPage(page);
      setError(null);
    } catch (err) {
      setError('Failed to fetch tag data');
      logError('TagDetail.fetchTagData', err);
    } finally {
      setLoading(false);
    }
  };

  const handlePageChange = (page) => {
    fetchTagData(page);
  };

  const formatAuthor = (author) => {
    if (!author) return '';
    
    // Handle case where author might be a JSON string/array
    if (typeof author === 'string') {
      try {
        // Try to parse as JSON in case it's serialized
        const parsed = JSON.parse(author);
        if (Array.isArray(parsed) && parsed.length > 0) {
          author = parsed[0];
        } else if (typeof parsed === 'string') {
          author = parsed;
        }
      } catch (e) {
        // If parsing fails, use the string as-is
      }
    }
    
    // Handle array case
    if (Array.isArray(author) && author.length > 0) {
      author = author[0];
    }
    
    // Clean up the author string
    if (typeof author === 'string') {
      author = author.trim();
      // Remove Obsidian-style wiki links: [[name]] -> name
      if (author.startsWith('[[') && author.endsWith(']]')) {
        author = author.slice(2, -2);
      }
      // Remove quotes
      author = author.replace(/^["']|["']$/g, '');
    }
    
    return author;
  };

  if (loading) {
    return <div className="loading">Loading tag...</div>;
  }

  if (error || !tagData) {
    return (
      <div className="error">
        <h2>Tag not found</h2>
        <p>{error || 'The requested tag could not be found.'}</p>
        <Link to="/tags" className="btn btn-primary">
          Back to Tags
        </Link>
      </div>
    );
  }

  return (
    <div className="tag-detail">
      {/* Tag Header */}
      <div className="tag-header-section">
        <div className="tag-breadcrumb">
          <Link to="/tags" className="breadcrumb-link">Tags</Link>
          <span className="breadcrumb-separator">/</span>
          <span className="breadcrumb-current">{tagData.name}</span>
        </div>

        <div className="tag-info">
          <div className="tag-title-section">
            <div 
              className="tag-color-large"
              style={{ backgroundColor: tagData.color }}
            ></div>
            <div className="tag-title-content">
              <h1 className="tag-title">{tagData.name}</h1>
              <div className="tag-stats">
                <span className="tag-stat">
                  {documents.length === 0 ? 'No documents' : 
                   documents.length === 1 ? '1 document' : 
                   `${pagination.total || documents.length} documents`}
                </span>
                {!tagData.description && (
                  <span className="tag-badge auto-generated">Auto-generated</span>
                )}
              </div>
            </div>
          </div>

          {tagData.description && (
            <div className="tag-description">
              <p>{tagData.description}</p>
            </div>
          )}

          <div className="tag-meta">
            <small>
              Created: {formatDateTime(tagData.created_at)}
              {tagData.creator && ` by ${tagData.creator}`}
            </small>
          </div>
        </div>
      </div>

      {/* Documents Section */}
      <div className="tag-documents-section">
        <h2>Documents with this tag</h2>
        
        {documents.length === 0 ? (
          <div className="no-documents">
            <h3>No documents found</h3>
            <p>No documents are currently tagged with "{tagData.name}"</p>
            <Link to="/documents/new" className="btn btn-primary">
              Create New Document
            </Link>
          </div>
        ) : (
          <>
            <div className="documents-grid">
              {documents.map((doc) => (
                <div key={doc.id} className="document-card">
                  <Link to={`/documents/${doc.id}`} className="document-link">
                    <h3 className="document-title">{doc.title}</h3>
                    
                    <div className="document-meta">
                      {doc.author && (
                        <span className="document-author">By {formatAuthor(doc.author)}</span>
                      )}
                      <span className="document-date">
                        Updated {formatDateTime(doc.updated_at)}
                      </span>
                    </div>
                    
                    <div className="document-preview">
                      {doc.markdown_content.substring(0, 150)}
                      {doc.markdown_content.length > 150 && '...'}
                    </div>
                    
                    {doc.tag_names && doc.tag_names.length > 1 && (
                      <div className="document-tags">
                        {doc.tag_names.filter(tag => tag !== tagData.name).slice(0, 3).map((tag) => (
                          <Link
                            key={tag}
                            to={`/tags/${tag.toLowerCase()}`}
                            className="document-tag"
                            onClick={(e) => e.stopPropagation()}
                          >
                            {tag}
                          </Link>
                        ))}
                        {doc.tag_names.length > 4 && (
                          <span className="more-tags">+{doc.tag_names.length - 4} more</span>
                        )}
                      </div>
                    )}
                  </Link>
                </div>
              ))}
            </div>

            {pagination && pagination.pages > 1 && (
              <Pagination
                pagination={pagination}
                currentPage={currentPage}
                onPageChange={handlePageChange}
              />
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default TagDetail;