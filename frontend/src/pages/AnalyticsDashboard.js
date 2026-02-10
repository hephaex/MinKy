import React, { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import PropTypes from 'prop-types';
import api from '../services/api';
import { logError } from '../utils/logger';
import '../styles/AnalyticsDashboard.css';

const AnalyticsDashboard = () => {
  const [analytics, setAnalytics] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [timeRange, setTimeRange] = useState(30);

  useEffect(() => {
    fetchAnalytics();
  }, [timeRange]);

  const fetchAnalytics = async () => {
    try {
      setLoading(true);
      const response = await api.get('/analytics/dashboard');
      setAnalytics(response.data.data);
      setError(null);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load analytics');
      logError('AnalyticsDashboard.fetchAnalytics', err);
    } finally {
      setLoading(false);
    }
  };

  const StatCard = ({ title, value, subtitle, trend }) => (
    <div className="stat-card">
      <div className="stat-header">
        <h3>{title}</h3>
        {trend && <span className={`trend ${trend > 0 ? 'positive' : 'negative'}`}>
          {trend > 0 ? '↗' : '↘'} {Math.abs(trend)}%
        </span>}
      </div>
      <div className="stat-value">{value?.toLocaleString() || 0}</div>
      {subtitle && <div className="stat-subtitle">{subtitle}</div>}
    </div>
  );

  StatCard.propTypes = {
    title: PropTypes.string.isRequired,
    value: PropTypes.number,
    subtitle: PropTypes.string,
    trend: PropTypes.number
  };

  const ChartPlaceholder = ({ title, data }) => (
    <div className="chart-container">
      <h3>{title}</h3>
      <div className="chart-placeholder">
        <div className="chart-bars">
          {data?.slice(0, 10).map((item, index) => (
            <div key={index} className="chart-bar" style={{height: `${Math.max(item.count * 5, 10)}px`}}>
              <div className="bar-label">{item.name || item.date}</div>
              <div className="bar-value">{item.count}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );

  ChartPlaceholder.propTypes = {
    title: PropTypes.string.isRequired,
    data: PropTypes.arrayOf(
      PropTypes.shape({
        name: PropTypes.string,
        date: PropTypes.string,
        count: PropTypes.number.isRequired
      })
    )
  };

  if (loading) {
    return (
      <div className="analytics-dashboard">
        <div className="loading">Loading analytics...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="analytics-dashboard">
        <div className="error">
          <h2>Analytics Error</h2>
          <p>{error}</p>
          <button onClick={fetchAnalytics}>Retry</button>
        </div>
      </div>
    );
  }

  const stats = analytics?.dashboard_stats;
  const timeline = analytics?.activity_timeline;
  const engagement = analytics?.user_engagement;
  const content = analytics?.content_analytics;
  const performance = analytics?.performance_metrics;

  return (
    <div className="analytics-dashboard">
      <div className="dashboard-header">
        <h1>Analytics Dashboard</h1>
        <div className="dashboard-controls">
          <select 
            value={timeRange} 
            onChange={(e) => setTimeRange(parseInt(e.target.value))}
            className="time-range-select"
          >
            <option value={7}>Last 7 days</option>
            <option value={30}>Last 30 days</option>
            <option value={90}>Last 90 days</option>
          </select>
          <button onClick={fetchAnalytics} className="refresh-btn">
            Refresh
          </button>
        </div>
      </div>

      {/* Analytics Navigation */}
      <div className="analytics-navigation">
        <div className="nav-section">
          <Link to="/tags" className="nav-card">
            <div className="nav-icon">🏷️</div>
            <div className="nav-content">
              <h3>Tags</h3>
              <p>Manage and explore document tags</p>
            </div>
          </Link>
          <Link to="/categories" className="nav-card">
            <div className="nav-icon">📂</div>
            <div className="nav-content">
              <h3>Categories</h3>
              <p>Organize documents by categories</p>
            </div>
          </Link>
        </div>
      </div>

      {/* Overview Stats */}
      <div className="stats-grid">
        <StatCard 
          title="Total Documents" 
          value={stats?.overview?.total_documents}
          subtitle={`${stats?.recent_activity?.documents_last_30_days || 0} this month`}
          trend={performance?.growth_metrics?.growth_rate_percent}
        />
        <StatCard 
          title="Total Users" 
          value={stats?.overview?.total_users}
          subtitle="Registered users"
        />
        <StatCard 
          title="Tags" 
          value={stats?.overview?.total_tags}
          subtitle="Unique tags"
        />
        <StatCard 
          title="Comments" 
          value={stats?.overview?.total_comments}
          subtitle={`${stats?.recent_activity?.comments_last_30_days || 0} this month`}
        />
      </div>

      {/* Activity Timeline */}
      <div className="charts-grid">
        <ChartPlaceholder 
          title="Document Activity Timeline" 
          data={timeline} 
        />
        
        <ChartPlaceholder 
          title="Popular Tags" 
          data={stats?.top_tags} 
        />
      </div>

      {/* Content Analytics */}
      <div className="content-analytics">
        <h2>Content Analytics</h2>
        <div className="content-stats">
          <div className="content-metric">
            <h4>Average Document Length</h4>
            <span>{content?.document_metrics?.average_length?.toLocaleString() || 0} characters</span>
          </div>
          <div className="content-metric">
            <h4>Total Versions</h4>
            <span>{content?.document_metrics?.total_versions || 0}</span>
          </div>
          <div className="content-metric">
            <h4>Average Versions per Document</h4>
            <span>{content?.document_metrics?.avg_versions_per_document || 0}</span>
          </div>
          <div className="content-metric">
            <h4>Total Attachments</h4>
            <span>{content?.attachment_stats?.total_attachments || 0}</span>
          </div>
        </div>
      </div>

      {/* User Engagement */}
      <div className="user-engagement">
        <h2>User Engagement</h2>
        <div className="engagement-table">
          <table>
            <thead>
              <tr>
                <th>User</th>
                <th>Documents</th>
                <th>Comments</th>
              </tr>
            </thead>
            <tbody>
              {engagement?.slice(0, 10).map((user, index) => (
                <tr key={index}>
                  <td>{user.username}</td>
                  <td>{user.documents}</td>
                  <td>{user.comments}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Document Distribution */}
      <div className="document-distribution">
        <h2>Document Distribution</h2>
        <div className="distribution-stats">
          <div className="distribution-item">
            <span className="label">Public Documents</span>
            <span className="value">{stats?.overview?.published_documents || 0}</span>
            <div className="bar">
              <div 
                className="bar-fill public" 
                style={{
                  width: `${(stats?.overview?.published_documents / Math.max(stats?.overview?.total_documents, 1)) * 100}%`
                }}
              ></div>
            </div>
          </div>
          <div className="distribution-item">
            <span className="label">Private Documents</span>
            <span className="value">{stats?.overview?.private_documents || 0}</span>
            <div className="bar">
              <div 
                className="bar-fill private" 
                style={{
                  width: `${(stats?.overview?.private_documents / Math.max(stats?.overview?.total_documents, 1)) * 100}%`
                }}
              ></div>
            </div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="dashboard-footer">
        <p>Last updated: {analytics?.generated_at ? new Date(analytics.generated_at).toLocaleString() : 'Unknown'}</p>
      </div>
    </div>
  );
};

export default AnalyticsDashboard;