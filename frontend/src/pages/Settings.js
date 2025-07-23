import React, { useState, useEffect } from 'react';
import { documentService } from '../services/api';
import { LanguageSelector } from '../i18n/i18n';
import api from '../services/api';
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
  
  // AI Settings State
  const [aiConfig, setAiConfig] = useState({
    ocrService: 'tesseract', // tesseract, google-vision, aws-textract
    ocrApiKey: '',
    llmProvider: 'openai', // openai, anthropic, google
    llmApiKey: '',
    llmModel: 'gpt-3.5-turbo',
    enableAiTags: true,
    enableAiSummary: false
  });
  const [aiStatus, setAiStatus] = useState(null);
  
  // Connection status for AI services
  const [connectionStatus, setConnectionStatus] = useState({
    ocr: null, // null = unknown, true = connected, false = disconnected, 'testing' = testing
    llm: null
  });

  useEffect(() => {
    loadGitConfig();
    loadAiConfig();
    performHealthCheck();
  }, []);

  const loadGitConfig = async () => {
    try {
      const response = await fetch('/api/git/config');
      const data = await response.json();
      
      if (data.success) {
        setGitConfig({
          username: data.config.username || 'hephaex',
          email: data.config.email || 'hephaex@example.com',
          repository: data.config.repository || 'https://github.com/hephaex/mark'
        });
      } else {
        // Fallback to default values if git config fails
        setGitConfig({
          username: 'hephaex',
          email: 'hephaex@example.com',
          repository: 'https://github.com/hephaex/mark'
        });
      }
    } catch (error) {
      console.error('Error loading git config:', error);
      // Fallback to default values
      setGitConfig({
        username: 'hephaex',
        email: 'hephaex@example.com',
        repository: 'https://github.com/hephaex/mark'
      });
    }
  };

  const loadAiConfig = async () => {
    try {
      const response = await api.get('/ai/config');
      
      if (response.data.success) {
        setAiConfig(response.data.config);
      } else {
        console.error('Failed to load AI config:', response.data.error);
      }
    } catch (error) {
      console.error('Error loading AI config:', error);
    }
  };

  const performHealthCheck = async () => {
    try {
      const response = await api.get('/ai/health');
      
      if (response.data.success) {
        setConnectionStatus(response.data.health);
      } else {
        console.error('Health check failed:', response.data.error);
      }
    } catch (error) {
      console.error('Error performing health check:', error);
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
      
      const response = await fetch('/api/git/push', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          message: 'Auto-commit from minky web interface'
        })
      });
      
      const data = await response.json();
      
      if (data.success) {
        setGitStatus({
          type: 'success',
          message: data.message || 'Successfully pushed changes to repository'
        });
      } else {
        setGitStatus({
          type: 'error',
          message: data.error || 'Git push failed'
        });
      }
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
      
      const response = await fetch('/api/git/pull', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        }
      });
      
      const data = await response.json();
      
      if (data.success) {
        setGitStatus({
          type: 'success',
          message: data.message || 'Successfully pulled changes from repository'
        });
      } else {
        setGitStatus({
          type: 'error',
          message: data.error || 'Git pull failed'
        });
      }
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

  const handleGitSync = async () => {
    try {
      setSyncing(true);
      
      const response = await fetch('/api/git/sync', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          message: 'Auto-sync from minky web interface'
        })
      });
      
      const data = await response.json();
      
      if (data.success) {
        setGitStatus({
          type: 'success',
          message: data.message || 'Successfully synced with repository'
        });
      } else {
        setGitStatus({
          type: 'error',
          message: data.error || 'Git sync failed'
        });
      }
    } catch (error) {
      setGitStatus({
        type: 'error',
        message: 'Git sync failed: ' + error.message
      });
    } finally {
      setSyncing(false);
    }
  };

  const clearGitStatus = () => {
    setGitStatus(null);
  };

  // AI Configuration Functions
  const handleAiConfigChange = (field, value) => {
    setAiConfig(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const saveAiConfig = async () => {
    try {
      const response = await api.post('/ai/config', aiConfig);
      
      if (response.data.success) {
        setAiStatus({
          type: 'success',
          message: 'AI configuration saved successfully'
        });
        // Reload configuration and perform health check after saving
        await loadAiConfig();
        await performHealthCheck();
      } else {
        setAiStatus({
          type: 'error',
          message: response.data.error || 'Failed to save AI configuration'
        });
      }
    } catch (error) {
      setAiStatus({
        type: 'error',
        message: 'Failed to save AI configuration: ' + (error.response?.data?.error || error.message)
      });
    }
  };

  const testAiConnection = async (service) => {
    try {
      // Set testing status
      setConnectionStatus(prev => ({
        ...prev,
        [service]: 'testing'
      }));
      
      const response = await api.post(`/ai/test/${service}`, aiConfig);
      
      if (response.data.success) {
        setConnectionStatus(prev => ({
          ...prev,
          [service]: true
        }));
        setAiStatus({
          type: 'success',
          message: response.data.message || `${service.toUpperCase()} connection test successful`
        });
      } else {
        setConnectionStatus(prev => ({
          ...prev,
          [service]: false
        }));
        setAiStatus({
          type: 'error',
          message: response.data.error || `${service.toUpperCase()} connection test failed`
        });
      }
    } catch (error) {
      setConnectionStatus(prev => ({
        ...prev,
        [service]: false
      }));
      setAiStatus({
        type: 'error',
        message: `${service.toUpperCase()} connection test failed: ` + (error.response?.data?.error || error.message)
      });
    }
  };

  const clearAiStatus = () => {
    setAiStatus(null);
  };

  // Helper function to render connection status icon
  const renderConnectionStatus = (service) => {
    const status = connectionStatus[service];
    
    switch (status) {
      case true:
        return (
          <span className="connection-status connected" title="Connected">
            <svg viewBox="0 0 16 16" fill="currentColor" className="status-icon">
              <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zm-3.97-3.03a.75.75 0 0 0-1.08.022L7.477 9.417 5.384 7.323a.75.75 0 0 0-1.06 1.061L6.97 11.03a.75.75 0 0 0 1.079-.02l3.992-4.99a.75.75 0 0 0-.01-1.05z"/>
            </svg>
            Connected
          </span>
        );
      case false:
        return (
          <span className="connection-status disconnected" title="Connection Failed">
            <svg viewBox="0 0 16 16" fill="currentColor" className="status-icon">
              <path d="M16 8A8 8 0 1 1 0 8a8 8 0 0 1 16 0zM5.354 4.646a.5.5 0 1 0-.708.708L7.293 8l-2.647 2.646a.5.5 0 0 0 .708.708L8 8.707l2.646 2.647a.5.5 0 0 0 .708-.708L8.707 8l2.647-2.646a.5.5 0 0 0-.708-.708L8 7.293 5.354 4.646z"/>
            </svg>
            Disconnected
          </span>
        );
      case 'testing':
        return (
          <span className="connection-status testing" title="Testing Connection">
            <svg viewBox="0 0 16 16" fill="currentColor" className="status-icon spinning">
              <path d="M8 0a8 8 0 0 1 7.74 6h-1.26A7 7 0 1 0 8 15v1a8 8 0 0 1 0-16z"/>
            </svg>
            Testing...
          </span>
        );
      default:
        return (
          <span className="connection-status unknown" title="Connection Status Unknown">
            <svg viewBox="0 0 16 16" fill="currentColor" className="status-icon">
              <path d="M8 15A7 7 0 1 1 8 1a7 7 0 0 1 0 14zm0 1A8 8 0 1 0 8 0a8 8 0 0 0 0 16z"/>
              <path d="M5.255 5.786a.237.237 0 0 0 .241.247h.825c.138 0 .248-.113.266-.25.09-.656.54-1.134 1.342-1.134.686 0 1.314.343 1.314 1.168 0 .635-.374.927-.965 1.371-.673.489-1.206 1.06-1.168 1.987l.003.217a.25.25 0 0 0 .25.246h.811a.25.25 0 0 0 .25-.25v-.105c0-.718.273-.927 1.01-1.486.609-.463 1.244-.977 1.244-2.056 0-1.511-1.276-2.241-2.673-2.241-1.267 0-2.655.59-2.75 2.286zm1.557 5.763c0 .533.425.927 1.01.927.609 0 1.028-.394 1.028-.927 0-.552-.42-.94-1.029-.94-.584 0-1.009.388-1.009.94z"/>
            </svg>
            Unknown
          </span>
        );
    }
  };

  return (
    <div className="settings">
      <div className="settings-header">
        <h2>Settings</h2>
      </div>

      {/* Language Settings Section */}
      <div className="settings-section">
        <h3>Language Settings</h3>
        <p>Select your preferred language for the interface</p>
        <div className="language-setting">
          <label>Interface Language:</label>
          <LanguageSelector />
        </div>
      </div>

      {/* AI Settings Section */}
      <div className="settings-section">
        <h3>AI Configuration</h3>
        <p>Configure AI services for OCR and LLM features</p>
        
        {/* AI Status Messages */}
        {aiStatus && (
          <div className={`status-message ${aiStatus.type}`}>
            <span>{aiStatus.message}</span>
            <button className="close-btn" onClick={clearAiStatus}>×</button>
          </div>
        )}

        {/* OCR Configuration */}
        <div className="ai-config-group">
          <div className="ai-config-header">
            <h4>📷 OCR Service Configuration</h4>
            {renderConnectionStatus('ocr')}
          </div>
          <div className="config-item">
            <label>OCR Provider:</label>
            <select 
              value={aiConfig.ocrService} 
              onChange={(e) => handleAiConfigChange('ocrService', e.target.value)}
            >
              <option value="tesseract">Tesseract (Local)</option>
              <option value="google-vision">Google Vision API</option>
              <option value="aws-textract">AWS Textract</option>
            </select>
          </div>
          
          {aiConfig.ocrService !== 'tesseract' && (
            <div className="config-item">
              <label>OCR API Key:</label>
              <input
                type="password"
                value={aiConfig.ocrApiKey}
                onChange={(e) => handleAiConfigChange('ocrApiKey', e.target.value)}
                placeholder="Enter API key for OCR service"
              />
            </div>
          )}
          
          <button 
            className="btn btn-secondary"
            onClick={() => testAiConnection('ocr')}
            disabled={connectionStatus.ocr === 'testing'}
          >
            {connectionStatus.ocr === 'testing' ? 'Testing...' : 'Test OCR Connection'}
          </button>
        </div>

        {/* LLM Configuration */}
        <div className="ai-config-group">
          <div className="ai-config-header">
            <h4>🤖 LLM Service Configuration</h4>
            {renderConnectionStatus('llm')}
          </div>
          <div className="config-item">
            <label>LLM Provider:</label>
            <select 
              value={aiConfig.llmProvider} 
              onChange={(e) => handleAiConfigChange('llmProvider', e.target.value)}
            >
              <option value="openai">OpenAI</option>
              <option value="anthropic">Anthropic (Claude)</option>
              <option value="google">Google (Gemini)</option>
            </select>
          </div>
          
          <div className="config-item">
            <label>API Key:</label>
            <input
              type="password"
              value={aiConfig.llmApiKey}
              onChange={(e) => handleAiConfigChange('llmApiKey', e.target.value)}
              placeholder="Enter API key for LLM service"
            />
          </div>
          
          <div className="config-item">
            <label>Model:</label>
            <select 
              value={aiConfig.llmModel} 
              onChange={(e) => handleAiConfigChange('llmModel', e.target.value)}
            >
              {aiConfig.llmProvider === 'openai' && (
                <>
                  <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
                  <option value="gpt-4">GPT-4</option>
                  <option value="gpt-4-turbo">GPT-4 Turbo</option>
                </>
              )}
              {aiConfig.llmProvider === 'anthropic' && (
                <>
                  <option value="claude-3-haiku">Claude 3 Haiku</option>
                  <option value="claude-3-sonnet">Claude 3 Sonnet</option>
                  <option value="claude-3-opus">Claude 3 Opus</option>
                </>
              )}
              {aiConfig.llmProvider === 'google' && (
                <>
                  <option value="gemini-pro">Gemini Pro</option>
                  <option value="gemini-pro-vision">Gemini Pro Vision</option>
                </>
              )}
            </select>
          </div>
          
          <button 
            className="btn btn-secondary"
            onClick={() => testAiConnection('llm')}
            disabled={connectionStatus.llm === 'testing'}
          >
            {connectionStatus.llm === 'testing' ? 'Testing...' : 'Test LLM Connection'}
          </button>
        </div>

        {/* AI Features Configuration */}
        <div className="ai-config-group">
          <h4>🎯 AI Features</h4>
          <div className="config-item checkbox-item">
            <label>
              <input
                type="checkbox"
                checked={aiConfig.enableAiTags}
                onChange={(e) => handleAiConfigChange('enableAiTags', e.target.checked)}
              />
              Enable AI-powered automatic tagging
            </label>
          </div>
          
          <div className="config-item checkbox-item">
            <label>
              <input
                type="checkbox"
                checked={aiConfig.enableAiSummary}
                onChange={(e) => handleAiConfigChange('enableAiSummary', e.target.checked)}
              />
              Enable AI-powered document summaries
            </label>
          </div>
        </div>

        <div className="ai-actions">
          <button 
            className="btn btn-primary"
            onClick={saveAiConfig}
          >
            💾 Save AI Configuration
          </button>
        </div>
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
          <button 
            className="btn btn-success" 
            onClick={handleGitSync}
            disabled={syncing}
            title="Pull changes then push local changes"
          >
            <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 4.754a3.246 3.246 0 1 0 0 6.492 3.246 3.246 0 0 0 0-6.492zM5.754 8a2.246 2.246 0 1 1 4.492 0 2.246 2.246 0 0 1-4.492 0z"/>
              <path d="M9.796 1.343c-.527-1.79-3.065-1.79-3.592 0l-.094.319a.873.873 0 0 1-1.255.52l-.292-.16c-1.64-.892-3.433.902-2.54 2.541l.159.292a.873.873 0 0 1-.52 1.255l-.319.094c-1.79.527-1.79 3.065 0 3.592l.319.094a.873.873 0 0 1 .52 1.255l-.16.292c-.892 1.64.901 3.434 2.541 2.54l.292-.159a.873.873 0 0 1 1.255.52l.094.319c.527 1.79 3.065 1.79 3.592 0l.094-.319a.873.873 0 0 1 1.255-.52l.292.16c1.64.893 3.434-.902 2.54-2.541l-.159-.292a.873.873 0 0 1 .52-1.255l.319-.094c1.79-.527 1.79-3.065 0-3.592l-.319-.094a.873.873 0 0 1-.52-1.255l.16-.292c.893-1.64-.902-3.433-2.541-2.54l-.292.159a.873.873 0 0 1-1.255-.52l-.094-.319zm-2.633.283c.246-.835 1.428-.835 1.674 0l.094.319a1.873 1.873 0 0 0 2.693 1.115l.291-.16c.764-.415 1.6.42 1.184 1.185l-.159.292a1.873 1.873 0 0 0 1.116 2.692l.318.094c.835.246.835 1.428 0 1.674l-.319.094a1.873 1.873 0 0 0-1.115 2.693l.16.291c.415.764-.42 1.6-1.185 1.184l-.291-.159a1.873 1.873 0 0 0-2.693 1.116l-.094.318c-.246.835-1.428.835-1.674 0l-.094-.319a1.873 1.873 0 0 0-2.692-1.115l-.292.16c-.764.415-1.6-.42-1.184-1.185l.159-.291A1.873 1.873 0 0 0 1.945 8.93l-.319-.094c-.835-.246-.835-1.428 0-1.674l.319-.094A1.873 1.873 0 0 0 3.06 4.377l-.16-.292c-.415-.764.42-1.6 1.185-1.184l.292.159a1.873 1.873 0 0 0 2.692-1.115l.094-.319z"/>
            </svg>
            {syncing ? 'Syncing...' : 'Git Sync'}
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