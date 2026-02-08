import React from 'react';

const StatCard = ({ title, value, subtitle, className = '' }) => (
  <div className={`admin-stat-card ${className}`}>
    <h3>{title}</h3>
    <div className="stat-value">{value?.toLocaleString() || 0}</div>
    {subtitle && <div className="stat-subtitle">{subtitle}</div>}
  </div>
);

const AdminOverview = ({ systemStats }) => {
  if (!systemStats) {
    return null;
  }

  return (
    <div className="overview-tab">
      <h2>System Overview</h2>

      <div className="stats-grid">
        <StatCard
          title="Users"
          value={systemStats.users?.total}
          subtitle={`${systemStats.users?.active} active, ${systemStats.users?.admins} admins`}
          className="users-stat"
        />
        <StatCard
          title="Documents"
          value={systemStats.content?.documents}
          subtitle={`${systemStats.content?.public_documents} public`}
          className="documents-stat"
        />
        <StatCard
          title="Tags"
          value={systemStats.content?.tags}
          subtitle="Unique tags"
          className="tags-stat"
        />
        <StatCard
          title="Comments"
          value={systemStats.content?.comments}
          subtitle="Total comments"
          className="comments-stat"
        />
      </div>

      <div className="activity-section">
        <h3>Recent Activity (This Week)</h3>
        <div className="activity-stats">
          <div className="activity-item">
            <span className="activity-label">New Users</span>
            <span className="activity-value">{systemStats.users?.new_this_week}</span>
          </div>
          <div className="activity-item">
            <span className="activity-label">New Documents</span>
            <span className="activity-value">{systemStats.content?.new_documents_week}</span>
          </div>
          <div className="activity-item">
            <span className="activity-label">New Comments</span>
            <span className="activity-value">{systemStats.content?.new_comments_week}</span>
          </div>
        </div>
      </div>

      <div className="storage-section">
        <h3>Storage Information</h3>
        <div className="storage-stats">
          <div className="storage-item">
            <span className="storage-label">Estimated Storage</span>
            <span className="storage-value">
              {(systemStats.storage?.estimated_kb / 1024).toFixed(2)} MB
            </span>
          </div>
          <div className="storage-item">
            <span className="storage-label">Average Document Size</span>
            <span className="storage-value">
              {(systemStats.storage?.avg_document_size / 1024).toFixed(2)} KB
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default AdminOverview;
