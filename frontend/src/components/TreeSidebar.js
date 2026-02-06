import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { documentService } from '../services/api';
import TreeView from './TreeView';
import './TreeView.css';
import './DocumentsSidebar.css';

const MODES = [
  { key: 'by-tag', label: '태그' },
  { key: 'by-date', label: '날짜' },
];

const TreeSidebar = ({ isVisible, onToggle }) => {
  const navigate = useNavigate();
  const [mode, setMode] = useState('by-tag');
  const [treeData, setTreeData] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');

  const fetchTree = useCallback(async (currentMode) => {
    try {
      setLoading(true);
      setError(null);
      const data = await documentService.getDocumentTree(currentMode);
      setTreeData(data.tree || []);
    } catch (err) {
      setError('트리 데이터를 불러올 수 없습니다');
      setTreeData([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isVisible) {
      fetchTree(mode);
    }
  }, [isVisible, mode, fetchTree]);

  const handleModeChange = useCallback((newMode) => {
    setMode(newMode);
    setSearchQuery('');
  }, []);

  const handleSelect = useCallback((node) => {
    if (node.documentId) {
      navigate(`/documents/${node.documentId}`);
    }
  }, [navigate]);

  const handleNewDocument = useCallback(() => {
    navigate('/documents/new');
  }, [navigate]);

  const filteredTree = useMemo(() => {
    if (!searchQuery.trim()) {
      return treeData;
    }

    const query = searchQuery.toLowerCase();

    const filterNode = (node) => {
      if (node.type === 'document') {
        return node.label.toLowerCase().includes(query) ? { ...node } : null;
      }

      const filteredChildren = (node.children || [])
        .map(filterNode)
        .filter(Boolean);

      if (filteredChildren.length > 0 || node.label.toLowerCase().includes(query)) {
        return {
          ...node,
          children: filteredChildren,
          count: filteredChildren.filter((c) => c.type === 'document').length +
                 filteredChildren.reduce((sum, c) => sum + (c.count || 0), 0)
        };
      }

      return null;
    };

    return treeData.map(filterNode).filter(Boolean);
  }, [treeData, searchQuery]);

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

        {/* Mode Switcher */}
        <div className="sidebar-section">
          <div className="tree-mode-tabs">
            {MODES.map((m) => (
              <button
                key={m.key}
                className={`tree-mode-tab ${mode === m.key ? 'active' : ''}`}
                onClick={() => handleModeChange(m.key)}
              >
                {m.label}
              </button>
            ))}
          </div>
        </div>

        {/* Search Filter */}
        <div className="sidebar-section">
          <input
            type="text"
            className="tree-search-input"
            placeholder="문서 검색..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>

        {/* Tree Content */}
        <div className="sidebar-section tree-section">
          {loading ? (
            <div className="loading-indicator">로딩 중...</div>
          ) : error ? (
            <div className="empty-state">{error}</div>
          ) : (
            <TreeView
              nodes={filteredTree}
              onSelect={handleSelect}
            />
          )}
        </div>
      </div>
    </div>
  );
};

export default TreeSidebar;
