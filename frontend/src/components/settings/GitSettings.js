import React, { useState, useEffect, useCallback } from 'react';
import api from '../../services/api';

const GitIcon = () => (
  <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/>
  </svg>
);

const SyncIcon = () => (
  <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
    <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
    <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.42 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115l.094-.319z"/>
  </svg>
);

const DEFAULT_CONFIG = {
  username: '',
  email: '',
  repository: '',
};

const GitSettings = () => {
  const [gitConfig, setGitConfig] = useState(DEFAULT_CONFIG);
  const [gitStatus, setGitStatus] = useState(null);
  const [syncing, setSyncing] = useState(false);

  const loadGitConfig = useCallback(async () => {
    try {
      const response = await api.get('/git/config');
      if (response.data.success) {
        setGitConfig({
          username: response.data.config.username || DEFAULT_CONFIG.username,
          email: response.data.config.email || DEFAULT_CONFIG.email,
          repository: response.data.config.repository || DEFAULT_CONFIG.repository,
        });
      } else {
        setGitConfig(DEFAULT_CONFIG);
        setGitStatus({ type: 'warning', message: 'Could not load git configuration' });
      }
    } catch (error) {
      setGitConfig(DEFAULT_CONFIG);
    }
  }, []);

  useEffect(() => {
    loadGitConfig();
  }, [loadGitConfig]);

  const handleGitAction = async (action, message) => {
    try {
      setSyncing(true);
      const response = await api.post(`/git/${action}`, { message });
      if (response.data.success) {
        setGitStatus({ type: 'success', message: response.data.message || `Git ${action} successful` });
      } else {
        setGitStatus({ type: 'error', message: response.data.error || `Git ${action} failed` });
      }
    } catch (error) {
      setGitStatus({ type: 'error', message: `Git ${action} failed: ${error.response?.data?.error || error.message || 'Unknown error'}` });
    } finally {
      setSyncing(false);
    }
  };

  const clearStatus = () => setGitStatus(null);

  return (
    <div className="settings-section">
      <h3>Git Integration</h3>
      <div className="git-config">
        <div className="config-item">
          <label>Repository URL:</label>
          <div className="config-value">{gitConfig.repository}</div>
        </div>
        <div className="config-item">
          <label>Username:</label>
          <div className="config-value">{gitConfig.username}</div>
        </div>
        <div className="config-item">
          <label>Email:</label>
          <div className="config-value">{gitConfig.email}</div>
        </div>
      </div>

      {gitStatus && (
        <div className={`status-message ${gitStatus.type}`}>
          <span>{gitStatus.message}</span>
          <button className="close-btn" onClick={clearStatus}>x</button>
        </div>
      )}

      <div className="git-actions">
        <button className="btn btn-primary" onClick={() => handleGitAction('pull')} disabled={syncing}>
          <GitIcon />
          {syncing ? 'Pulling...' : 'Git Pull'}
        </button>
        <button className="btn btn-primary" onClick={() => handleGitAction('push', 'Auto-commit from minky web interface')} disabled={syncing}>
          <GitIcon />
          {syncing ? 'Pushing...' : 'Git Push'}
        </button>
        <button className="btn btn-success" onClick={() => handleGitAction('sync', 'Auto-sync from minky web interface')} disabled={syncing} title="Pull changes then push local changes">
          <SyncIcon />
          {syncing ? 'Syncing...' : 'Git Sync'}
        </button>
      </div>
    </div>
  );
};

export default GitSettings;
