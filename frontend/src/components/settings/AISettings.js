import React, { useState, useEffect, useCallback } from 'react';
import api from '../../services/api';

const ConnectionStatusIcon = ({ status }) => {
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

const OCR_PROVIDERS = [
  { value: 'tesseract', label: 'Tesseract (Local)' },
  { value: 'google-vision', label: 'Google Vision API' },
  { value: 'aws-textract', label: 'AWS Textract' },
];

const LLM_PROVIDERS = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic (Claude)' },
  { value: 'google', label: 'Google (Gemini)' },
  { value: 'local', label: 'Local LLM' },
];

const LLM_MODELS = {
  openai: ['gpt-4o', 'gpt-4-turbo', 'gpt-4'],
  anthropic: ['claude-sonnet-4', 'claude-opus-4'],
  google: ['gemini-pro', 'gemini-pro-vision'],
  local: ['llama2', 'llama3', 'mistral', 'codellama', 'custom'],
};

const AISettings = () => {
  const [aiConfig, setAiConfig] = useState({
    ocrService: 'tesseract',
    ocrApiKey: '',
    llmProvider: 'openai',
    llmApiKey: '',
    llmModel: 'gpt-3.5-turbo',
    enableAiTags: true,
    enableAiSummary: false,
  });
  const [aiStatus, setAiStatus] = useState(null);
  const [connectionStatus, setConnectionStatus] = useState({
    ocr: null,
    llm: null,
  });
  const [saving, setSaving] = useState(false);

  const loadAiConfig = useCallback(async () => {
    try {
      const response = await api.get('/ai/config');
      if (response.data.success) {
        setAiConfig(response.data.config);
      } else {
        setAiStatus({ type: 'warning', message: 'Could not load AI configuration. Using defaults.' });
      }
    } catch (error) {
      setAiStatus({ type: 'warning', message: 'Could not load AI configuration. Using defaults.' });
    }
  }, []);

  const performHealthCheck = useCallback(async () => {
    try {
      const response = await api.get('/ai/health');
      if (response.data.success) {
        setConnectionStatus(response.data.health);
      }
    } catch (error) {
      setConnectionStatus({ ocr: false, llm: false });
    }
  }, []);

  useEffect(() => {
    loadAiConfig();
    performHealthCheck();
  }, [loadAiConfig, performHealthCheck]);

  const handleConfigChange = (field, value) => {
    setAiConfig((prev) => ({
      ...prev,
      [field]: value,
    }));
  };

  const saveConfig = async () => {
    if (saving) return;
    setSaving(true);
    try {
      const response = await api.post('/ai/config', aiConfig);
      if (response.data.success) {
        setAiStatus({ type: 'success', message: 'AI configuration saved successfully' });
        await loadAiConfig();
        await performHealthCheck();
      } else {
        setAiStatus({ type: 'error', message: response.data.error || 'Failed to save AI configuration' });
      }
    } catch (error) {
      setAiStatus({ type: 'error', message: 'Failed to save AI configuration: ' + (error.response?.data?.error || error.message) });
    } finally {
      setSaving(false);
    }
  };

  const testConnection = async (service) => {
    try {
      setConnectionStatus((prev) => ({ ...prev, [service]: 'testing' }));
      const response = await api.post(`/ai/test/${service}`, aiConfig);
      if (response.data.success) {
        setConnectionStatus((prev) => ({ ...prev, [service]: true }));
        setAiStatus({ type: 'success', message: response.data.message || `${service.toUpperCase()} connection test successful` });
      } else {
        setConnectionStatus((prev) => ({ ...prev, [service]: false }));
        setAiStatus({ type: 'error', message: response.data.error || `${service.toUpperCase()} connection test failed` });
      }
    } catch (error) {
      setConnectionStatus((prev) => ({ ...prev, [service]: false }));
      setAiStatus({ type: 'error', message: `${service.toUpperCase()} connection test failed: ` + (error.response?.data?.error || error.message) });
    }
  };

  const clearStatus = () => setAiStatus(null);

  return (
    <div className="settings-section">
      <h3>AI Configuration</h3>
      <p>Configure AI services for OCR and LLM features</p>

      {aiStatus && (
        <div className={`status-message ${aiStatus.type}`}>
          <span>{aiStatus.message}</span>
          <button className="close-btn" onClick={clearStatus}>x</button>
        </div>
      )}

      {/* OCR Configuration */}
      <div className="ai-config-group">
        <div className="ai-config-header">
          <h4>OCR Service Configuration</h4>
          <ConnectionStatusIcon status={connectionStatus.ocr} />
        </div>
        <div className="config-item">
          <label>OCR Provider:</label>
          <select value={aiConfig.ocrService} onChange={(e) => handleConfigChange('ocrService', e.target.value)}>
            {OCR_PROVIDERS.map((p) => (
              <option key={p.value} value={p.value}>{p.label}</option>
            ))}
          </select>
        </div>
        {aiConfig.ocrService !== 'tesseract' && (
          <div className="config-item">
            <label>OCR API Key:</label>
            <input
              type="password"
              value={aiConfig.ocrApiKey}
              onChange={(e) => handleConfigChange('ocrApiKey', e.target.value)}
              placeholder="Enter API key for OCR service"
            />
          </div>
        )}
        <button className="btn btn-secondary" onClick={() => testConnection('ocr')} disabled={connectionStatus.ocr === 'testing'}>
          {connectionStatus.ocr === 'testing' ? 'Testing...' : 'Test OCR Connection'}
        </button>
      </div>

      {/* LLM Configuration */}
      <div className="ai-config-group">
        <div className="ai-config-header">
          <h4>LLM Service Configuration</h4>
          <ConnectionStatusIcon status={connectionStatus.llm} />
        </div>
        <div className="config-item">
          <label>LLM Provider:</label>
          <select value={aiConfig.llmProvider} onChange={(e) => handleConfigChange('llmProvider', e.target.value)}>
            {LLM_PROVIDERS.map((p) => (
              <option key={p.value} value={p.value}>{p.label}</option>
            ))}
          </select>
        </div>
        <div className="config-item">
          <label>{aiConfig.llmProvider === 'local' ? 'Server URL:' : 'API Key:'}</label>
          <input
            type={aiConfig.llmProvider === 'local' ? 'text' : 'password'}
            value={aiConfig.llmApiKey}
            onChange={(e) => handleConfigChange('llmApiKey', e.target.value)}
            placeholder={aiConfig.llmProvider === 'local' ? 'http://localhost:8080' : 'Enter API key for LLM service'}
          />
        </div>
        <div className="config-item">
          <label>Model:</label>
          <select value={aiConfig.llmModel} onChange={(e) => handleConfigChange('llmModel', e.target.value)}>
            {(LLM_MODELS[aiConfig.llmProvider] || []).map((model) => (
              <option key={model} value={model}>{model}</option>
            ))}
          </select>
        </div>
        <button className="btn btn-secondary" onClick={() => testConnection('llm')} disabled={connectionStatus.llm === 'testing'}>
          {connectionStatus.llm === 'testing' ? 'Testing...' : 'Test LLM Connection'}
        </button>
      </div>

      {/* AI Features */}
      <div className="ai-config-group">
        <h4>AI Features</h4>
        <div className="config-item checkbox-item">
          <label>
            <input type="checkbox" checked={aiConfig.enableAiTags} onChange={(e) => handleConfigChange('enableAiTags', e.target.checked)} />
            Enable AI-powered automatic tagging
          </label>
        </div>
        <div className="config-item checkbox-item">
          <label>
            <input type="checkbox" checked={aiConfig.enableAiSummary} onChange={(e) => handleConfigChange('enableAiSummary', e.target.checked)} />
            Enable AI-powered document summaries
          </label>
        </div>
      </div>

      <div className="ai-actions">
        <button className="btn btn-primary" onClick={saveConfig} disabled={saving}>
          {saving ? 'Saving...' : 'Save AI Configuration'}
        </button>
      </div>
    </div>
  );
};

export default AISettings;
