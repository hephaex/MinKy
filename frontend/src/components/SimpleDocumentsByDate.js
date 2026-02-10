import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { logError } from '../utils/logger';

const SimpleDocumentsByDate = ({ dateKey, onDocumentClick }) => {
  const [documents, setDocuments] = useState([]);
  const [pagination, setPagination] = useState(null);
  const [dateRange, setDateRange] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  useEffect(() => {
    if (!dateKey) return;

    const loadDocuments = async () => {
      try {
        setLoading(true);
        const url = `/api/documents/by-date?date_key=${encodeURIComponent(dateKey)}&page=1&per_page=50`;
        const response = await fetch(url);
        if (!response.ok) {
          const errorText = await response.text();
          logError('SimpleDocumentsByDate.loadDocuments', new Error(`API Error: ${response.status}`), { errorText });
          throw new Error(`Failed to fetch documents: ${response.status}`);
        }
        
        const data = await response.json();
        setDocuments(data.documents);
        setPagination(data.pagination);
        setDateRange(data.date_range);
        setError(null);
      } catch (err) {
        logError('SimpleDocumentsByDate.loadDocuments', err);
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    loadDocuments();
  }, [dateKey]);

  const formatDateRange = (dateRange) => {
    if (!dateRange) return '';
    
    if (dateKey.length === 4) {
      return `${dateKey}년`;
    } else if (dateKey.length === 7) {
      const [year, month] = dateKey.split('-');
      return `${year}년 ${parseInt(month)}월`;
    } else if (dateKey.length === 10) {
      const date = new Date(dateRange.start);
      return date.toLocaleDateString('ko-KR', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'long'
      });
    } else if (dateKey.includes('W')) {
      const [year, week] = dateKey.split('-W');
      return `${year}년 ${parseInt(week)}주차`;
    }
    
    return dateKey;
  };

  const formatDate = (dateString) => {
    const date = new Date(dateString);
    return date.toLocaleDateString('ko-KR', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  const truncateContent = (content, maxLength = 120) => {
    if (!content) return '';
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + '...';
  };

  if (!dateKey) {
    return (
      <div style={{
        padding: '24px',
        maxWidth: '1200px',
        margin: '0 auto',
        minHeight: '600px'
      }}>
        <div style={{
          textAlign: 'center',
          padding: '80px 20px',
          color: '#666'
        }}>
          <h3 style={{ margin: '0 0 16px 0', fontSize: '24px', color: '#999' }}>
            날짜를 선택해주세요
          </h3>
          <p style={{ margin: '0', fontSize: '16px', lineHeight: '1.5' }}>
            왼쪽 사이드바에서 날짜를 클릭하여 해당 기간의 문서를 확인할 수 있습니다.
          </p>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div style={{
        padding: '24px',
        maxWidth: '1200px',
        margin: '0 auto',
        minHeight: '600px'
      }}>
        <div style={{
          textAlign: 'center',
          padding: '80px 20px',
          color: '#666'
        }}>
          <div style={{
            width: '40px',
            height: '40px',
            border: '4px solid #f3f3f3',
            borderTop: '4px solid #007bff',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite',
            margin: '0 auto 16px'
          }}></div>
          <p>문서를 불러오는 중...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{
        padding: '24px',
        maxWidth: '1200px',
        margin: '0 auto',
        minHeight: '600px'
      }}>
        <div style={{
          textAlign: 'center',
          padding: '80px 20px',
          color: '#dc3545'
        }}>
          <h3 style={{ margin: '0 0 16px 0', fontSize: '24px' }}>
            오류가 발생했습니다
          </h3>
          <p style={{ margin: '0 0 24px 0', fontSize: '16px' }}>{error}</p>
          <button 
            onClick={() => window.location.reload()}
            style={{
              padding: '12px 24px',
              background: '#007bff',
              color: 'white',
              border: 'none',
              borderRadius: '6px',
              fontSize: '16px',
              cursor: 'pointer'
            }}
          >
            다시 시도
          </button>
        </div>
      </div>
    );
  }

  return (
    <div style={{
      padding: '24px',
      maxWidth: '1200px',
      margin: '0 auto',
      minHeight: '600px'
    }}>
      <div style={{
        marginBottom: '24px',
        borderBottom: '2px solid #e0e0e0',
        paddingBottom: '16px'
      }}>
        <h2 style={{
          margin: '0 0 8px 0',
          fontSize: '28px',
          fontWeight: '700',
          color: '#1a1a1a'
        }}>
          {formatDateRange(dateRange)}
        </h2>
        <p style={{
          margin: '0',
          fontSize: '16px',
          color: '#666'
        }}>
          총 {pagination?.total || 0}개의 문서
        </p>
      </div>

      {documents.length === 0 ? (
        <div style={{
          textAlign: 'center',
          padding: '80px 20px',
          color: '#666'
        }}>
          <h3 style={{ margin: '0 0 16px 0', fontSize: '24px', color: '#999' }}>
            문서가 없습니다
          </h3>
          <p style={{ margin: '0', fontSize: '16px' }}>
            이 기간에 작성된 문서가 없습니다.
          </p>
        </div>
      ) : (
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(400px, 1fr))',
          gap: '24px',
          marginBottom: '32px'
        }}>
          {documents.map(doc => (
            <div key={doc.id} style={{
              background: 'white',
              border: '1px solid #e0e0e0',
              borderRadius: '12px',
              padding: '24px',
              transition: 'all 0.2s ease',
              boxShadow: '0 2px 4px rgba(0, 0, 0, 0.05)',
              cursor: 'pointer'
            }}
            onClick={() => onDocumentClick?.(doc)}
            onMouseEnter={(e) => {
              e.currentTarget.style.transform = 'translateY(-2px)';
              e.currentTarget.style.boxShadow = '0 8px 16px rgba(0, 0, 0, 0.1)';
              e.currentTarget.style.borderColor = '#c0c0c0';
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.transform = 'translateY(0)';
              e.currentTarget.style.boxShadow = '0 2px 4px rgba(0, 0, 0, 0.05)';
              e.currentTarget.style.borderColor = '#e0e0e0';
            }}
            >
              <div style={{ marginBottom: '16px' }}>
                <h3 style={{
                  margin: '0 0 8px 0',
                  fontSize: '20px',
                  fontWeight: '600',
                  lineHeight: '1.3'
                }}>
                  <Link 
                    to={`/documents/${doc.id}`}
                    style={{
                      color: '#1a1a1a',
                      textDecoration: 'none',
                      transition: 'color 0.2s'
                    }}
                    onMouseEnter={(e) => e.target.style.color = '#007bff'}
                    onMouseLeave={(e) => e.target.style.color = '#1a1a1a'}
                  >
                    {doc.title || '제목 없음'}
                  </Link>
                </h3>
                <span style={{
                  fontSize: '14px',
                  color: '#999',
                  fontWeight: 'normal'
                }}>
                  {formatDate(doc.created_at)}
                </span>
              </div>
              
              <div style={{ marginBottom: '16px' }}>
                <p style={{
                  margin: '0',
                  fontSize: '15px',
                  lineHeight: '1.5',
                  color: '#555'
                }}>
                  {truncateContent(doc.content || doc.markdown_content || '')}
                </p>
              </div>
              
              <div style={{
                display: 'flex',
                flexDirection: 'column',
                gap: '8px',
                paddingTop: '16px',
                borderTop: '1px solid #f0f0f0'
              }}>
                {doc.author && (
                  <span style={{ fontSize: '14px', color: '#666' }}>
                    작성자: {doc.author}
                  </span>
                )}
                {doc.tags && doc.tags.length > 0 && (
                  <div style={{
                    display: 'flex',
                    flexWrap: 'wrap',
                    gap: '6px'
                  }}>
                    {doc.tags.slice(0, 3).map((tag, index) => (
                      <span key={index} style={{
                        display: 'inline-block',
                        padding: '4px 8px',
                        background: '#f8f9fa',
                        border: '1px solid #e0e0e0',
                        borderRadius: '12px',
                        fontSize: '12px',
                        color: '#666',
                        fontWeight: '500'
                      }}>
                        {tag.name || tag}
                      </span>
                    ))}
                    {doc.tags.length > 3 && (
                      <span style={{
                        display: 'inline-block',
                        padding: '4px 8px',
                        background: '#e9ecef',
                        borderRadius: '12px',
                        fontSize: '12px',
                        color: '#6c757d',
                        fontWeight: '500'
                      }}>
                        +{doc.tags.length - 3}
                      </span>
                    )}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SimpleDocumentsByDate;