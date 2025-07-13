import React, { useState, useEffect } from 'react';
import { documentService } from '../services/api';
import './Settings.css';

const Settings = () => {
  const [gitConfig, setGitConfig] = useState({
    username: '',
    email: '',
    repository: 'https://github.com/hephaex/mark'
  });
  const [exportStatus, setExportStatus] = useState(null);
  const [gitStatus, setGitStatus] = useState(null);
  const [exporting, setExporting] = useState(false);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    loadGitConfig();
  }, []);

  const loadGitConfig = async () => {
    try {
      // TODO: Load actual git configuration from backend
      setGitConfig({
        username: 'hephaex',
        email: 'hephaex@example.com',
        repository: 'https://github.com/hephaex/mark'
      });
    } catch (error) {
      console.error('Error loading git config:', error);
    }
  };

  const handleExportAll = async (shortFilename = false) => {
    try {
      setExporting(true);
      const response = await documentService.exportAllDocuments(shortFilename);
      
      setExportStatus({
        type: 'success',
        message: `Export completed: ${response.results.exported} documents exported to backup folder`
      });
    } catch (error) {
      setExportStatus({
        type: 'error',
        message: 'Export failed: ' + error.message
      });
    } finally {
      setExporting(false);
    }
  };

  const handleGitPush = async () => {
    try {
      setSyncing(true);
      // TODO: Implement actual git push functionality
      await new Promise(resolve => setTimeout(resolve, 2000)); // Simulate API call
      
      setGitStatus({
        type: 'success',
        message: 'Successfully pushed changes to repository'
      });
    } catch (error) {
      setGitStatus({
        type: 'error',
        message: 'Git push failed: ' + error.message
      });
    } finally {
      setSyncing(false);
    }
  };

  const handleGitPull = async () => {
    try {
      setSyncing(true);
      // TODO: Implement actual git pull functionality
      await new Promise(resolve => setTimeout(resolve, 2000)); // Simulate API call
      
      setGitStatus({
        type: 'success',
        message: 'Successfully pulled changes from repository'
      });
    } catch (error) {
      setGitStatus({
        type: 'error',
        message: 'Git pull failed: ' + error.message
      });
    } finally {
      setSyncing(false);
    }
  };

  const clearExportStatus = () => {
    setExportStatus(null);
  };

  const clearGitStatus = () => {
    setGitStatus(null);
  };

  return (
    <div className="settings">
      <div className="settings-header">
        <h2>Settings</h2>
      </div>

      {/* Git Configuration Section */}
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

        {/* Git Status Messages */}
        {gitStatus && (
          <div className={`status-message ${gitStatus.type}`}>
            <span>{gitStatus.message}</span>
            <button className="close-btn" onClick={clearGitStatus}>×</button>
          </div>
        )}

        <div className="git-actions">
          <button 
            className="btn btn-primary" 
            onClick={handleGitPull}
            disabled={syncing}
          >
            <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/>
            </svg>
            {syncing ? 'Pulling...' : 'Git Pull'}
          </button>
          <button 
            className="btn btn-primary" 
            onClick={handleGitPush}
            disabled={syncing}
          >
            <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.012 8.012 0 0 0 16 8c0-4.42-3.58-8-8-8z"/>
            </svg>
            {syncing ? 'Pushing...' : 'Git Push'}
          </button>
        </div>
      </div>

      {/* Export Section */}
      <div className="settings-section">
        <h3>Document Export</h3>
        <p>Export all documents from the database to backup folder</p>

        {/* Export Status Messages */}
        {exportStatus && (
          <div className={`status-message ${exportStatus.type}`}>
            <span>{exportStatus.message}</span>
            <button className="close-btn" onClick={clearExportStatus}>×</button>
          </div>
        )}

        <div className="export-actions">
          <button 
            className="btn btn-secondary" 
            onClick={() => handleExportAll(false)}
            disabled={exporting}
          >
            <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5z"/>
              <path d="M7.646 11.854a.5.5 0 0 0 .708 0l3-3a.5.5 0 0 0-.708-.708L8.5 10.293V1.5a.5.5 0 0 0-1 0v8.793L5.354 8.146a.5.5 0 1 0-.708.708l3 3z"/>
            </svg>
            {exporting ? 'Exporting...' : 'Export All Documents'}
          </button>
          <button 
            className="btn btn-secondary" 
            onClick={() => handleExportAll(true)}
            disabled={exporting}
          >
            <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5z"/>
              <path d="M7.646 11.854a.5.5 0 0 0 .708 0l3-3a.5.5 0 0 0-.708-.708L8.5 10.293V1.5a.5.5 0 0 0-1 0v8.793L5.354 8.146a.5.5 0 1 0-.708.708l3 3z"/>
            </svg>
            {exporting ? 'Exporting...' : 'Export (Short Names)'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;