import React, { useState, useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { documentService } from '../services/api';
import './DocumentsSidebar.css';

const DocumentsSidebar = ({ isVisible, onToggle }) => {
  const navigate = useNavigate();
  const [recentDocuments, setRecentDocuments] = useState([]);
  const [allDocuments, setAllDocuments] = useState([]);
  const [loading, setLoading] = useState(false);
  const [allMdExpanded, setAllMdExpanded] = useState(false);

  useEffect(() => {
    if (isVisible) {
      fetchRecentDocuments();
      fetchAllDocuments();
    }
  }, [isVisible]);

  const fetchRecentDocuments = async () => {
    try {
      setLoading(true);
      const response = await documentService.getDocuments(1, 5, '');
      setRecentDocuments(response.documents || []);
    } catch (error) {
      console.error('Error fetching recent documents:', error);
    } finally {
      setLoading(false);
    }
  };

  const fetchAllDocuments = async () => {
    try {
      const response = await documentService.getDocuments(1, 50, '');
      setAllDocuments(response.documents || []);
    } catch (error) {
      console.error('Error fetching all documents:', error);
    }
  };

  const handleNewDocument = () => {
    navigate('/documents/new');
  };

  const formatDate = (dateString) => {
    return new Date(dateString).toLocaleDateString('ko-KR', {
      month: 'short',
      day: 'numeric'
    });
  };

  const truncateTitle = (title, maxLength = 30) => {
    return title.length > maxLength ? title.substring(0, maxLength) + '...' : title;
  };

  return (
    <div className={`documents-sidebar ${!isVisible ? 'hidden' : ''}`}>
      <div className="sidebar-header">
        <button className="sidebar-toggle" onClick={onToggle}>
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M2.5 12a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5z"/>
          </svg>
        </button>
        <h2>Documents</h2>
      </div>

      <div className="sidebar-content">
        {/* New Document Button */}
        <div className="sidebar-section">
          <button className="new-document-btn" onClick={handleNewDocument}>
            <div className="btn-icon">+</div>
            <span>새 문서</span>
          </button>
        </div>

        {/* Quick Actions */}
        <div className="sidebar-section">
          <div className="section-item">
            <div className="item-icon">📥</div>
            <Link to="/import" className="item-link">Import</Link>
          </div>
        </div>

        {/* Recent Items */}
        <div className="sidebar-section">
          <h3 className="section-title">최근 항목</h3>
          {loading ? (
            <div className="loading-indicator">로딩 중...</div>
          ) : (
            <div className="document-list">
              {recentDocuments.map((doc) => (
                <Link 
                  key={doc.id} 
                  to={`/documents/${doc.id}`} 
                  className="document-item"
                >
                  <div className="document-title">
                    {truncateTitle(doc.title)}
                  </div>
                  <div className="document-date">
                    {formatDate(doc.created_at)}
                  </div>
                </Link>
              ))}
              {recentDocuments.length === 0 && !loading && (
                <div className="empty-state">최근 문서가 없습니다</div>
              )}
            </div>
          )}
        </div>

        {/* All MD Files */}
        <div className="sidebar-section">
          <div className="section-header">
            <h3 className="section-title">모든 md</h3>
            <button 
              className="toggle-btn"
              onClick={() => setAllMdExpanded(!allMdExpanded)}
            >
              <svg 
                width="12" 
                height="12" 
                viewBox="0 0 12 12" 
                fill="currentColor"
                style={{ transform: allMdExpanded ? 'rotate(90deg)' : 'rotate(0deg)', transition: 'transform 0.2s' }}
              >
                <path d="M4.5 1.5l4 3-4 3z"/>
              </svg>
            </button>
          </div>
          {allMdExpanded && (
            <div className="document-list">
              {allDocuments.slice(0, 15).map((doc) => (
                <Link 
                  key={doc.id} 
                  to={`/documents/${doc.id}`} 
                  className="document-item"
                >
                  <div className="document-title">
                    {truncateTitle(doc.title)}
                  </div>
                  <div className="document-date">
                    {formatDate(doc.updated_at)}
                  </div>
                </Link>
              ))}
              {allDocuments.length === 0 && !loading && (
                <div className="empty-state">문서가 없습니다</div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default DocumentsSidebar;