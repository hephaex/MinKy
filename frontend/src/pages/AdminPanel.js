import React, { useState, useEffect } from 'react';
import api from '../services/api';
import {
  AdminOverview,
  AdminUsers,
  AdminDocuments,
  AdminMaintenance,
  AdminTabs
} from '../components/admin';
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
    } finally {
      setLoading(false);
    }
  };

  const updateUser = async (userId, updates) => {
    try {
      await api.put(`/admin/users/${userId}`, updates);
      fetchUsers();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to update user');
    }
  };

  const performCleanup = async (type) => {
    try {
      setLoading(true);
      const response = await api.post('/admin/system/cleanup', { type });
      alert(`Cleanup completed: ${JSON.stringify(response.data.results)}`);
      fetchSystemStats();
    } catch (err) {
      setError(err.response?.data?.error || 'Failed to perform cleanup');
    } finally {
      setLoading(false);
    }
  };

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
        <AdminTabs activeTab={activeTab} onTabChange={setActiveTab} />
      </div>

      {error && (
        <div className="error-message">
          <p>{error}</p>
          <button onClick={() => setError(null)}>Dismiss</button>
        </div>
      )}

      <div className="admin-content">
        {activeTab === 'overview' && (
          <AdminOverview systemStats={systemStats} />
        )}

        {activeTab === 'users' && (
          <AdminUsers
            users={users}
            currentPage={userPage}
            onPageChange={setUserPage}
            onUserUpdate={updateUser}
          />
        )}

        {activeTab === 'documents' && (
          <AdminDocuments
            documents={documents}
            currentPage={documentPage}
            onPageChange={setDocumentPage}
          />
        )}

        {activeTab === 'maintenance' && (
          <AdminMaintenance onCleanup={performCleanup} />
        )}
      </div>
    </div>
  );
};

export default AdminPanel;
