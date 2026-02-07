import React, { useState } from 'react';
import { documentService } from '../../services/api';

const DownloadIcon = () => (
  <svg className="btn-icon" viewBox="0 0 16 16" fill="currentColor">
    <path d="M.5 9.9a.5.5 0 0 1 .5.5v2.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-2.5a.5.5 0 0 1 1 0v2.5a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2v-2.5a.5.5 0 0 1 .5-.5z"/>
    <path d="M7.646 11.854a.5.5 0 0 0 .708 0l3-3a.5.5 0 0 0-.708-.708L8.5 10.293V1.5a.5.5 0 0 0-1 0v8.793L5.354 8.146a.5.5 0 1 0-.708.708l3 3z"/>
  </svg>
);

const ExportSettings = () => {
  const [exportStatus, setExportStatus] = useState(null);
  const [exporting, setExporting] = useState(false);

  const handleExportAll = async (shortFilename = false) => {
    try {
      setExporting(true);
      const response = await documentService.exportAllDocuments(shortFilename);
      setExportStatus({
        type: 'success',
        message: `Export completed: ${response.results.exported} documents exported to backup folder`,
      });
    } catch (error) {
      setExportStatus({
        type: 'error',
        message: 'Export failed: ' + (error.message || 'Unknown error occurred'),
      });
    } finally {
      setExporting(false);
    }
  };

  const clearStatus = () => setExportStatus(null);

  return (
    <div className="settings-section">
      <h3>Document Export</h3>
      <p>Export all documents from the database to backup folder</p>

      {exportStatus && (
        <div className={`status-message ${exportStatus.type}`}>
          <span>{exportStatus.message}</span>
          <button className="close-btn" onClick={clearStatus}>x</button>
        </div>
      )}

      <div className="export-actions">
        <button className="btn btn-secondary" onClick={() => handleExportAll(false)} disabled={exporting}>
          <DownloadIcon />
          {exporting ? 'Exporting...' : 'Export All Documents'}
        </button>
        <button className="btn btn-secondary" onClick={() => handleExportAll(true)} disabled={exporting}>
          <DownloadIcon />
          {exporting ? 'Exporting...' : 'Export (Short Names)'}
        </button>
      </div>
    </div>
  );
};

export default ExportSettings;
