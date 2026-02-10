import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { tagService } from '../services/api';
import SearchBar from '../components/SearchBar';
import Pagination from '../components/Pagination';
import { logError } from '../utils/logger';
import './TagList.css';

const TagList = () => {
  const [tags, setTags] = useState([]);
  const [statistics, setStatistics] = useState(null);
  const [pagination, setPagination] = useState({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentPage, setCurrentPage] = useState(1);
  const [viewMode, setViewMode] = useState('all'); // 'all', 'popular', 'auto-generated'

  useEffect(() => {
    fetchTags(1, searchQuery);
    fetchStatistics();
  }, [searchQuery, viewMode]);

  const fetchTags = async (page = 1, search = '') => {
    try {
      setLoading(true);
      let data;
      
      if (viewMode === 'popular') {
        data = await tagService.getTags(page, 20, search, true);
      } else {
        data = await tagService.getTags(page, 20, search, false);
      }
      
      setTags(data.tags);
      setPagination(data.pagination || {});
      setCurrentPage(page);
      setError(null);
    } catch (err) {
      setError('Failed to fetch tags');
      logError('TagList.fetchTags', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchStatistics = async () => {
    try {
      const stats = await tagService.getTagStatistics();
      setStatistics(stats);
    } catch (err) {
      logError('TagList.fetchStatistics', err);
    }
  };

  const handleSearch = (query) => {
    setSearchQuery(query);
    setCurrentPage(1);
  };

  const handlePageChange = (page) => {
    fetchTags(page, searchQuery);
  };

  const handleViewModeChange = (mode) => {
    setViewMode(mode);
    setCurrentPage(1);
  };

  const getTagsByType = () => {
    if (viewMode === 'auto-generated') {
      return tags.filter(tag => !tag.description);
    }
    return tags;
  };

  const formatUsageCount = (count) => {
    if (count === 0) return 'Unused';
    if (count === 1) return '1 document';
    return `${count} documents`;
  };

  if (loading && !tags.length) {
    return <div className="loading">Loading tags...</div>;
  }

  if (error && !tags.length) {
    return <div className="error">{error}</div>;
  }

  const filteredTags = getTagsByType();

  return (
    <div className="tag-list">
      <div className="tag-list-header">
        <h2>Tags</h2>
        <div className="header-actions">
          <SearchBar onSearch={handleSearch} initialValue={searchQuery} placeholder="Search tags..." />
        </div>
      </div>

      {/* Statistics Dashboard */}
      {statistics && (
        <div className="tag-statistics">
          <div className="stats-grid">
            <div className="stat-card">
              <div className="stat-number">{statistics.total_tags}</div>
              <div className="stat-label">Total Tags</div>
            </div>
            <div className="stat-card">
              <div className="stat-number">{statistics.auto_generated_tags}</div>
              <div className="stat-label">Auto-generated</div>
            </div>
            <div className="stat-card">
              <div className="stat-number">{statistics.manual_tags}</div>
              <div className="stat-label">Manual Tags</div>
            </div>
          </div>

          {/* Popular Tags */}
          {statistics.popular_tags && statistics.popular_tags.length > 0 && (
            <div className="popular-tags-section">
              <h3>Most Popular Tags</h3>
              <div className="popular-tags">
                {statistics.popular_tags.slice(0, 8).map((tag) => (
                  <Link
                    key={tag.name}
                    to={`/tags/${tag.name.toLowerCase()}`}
                    className="popular-tag"
                    style={{ backgroundColor: tag.color }}
                  >
                    <span className="tag-name">{tag.name}</span>
                    <span className="tag-count">{tag.usage_count}</span>
                  </Link>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* View Mode Tabs */}
      <div className="view-mode-tabs">
        <button 
          className={`tab ${viewMode === 'all' ? 'active' : ''}`}
          onClick={() => handleViewModeChange('all')}
        >
          All Tags ({statistics?.total_tags || 0})
        </button>
        <button 
          className={`tab ${viewMode === 'popular' ? 'active' : ''}`}
          onClick={() => handleViewModeChange('popular')}
        >
          Popular
        </button>
        <button 
          className={`tab ${viewMode === 'auto-generated' ? 'active' : ''}`}
          onClick={() => handleViewModeChange('auto-generated')}
        >
          Auto-generated ({statistics?.auto_generated_tags || 0})
        </button>
      </div>

      {/* Tags List */}
      {filteredTags.length === 0 ? (
        <div className="no-tags">
          {searchQuery ? (
            <>
              <h3>No tags found</h3>
              <p>No tags match your search for "{searchQuery}"</p>
              <button className="btn btn-secondary" onClick={() => handleSearch('')}>
                Clear Search
              </button>
            </>
          ) : (
            <>
              <h3>No tags yet</h3>
              <p>Tags will appear here as you create documents or add them manually</p>
            </>
          )}
        </div>
      ) : (
        <>
          <div className="tags-grid">
            {filteredTags.map((tag) => (
              <div key={tag.slug} className="tag-card">
                <Link to={`/tags/${tag.slug}`} className="tag-link">
                  <div className="tag-header">
                    <div 
                      className="tag-color" 
                      style={{ backgroundColor: tag.color }}
                    ></div>
                    <h3 className="tag-name">{tag.name}</h3>
                  </div>
                  
                  <div className="tag-meta">
                    <span className="tag-usage">
                      {formatUsageCount(tag.usage_count || tag.document_count || 0)}
                    </span>
                    {!tag.description && (
                      <span className="tag-badge auto-generated">Auto-generated</span>
                    )}
                  </div>
                  
                  {tag.description && (
                    <p className="tag-description">{tag.description}</p>
                  )}
                  
                  <div className="tag-dates">
                    <small>Created: {new Date(tag.created_at).toLocaleDateString()}</small>
                  </div>
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
  );
};

export default TagList;