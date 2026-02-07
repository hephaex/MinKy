import React from 'react';
import { LanguageSelector } from '../../i18n/i18n';

const LanguageSettings = () => {
  return (
    <div className="settings-section">
      <h3>Language Settings</h3>
      <p>Select your preferred language for the interface</p>
      <div className="language-setting">
        <label>Interface Language:</label>
        <LanguageSelector />
      </div>
    </div>
  );
};

export default LanguageSettings;
