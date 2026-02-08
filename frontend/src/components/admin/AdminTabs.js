import React from 'react';

const TabButton = ({ tab, label, active, onClick }) => (
  <button
    className={`tab-button ${active ? 'active' : ''}`}
    onClick={onClick}
  >
    {label}
  </button>
);

const AdminTabs = ({ activeTab, onTabChange }) => {
  const tabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'users', label: 'Users' },
    { id: 'documents', label: 'Documents' },
    { id: 'maintenance', label: 'Maintenance' },
  ];

  return (
    <div className="admin-tabs">
      {tabs.map(tab => (
        <TabButton
          key={tab.id}
          tab={tab.id}
          label={tab.label}
          active={activeTab === tab.id}
          onClick={() => onTabChange(tab.id)}
        />
      ))}
    </div>
  );
};

export default AdminTabs;
