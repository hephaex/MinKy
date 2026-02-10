import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import Pagination from './Pagination';
import { formatDateTime, formatDateRange as formatDateRangeUtil } from '../utils/dateUtils';
import './DocumentsByDate.css';

const DocumentsByDate = ({ dateKey, onDocumentClick }) => {
  const [state, setState] = useState({
    documents: [],
    pagination: null,
    dateRange: null,
    loading: true,
    error: null,
    currentPage: 1
  });

  const fetchDocumentsByDate = React.useCallback(async (page = 1) => {
    if (!dateKey) return;
    
    try {
      setState(prev => ({ ...prev, loading: true }));
      const response = await fetch(
        `/api/documents/by-date?date_key=${encodeURIComponent(dateKey)}&page=${page}&per_page=10`
      );
      if (!response.ok) throw new Error('Failed to fetch documents');
      
      const data = await response.json();
      setState(prev => ({
        ...prev,
        documents: data.documents,
        pagination: data.pagination,
        dateRange: data.date_range,
        currentPage: page,
        error: null,
        loading: false
      }));
    } catch (err) {
      setState(prev => ({
        ...prev,
        error: err.message,
        loading: false
      }));
    }
  }, [dateKey]);

  useEffect(() => {
    if (dateKey) {
      fetchDocumentsByDate(1);
    }
  }, [dateKey, fetchDocumentsByDate]);

  const handlePageChange = React.useCallback((page) => {
    fetchDocumentsByDate(page);
  }, [fetchDocumentsByDate]);

  const formatDateRange = (dateRange) => {
    if (!dateRange) return '';
    
    const start = new Date(dateRange.start);
    
    if (dateKey.length === 4) {
      // Year
      return `${dateKey}년`;
    } else if (dateKey.length === 7) {
      // Month
      const [year, month] = dateKey.split('-');
      return `${year}년 ${parseInt(month)}월`;
    } else if (dateKey.length === 10) {
      // Day
      return start.toLocaleDateString('ko-KR', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'long'
      });
    } else if (dateKey.includes('W')) {
      // Week
      const [year, week] = dateKey.split('-W');
      return `${year}년 ${parseInt(week)}주차`;
    }
    
    return dateKey;
  };

  const truncateContent = (content, maxLength = 120) => {
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + '...';
  };

  if (!dateKey) {
    return (
      <div className="documents-by-date">
        <div className="no-selection">
          <h3>날짜를 선택해주세요</h3>
          <p>왼쪽 사이드바에서 날짜를 클릭하여 해당 기간의 문서를 확인할 수 있습니다.</p>
        </div>
      </div>
    );
  }

  if (state.loading) {
    return (
      <div className="documents-by-date">
        <div className="loading">
          <div className="spinner"></div>
          <p>문서를 불러오는 중...</p>
        </div>
      </div>
    );
  }

  if (state.error) {
    return (
      <div className="documents-by-date">
        <div className="error">
          <h3>오류가 발생했습니다</h3>
          <p>{state.error}</p>
          <button onClick={() => fetchDocumentsByDate(state.currentPage)} className="retry-btn">
            다시 시도
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="documents-by-date">
      <div className="documents-header">
        <h2>{formatDateRange(state.dateRange)}</h2>
        <p className="document-count">
          총 {state.pagination?.total || 0}개의 문서
        </p>
      </div>

      {state.documents.length === 0 ? (
        <div className="no-documents">
          <h3>문서가 없습니다</h3>
          <p>이 기간에 작성된 문서가 없습니다.</p>
        </div>
      ) : (
        <>
          <div className="documents-grid">
            {state.documents.map(doc => (
              <div key={doc.id} className="document-card">
                <div className="document-header">
                  <h3 className="document-title">
                    <Link 
                      to={`/documents/${doc.id}`}
                      onClick={() => onDocumentClick?.(doc)}
                    >
                      {doc.title || '제목 없음'}
                    </Link>
                  </h3>
                  <span className="document-date">
                    {formatDateTime(doc.created_at)}
                  </span>
                </div>
                
                <div className="document-content">
                  <p>{truncateContent(doc.content || doc.markdown_content || '')}</p>
                </div>
                
                <div className="document-meta">
                  {doc.author && (
                    <span className="document-author">작성자: {doc.author}</span>
                  )}
                  {doc.tags && doc.tags.length > 0 && (
                    <div className="document-tags">
                      {doc.tags.slice(0, 3).map(tag => (
                        <span key={tag.id || tag} className="tag">
                          {tag.name || tag}
                        </span>
                      ))}
                      {doc.tags.length > 3 && (
                        <span className="tag-more">+{doc.tags.length - 3}</span>
                      )}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>

          {state.pagination && state.pagination.pages > 1 && (
            <Pagination
              currentPage={state.pagination.page}
              totalPages={state.pagination.pages}
              onPageChange={handlePageChange}
            />
          )}
        </>
      )}
    </div>
  );
};

export default DocumentsByDate;