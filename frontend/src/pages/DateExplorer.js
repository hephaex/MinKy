import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { documentService } from '../services/api';
import DocumentCard from '../components/DocumentCard';
import { logError } from '../utils/logger';
import './DateExplorer.css';

const DateExplorer = () => {
  const [recentDocuments, setRecentDocuments] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [activeAction, setActiveAction] = useState('recent');
  const [syncStatus, setSyncStatus] = useState(null);
  const navigate = useNavigate();

  useEffect(() => {
    loadRecentDocuments();
  }, []);

  const loadRecentDocuments = async () => {
    try {
      setLoading(true);
      setError(null);
      // Fetch recent documents (sorted by creation date)
      const response = await fetch('/api/documents?per_page=12&page=1');
      if (!response.ok) throw new Error('Failed to fetch documents');
      
      const data = await response.json();
      setRecentDocuments(data.documents);
    } catch (error) {
      logError('DateExplorer.fetchRecentDocuments', error);
      setError(error.message);
    } finally {
      setLoading(false);
    }
  };

  const handleDocumentClick = (document) => {
    navigate(`/documents/${document.id}`);
  };

  const handleCreateNew = () => {
    navigate('/documents/new');
  };

  const handleSearch = () => {
    navigate('/documents?search=');
  };

  const handleExploreByDate = () => {
    navigate('/explore-date');
  };

  const handleViewTags = () => {
    navigate('/tags');
  };

  const clearSyncStatus = () => {
    setSyncStatus(null);
  };

  const handleRepositorySync = () => {
    setActiveAction('sync-repo');
    // TODO: Implement Repository sync functionality
    setSyncStatus({
      type: 'info',
      message: 'Repository sync functionality will be implemented soon'
    });
  };

  const handleSettings = () => {
    setActiveAction('settings');
    navigate('/settings');
  };

  const formatDate = (dateString) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffTime = Math.abs(now - date);
    const diffDays = Math.ceil(diffTime / (1000 * 60 * 60 * 24));
    
    if (diffDays === 1) return '방금 전';
    if (diffDays < 7) return `${diffDays}일 전`;
    if (diffDays < 30) return `${Math.ceil(diffDays / 7)}주 전`;
    if (diffDays < 365) return `${Math.ceil(diffDays / 30)}개월 전`;
    return `${Math.ceil(diffDays / 365)}년 전`;
  };

  const formatTime = (dateString) => {
    return new Date(dateString).toLocaleTimeString('ko-KR', { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
  };

  if (loading) {
    return (
      <div className="date-explorer">
        <div className="loading">
          <h3>로딩 중</h3>
          <p>문서를 불러오고 있습니다...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="date-explorer">
        <div className="error">
          <h3>오류 발생</h3>
          <p>{error}</p>
          <button 
            className="create-button" 
            onClick={loadRecentDocuments}
            style={{ marginTop: '16px' }}
          >
            다시 시도
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="date-explorer">
      <div className="date-explorer-header">
        <h2>탐색</h2>
      </div>

      {/* Quick Actions */}
      <div className="quick-actions">
        <h3>빠른 작업</h3>
        <div className="action-grid">
          <button 
            className={`action-card ${activeAction === 'create' ? 'active' : ''}`}
            onClick={() => {
              setActiveAction('create');
              handleCreateNew();
            }}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2z"/>
            </svg>
            <span className="action-text">+ New Document Creation</span>
          </button>
          
          <button 
            className={`action-card ${activeAction === 'search' ? 'active' : ''}`}
            onClick={() => {
              setActiveAction('search');
              handleSearch();
            }}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11.742 10.344a6.5 6.5 0 1 0-1.397 1.398h-.001c.03.04.062.078.098.115l3.85 3.85a1 1 0 0 0 1.415-1.414l-3.85-3.85a1.007 1.007 0 0 0-.115-.1zM12 6.5a5.5 5.5 0 1 1-11 0 5.5 5.5 0 0 1 11 0z"/>
            </svg>
            <span className="action-text">Document Search</span>
          </button>
          
          <button 
            className={`action-card ${activeAction === 'date' ? 'active' : ''}`}
            onClick={() => {
              setActiveAction('date');
              handleExploreByDate();
            }}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M6.445 11.688V6.354h-.633V5.25l.633-.109h.828v6.547h-.828zm2.953 0V6.354h-.633V5.25l.633-.109h.828v6.547h-.828zm.844-8.158v.686c0 .348-.054.612-.162.792-.109.18-.309.27-.6.27-.291 0-.492-.09-.602-.27-.109-.18-.164-.444-.164-.792v-.686c0-.348.055-.612.164-.792.11-.18.311-.27.602-.27.291 0 .491.09.6.27.108.18.162.444.162.792z"/>
              <path d="M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zM7 3.5a.5.5 0 0 1 .5-.5h1a.5.5 0 0 1 .5.5v1a.5.5 0 0 1-.5.5h-1a.5.5 0 0 1-.5-.5v-1z"/>
            </svg>
            <span className="action-text">Browse by Date</span>
          </button>
          
          <button 
            className={`action-card ${activeAction === 'tags' ? 'active' : ''}`}
            onClick={() => {
              setActiveAction('tags');
              handleViewTags();
            }}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M2 2a1 1 0 0 1 1-1h4.586a1 1 0 0 1 .707.293l7 7a1 1 0 0 1 0 1.414l-7 7a1 1 0 0 1-1.414 0l-7-7A1 1 0 0 1 0 9.586V3a1 1 0 0 1 1-1H2zm2 3a1 1 0 1 0 0-2 1 1 0 0 0 0 2z"/>
            </svg>
            <span className="action-text">View Tags</span>
          </button>

          <button 
            className={`action-card ${activeAction === 'sync-repo' ? 'active' : ''}`}
            onClick={handleRepositorySync}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/>
            </svg>
            <span className="action-text">Sync Repository</span>
          </button>

          <button 
            className={`action-card ${activeAction === 'settings' ? 'active' : ''}`}
            onClick={() => {
              setActiveAction('settings');
              handleSettings();
            }}
          >
            <svg className="action-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
              <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.292-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.292c.415.764-.42 1.6-1.185 1.184l-.292-.159a1.873 1.873 0 0 0-2.692 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.693-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.292A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115l.094-.319z"/>
            </svg>
            <span className="action-text">Settings</span>
          </button>
        </div>
      </div>

      {/* Sync Status Messages */}
      {syncStatus && (
        <div className={`sync-status ${syncStatus.type}`}>
          <span>{syncStatus.message}</span>
          <button className="close-btn" onClick={clearSyncStatus}>×</button>
        </div>
      )}

      {/* Recent Documents */}
      <div className="recent-section">
        <div className="section-header">
          <h3 className="section-title">최근 문서</h3>
          <button className="create-button" onClick={handleCreateNew}>
            <svg className="icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2z"/>
            </svg>
            새로 만들기
          </button>
        </div>
        
        {recentDocuments.length === 0 ? (
          <div className="empty-state">
            <h3>문서가 없습니다</h3>
            <p>첫 번째 문서를 작성해보세요!</p>
            <button className="create-button" onClick={handleCreateNew}>
              <svg className="icon" viewBox="0 0 16 16" fill="currentColor">
                <path d="M8 2a.5.5 0 0 1 .5.5v5h5a.5.5 0 0 1 0 1h-5v5a.5.5 0 0 1-1 0v-5h-5a.5.5 0 0 1 0-1h5v-5A.5.5 0 0 1 8 2z"/>
              </svg>
              문서 작성하기
            </button>
          </div>
        ) : (
          <div className="documents-grid">
            {recentDocuments.map(doc => (
              <DocumentCard
                key={doc.id}
                document={{
                  ...doc,
                  updated_at: doc.created_at, // Use created_at for recent documents
                  title: doc.title || '제목 없음'
                }}
                formatDate={(dateString) => `${formatDate(dateString)} • ${formatTime(dateString)}`}
              />
            ))}
          </div>
        )}
      </div>

    </div>
  );
};

export default DateExplorer;