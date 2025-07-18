import React, { useState, useEffect } from 'react';
import api from '../services/api';
import '../styles/AdminPanel.css';

const AdminPanel = () => {
  const [activeTab, setActiveTab] = useState('overview');
  const [users, setUsers] = useState([]);
  const [documents, setDocuments] = useState([]);
  const [systemStats, setSystemStats] = useState(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);
  const [userPage, setUserPage] = useState(1);
  const [documentPage, setDocumentPage] = useState(1);

  useEffect(() => {
    if (activeTab === 'overview') {
      fetchSystemStats();
    } else if (activeTab === 'users') {
      fetchUsers();
    } else if (activeTab === 'documents') {
      fetchDocuments();
    }
  }, [activeTab, userPage, documentPage]);

  const fetchSystemStats = async () => {
    try {
      setLoading(true);
      const response = await api.get('/admin/system/stats');
      setSystemStats(response.data.stats);
      setError(null);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load system stats');
      console.error('System stats error:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchUsers = async () => {
    try {
      setLoading(true);
      const response = await api.get(`/admin/users?page=${userPage}&per_page=20`);
      setUsers(response.data.users);
      setError(null);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load users');
      console.error('Users fetch error:', err);
    } finally {
      setLoading(false);
    }
  };

  const fetchDocuments = async () => {
    try {
      setLoading(true);
      const response = await api.get(`/admin/documents?page=${documentPage}&per_page=20`);
      setDocuments(response.data.documents);
      setError(null);
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to load documents');
      console.error('Documents fetch error:', err);
    } finally {
      setLoading(false);
    }
  };

  const updateUser = async (userId, updates) => {
    try {
      await api.put(`/admin/users/${userId}`, updates);
      fetchUsers(); // Refresh the list
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to update user');
      console.error('User update error:', err);
    }
  };

  const performCleanup = async (type) => {
    try {
      setLoading(true);
      const response = await api.post('/admin/system/cleanup', { type });
      alert(`Cleanup completed: ${JSON.stringify(response.data.results)}`);
      fetchSystemStats(); // Refresh stats
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to perform cleanup');
      console.error('Cleanup error:', err);
    } finally {
      setLoading(false);
    }
  };

  const TabButton = ({ tab, label, active, onClick }) => (
    <button
      className={`tab-button ${active ? 'active' : ''}`}
      onClick={onClick}
    >
      {label}
    </button>
  );

  const StatCard = ({ title, value, subtitle, className = '' }) => (
    <div className={`admin-stat-card ${className}`}>
      <h3>{title}</h3>
      <div className="stat-value">{value?.toLocaleString() || 0}</div>
      {subtitle && <div className="stat-subtitle">{subtitle}</div>}
    </div>
  );

  const UserRow = ({ user, onUpdate }) => (
    <tr>
      <td>{user.username}</td>
      <td>{user.email}</td>
      <td>{user.full_name || 'N/A'}</td>
      <td>
        <span className={`status-badge ${user.is_active ? 'active' : 'inactive'}`}>
          {user.is_active ? 'Active' : 'Inactive'}
        </span>
      </td>
      <td>
        <span className={`admin-badge ${user.is_admin ? 'admin' : 'user'}`}>
          {user.is_admin ? 'Admin' : 'User'}
        </span>
      </td>
      <td>{user.document_count || 0}</td>
      <td>{user.comment_count || 0}</td>
      <td>
        <div className="user-actions">
          <button
            className="btn-small"
            onClick={() => onUpdate(user.id, { is_active: !user.is_active })}
          >
            {user.is_active ? 'Deactivate' : 'Activate'}
          </button>
          <button
            className="btn-small"
            onClick={() => onUpdate(user.id, { is_admin: !user.is_admin })}
          >
            {user.is_admin ? 'Remove Admin' : 'Make Admin'}
          </button>
        </div>
      </td>
    </tr>
  );

  if (loading && !systemStats && !users.length && !documents.length) {
    return (
      <div className="admin-panel">
        <div className="loading">Loading admin panel...</div>
      </div>
    );
  }

  return (
    <div className="admin-panel">
      <div className="admin-header">
        <h1>Admin Panel</h1>
        <div className="admin-tabs">
          <TabButton
            tab="overview"
            label="Overview"
            active={activeTab === 'overview'}
            onClick={() => setActiveTab('overview')}
          />
          <TabButton
            tab="users"
            label="Users"
            active={activeTab === 'users'}
            onClick={() => setActiveTab('users')}
          />
          <TabButton
            tab="documents"
            label="Documents"
            active={activeTab === 'documents'}
            onClick={() => setActiveTab('documents')}
          />
          <TabButton
            tab="maintenance"
            label="Maintenance"
            active={activeTab === 'maintenance'}
            onClick={() => setActiveTab('maintenance')}
          />
        </div>
      </div>

      {error && (
        <div className="error-message">
          <p>{error}</p>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="admin-content">
        {activeTab === 'overview' && systemStats && (
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
                  <span className="storage-value">{(systemStats.storage?.estimated_kb / 1024).toFixed(2)} MB</span>
                </div>
                <div className="storage-item">
                  <span className="storage-label">Average Document Size</span>
                  <span className="storage-value">{(systemStats.storage?.avg_document_size / 1024).toFixed(2)} KB</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'users' && (
          <div className="users-tab">
            <h2>User Management</h2>
            
            <div className="table-container">
              <table className="admin-table">
                <thead>
                  <tr>
                    <th>Username</th>
                    <th>Email</th>
                    <th>Full Name</th>
                    <th>Status</th>
                    <th>Role</th>
                    <th>Documents</th>
                    <th>Comments</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {users.map(user => (
                    <UserRow
                      key={user.id}
                      user={user}
                      onUpdate={updateUser}
                    />
                  ))}
                </tbody>
              </table>
            </div>

            <div className="pagination">
              <button
                disabled={userPage === 1}
                onClick={() => setUserPage(userPage - 1)}
              >
                Previous
              </button>
              <span>Page {userPage}</span>
              <button
                onClick={() => setUserPage(userPage + 1)}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {activeTab === 'documents' && (
          <div className="documents-tab">
            <h2>Document Management</h2>
            
            <div className="documents-list">
              {documents.map(doc => (
                <div key={doc.id} className="document-card">
                  <div className="document-header">
                    <h3>{doc.title}</h3>
                    <span className={`visibility-badge ${doc.is_public ? 'public' : 'private'}`}>
                      {doc.is_public ? 'Public' : 'Private'}
                    </span>
                  </div>
                  <div className="document-meta">
                    <p>Author: {doc.owner?.username || 'Unknown'}</p>
                    <p>Created: {new Date(doc.created_at).toLocaleDateString()}</p>
                    <p>Updated: {new Date(doc.updated_at).toLocaleDateString()}</p>
                    <p>Tags: {doc.tags?.map(tag => tag.name).join(', ') || 'None'}</p>
                  </div>
                </div>
              ))}
            </div>

            <div className="pagination">
              <button
                disabled={documentPage === 1}
                onClick={() => setDocumentPage(documentPage - 1)}
              >
                Previous
              </button>
              <span>Page {documentPage}</span>
              <button
                onClick={() => setDocumentPage(documentPage + 1)}
              >
                Next
              </button>
            </div>
          </div>
        )}

        {activeTab === 'maintenance' && (
          <div className="maintenance-tab">
            <h2>System Maintenance</h2>
            
            <div className="maintenance-section">
              <h3>Cleanup Operations</h3>
              <div className="cleanup-actions">
                <button
                  className="cleanup-btn"
                  onClick={() => performCleanup('orphaned_attachments')}
                >
                  Remove Orphaned Attachments
                </button>
                <button
                  className="cleanup-btn"
                  onClick={() => performCleanup('old_versions')}
                >
                  Clean Old Versions
                </button>
                <button
                  className="cleanup-btn danger"
                  onClick={() => performCleanup('all')}
                >
                  Full Cleanup
                </button>
              </div>
            </div>

            <div className="maintenance-section">
              <h3>System Information</h3>
              <div className="system-info">
                <p>System Status: <span className="status-good">Healthy</span></p>
                <p>Last Cleanup: Never</p>
                <p>Database Size: Calculating...</p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AdminPanel;