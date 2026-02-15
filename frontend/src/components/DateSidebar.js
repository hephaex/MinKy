import React, { useState, useEffect } from 'react';
import './DateSidebar.css';

// Helper to get auth token for API calls
const getAuthHeaders = () => {
  const token = localStorage.getItem('token');
  return token ? { 'Authorization': `Bearer ${token}` } : {};
};

const DateSidebar = ({ onDocumentSelect, selectedDateKey }) => {
  const [state, setState] = useState({
    timeline: {},
    expandedItems: new Set(),
    groupBy: 'month',
    loading: true,
    error: null
  });

  useEffect(() => {
    const fetchTimeline = async () => {
      try {
        setState(prev => ({ ...prev, loading: true }));
        // SECURITY: Include auth headers for consistency
        const response = await fetch(`/api/documents/timeline?group_by=${state.groupBy}`, {
          headers: getAuthHeaders()
        });
        if (!response.ok) throw new Error('Failed to fetch timeline');
        
        const data = await response.json();
        setState(prev => ({ 
          ...prev, 
          timeline: data.timeline, 
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
    };
    
    fetchTimeline();
  }, [state.groupBy]);

  const toggleExpanded = (key) => {
    const newExpanded = new Set(state.expandedItems);
    if (newExpanded.has(key)) {
      newExpanded.delete(key);
    } else {
      newExpanded.add(key);
    }
    setState(prev => ({ ...prev, expandedItems: newExpanded }));
  };

  const handleItemClick = (item) => {
    if (item.children && Object.keys(item.children).length > 0) {
      toggleExpanded(item.key);
    } else {
      onDocumentSelect?.(item.key);
    }
  };

  const renderTimelineItem = (item, level = 0) => {
    const isExpanded = state.expandedItems.has(item.key);
    const hasChildren = item.children && Object.keys(item.children).length > 0;
    const isSelected = selectedDateKey === item.key;

    return (
      <div key={item.key} className="timeline-item">
        <div 
          className={`timeline-item-content level-${level} ${isSelected ? 'selected' : ''}`}
          onClick={() => handleItemClick(item)}
          style={{ paddingLeft: `${level * 20 + 12}px` }}
        >
          {hasChildren && (
            <span className={`expand-icon ${isExpanded ? 'expanded' : ''}`}>
              ▶
            </span>
          )}
          <span className="timeline-label">{item.label}</span>
          <span className="timeline-count">({item.count})</span>
        </div>

        {hasChildren && isExpanded && (
          <div className="timeline-children">
            {Object.values(item.children)
              .sort((a, b) => b.key.localeCompare(a.key))
              .map(child => renderTimelineItem(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  if (state.loading) {
    return (
      <div className="date-sidebar">
        <div className="sidebar-header">
          <h3>문서 탐색</h3>
        </div>
        <div className="sidebar-content">
          <div className="loading">로딩 중...</div>
        </div>
      </div>
    );
  }

  if (state.error) {
    return (
      <div className="date-sidebar">
        <div className="sidebar-header">
          <h3>문서 탐색</h3>
        </div>
        <div className="sidebar-content">
          <div className="error">오류: {state.error}</div>
        </div>
      </div>
    );
  }

  return (
    <div className="date-sidebar">
      <div className="sidebar-header">
        <h3>문서 탐색</h3>
        <div className="view-controls">
          <select 
            value={state.groupBy} 
            onChange={(e) => setState(prev => ({ ...prev, groupBy: e.target.value }))}
            className="group-by-select"
          >
            <option value="day">일별</option>
            <option value="week">주별</option>
            <option value="month">월별</option>
            <option value="year">년별</option>
          </select>
        </div>
      </div>

      <div className="sidebar-content">
        {Object.keys(state.timeline).length === 0 ? (
          <div className="no-documents">문서가 없습니다</div>
        ) : (
          <div className="timeline-list">
            {Object.values(state.timeline)
              .sort((a, b) => b.key.localeCompare(a.key))
              .map(item => renderTimelineItem(item))}
          </div>
        )}
      </div>
    </div>
  );
};

export default DateSidebar;