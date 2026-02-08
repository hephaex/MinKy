import React from 'react';

const AdminMaintenance = ({ onCleanup }) => {
  return (
    <div className="maintenance-tab">
      <h2>System Maintenance</h2>

      <div className="maintenance-section">
        <h3>Cleanup Operations</h3>
        <div className="cleanup-actions">
          <button
            className="cleanup-btn"
            onClick={() => onCleanup('orphaned_attachments')}
          >
            Remove Orphaned Attachments
          </button>
          <button
            className="cleanup-btn"
            onClick={() => onCleanup('old_versions')}
          >
            Clean Old Versions
          </button>
          <button
            className="cleanup-btn danger"
            onClick={() => onCleanup('all')}
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
  );
};

export default AdminMaintenance;
