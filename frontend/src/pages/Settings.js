import React from 'react';
import { LanguageSettings, AISettings, GitSettings, ExportSettings } from '../components/settings';
import './Settings.css';

const Settings = () => {
  return (
    <div className="settings">
      <div className="settings-header">
        <h2>Settings</h2>
      </div>

      <LanguageSettings />
      <AISettings />
      <GitSettings />
      <ExportSettings />
    </div>
  );
};

export default Settings;
