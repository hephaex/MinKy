import React, { useState, useEffect } from 'react';

const SimpleDateSidebar = ({ onDocumentSelect, selectedDateKey }) => {
  const [timeline, setTimeline] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [expandedItems, setExpandedItems] = useState(new Set(['2025']));

  useEffect(() => {
    const loadTimeline = async () => {
      try {
        setLoading(true);
        const response = await fetch('/api/documents/timeline?group_by=month');
        if (!response.ok) throw new Error('Failed to fetch timeline');
        
        const data = await response.json();
        setTimeline(data.timeline);
        setError(null);
      } catch (err) {
        setError(err.message);
      } finally {
        setLoading(false);
      }
    };

    loadTimeline();
  }, []);

  const toggleExpanded = (key) => {
    const newExpanded = new Set(expandedItems);
    if (newExpanded.has(key)) {
      newExpanded.delete(key);
    } else {
      newExpanded.add(key);
    }
    setExpandedItems(newExpanded);
  };

  const handleItemClick = (item) => {
    onDocumentSelect?.(item.key);
  };
  
  const handleArrowClick = (item) => {
    toggleExpanded(item.key);
  };

  if (loading) {
    return (
      <div style={{
        width: '280px',
        height: 'calc(100vh - 80px)',
        background: '#f8f9fa',
        borderRight: '1px solid #e0e0e0',
        position: 'fixed',
        left: '0',
        top: '80px',
        padding: '20px'
      }}>
        <h3>문서 탐색</h3>
        <p>로딩 중...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{
        width: '280px',
        height: 'calc(100vh - 80px)',
        background: '#f8f9fa',
        borderRight: '1px solid #e0e0e0',
        position: 'fixed',
        left: '0',
        top: '80px',
        padding: '20px'
      }}>
        <h3>문서 탐색</h3>
        <p style={{ color: 'red' }}>오류: {error}</p>
      </div>
    );
  }

  return (
    <div style={{
      width: '280px',
      height: 'calc(100vh - 80px)',
      background: '#f8f9fa',
      borderRight: '1px solid #e0e0e0',
      position: 'fixed',
      left: '0',
      top: '80px',
      display: 'flex',
      flexDirection: 'column'
    }}>
      <div style={{
        padding: '20px 16px 16px',
        background: '#fff',
        borderBottom: '1px solid #e0e0e0'
      }}>
        <h3 style={{ margin: '0 0 12px 0', fontSize: '18px' }}>문서 탐색</h3>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '8px 0' }}>
        {Object.keys(timeline).length === 0 ? (
          <div style={{ padding: '20px 16px', textAlign: 'center', color: '#666' }}>
            문서가 없습니다
          </div>
        ) : (
          <div>
            {Object.values(timeline)
              .sort((a, b) => b.key.localeCompare(a.key))
              .map(item => (
                <div key={item.key}>
                  <div 
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      padding: '8px 16px',
                      cursor: 'pointer',
                      fontSize: '15px',
                      fontWeight: '600',
                      color: '#333',
                      borderLeft: selectedDateKey === item.key ? '3px solid #2196f3' : '3px solid transparent',
                      backgroundColor: selectedDateKey === item.key ? '#e3f2fd' : 'transparent'
                    }}
                    onMouseEnter={(e) => e.target.style.backgroundColor = '#e9ecef'}
                    onMouseLeave={(e) => e.target.style.backgroundColor = selectedDateKey === item.key ? '#e3f2fd' : 'transparent'}
                  >
                    {item.children && Object.keys(item.children).length > 0 && (
                      <span 
                        onClick={() => handleArrowClick(item)}
                        style={{
                          marginRight: '8px',
                          transform: expandedItems.has(item.key) ? 'rotate(90deg)' : 'rotate(0deg)',
                          transition: 'transform 0.2s ease',
                          cursor: 'pointer'
                        }}>
                        ▶
                      </span>
                    )}
                    <span 
                      style={{ flex: 1, cursor: 'pointer' }}
                      onClick={() => handleItemClick(item)}
                    >
                      {item.label}
                    </span>
                    <span 
                      style={{
                        marginLeft: '8px',
                        fontSize: '12px',
                        color: '#999',
                        background: '#f0f0f0',
                        padding: '2px 6px',
                        borderRadius: '10px',
                        cursor: 'pointer'
                      }}
                      onClick={() => handleItemClick(item)}
                    >
                      ({item.count})
                    </span>
                  </div>

                  {item.children && expandedItems.has(item.key) && (
                    <div style={{ marginLeft: '20px', borderLeft: '1px solid #e0e0e0' }}>
                      {Object.values(item.children)
                        .sort((a, b) => b.key.localeCompare(a.key))
                        .map(child => (
                          <div 
                            key={child.key}
                            onClick={() => onDocumentSelect?.(child.key)}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              padding: '8px 16px',
                              cursor: 'pointer',
                              fontSize: '14px',
                              fontWeight: '500',
                              color: '#555',
                              borderLeft: selectedDateKey === child.key ? '3px solid #2196f3' : '3px solid transparent',
                              backgroundColor: selectedDateKey === child.key ? '#e3f2fd' : 'transparent'
                            }}
                            onMouseEnter={(e) => e.target.style.backgroundColor = '#e9ecef'}
                            onMouseLeave={(e) => e.target.style.backgroundColor = selectedDateKey === child.key ? '#e3f2fd' : 'transparent'}
                          >
                            <span style={{ flex: 1 }}>{child.label}</span>
                            <span style={{
                              marginLeft: '8px',
                              fontSize: '12px',
                              color: '#999',
                              background: '#f0f0f0',
                              padding: '2px 6px',
                              borderRadius: '10px'
                            }}>
                              ({child.count})
                            </span>
                          </div>
                        ))}
                    </div>
                  )}
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default SimpleDateSidebar;