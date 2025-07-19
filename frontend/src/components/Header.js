import React, { useState } from 'react';
import { Link } from 'react-router-dom';
import { useI18n, LanguageSelector } from '../i18n/i18n';
import './Header.css';

const Header = () => {
  const { t } = useI18n();
  const [openDropdown, setOpenDropdown] = useState(null);

  const toggleDropdown = (dropdown) => {
    setOpenDropdown(openDropdown === dropdown ? null : dropdown);
  };

  const closeDropdown = () => {
    setOpenDropdown(null);
  };

  return (
    <header className="header">
      <div className="header-content">
        <Link to="/" className="logo">
          <h1>{t('common.app_name')}</h1>
          <span>{t('common.app_description')}</span>
        </Link>
        <nav className="main-nav">
          {/* Documents Section */}
          <div className="nav-dropdown">
            <button 
              className="nav-button"
              onClick={() => toggleDropdown('documents')}
              onBlur={() => setTimeout(closeDropdown, 150)}
            >
              {t('navigation.documents')} ▼
            </button>
            {openDropdown === 'documents' && (
              <div className="dropdown-menu">
                <div className="dropdown-section">
                  <Link to="/" className="dropdown-link main-item" onClick={closeDropdown}>
                    {t('navigation.document_list')}
                  </Link>
                  <div className="submenu">
                    <Link to="/documents/new" className="dropdown-link sub-item" onClick={closeDropdown}>
                      {t('navigation.new_document')}
                    </Link>
                    <Link to="/" className="dropdown-link sub-item" onClick={closeDropdown}>
                      {t('navigation.upload_md')}
                    </Link>
                    <Link to="/ocr" className="dropdown-link sub-item" onClick={closeDropdown}>
                      {t('navigation.ocr')}
                    </Link>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Explore Section */}
          <div className="nav-dropdown">
            <button 
              className="nav-button"
              onClick={() => toggleDropdown('explore')}
              onBlur={() => setTimeout(closeDropdown, 150)}
            >
              {t('navigation.explore')} ▼
            </button>
            {openDropdown === 'explore' && (
              <div className="dropdown-menu">
                <Link to="/tags" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.tags')}
                </Link>
                <Link to="/categories" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.categories')}
                </Link>
                <Link to="/analytics" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.analytics')}
                </Link>
                <Link to="/explore" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.date_explorer')}
                </Link>
              </div>
            )}
          </div>

          {/* Config Section */}
          <div className="nav-dropdown">
            <button 
              className="nav-button"
              onClick={() => toggleDropdown('config')}
              onBlur={() => setTimeout(closeDropdown, 150)}
            >
              {t('navigation.config')} ▼
            </button>
            {openDropdown === 'config' && (
              <div className="dropdown-menu">
                <Link to="/admin" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.admin')}
                </Link>
                <Link to="/settings" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.settings')}
                </Link>
                <Link to="/sync" className="dropdown-link" onClick={closeDropdown}>
                  {t('navigation.sync_git')}
                </Link>
              </div>
            )}
          </div>
        </nav>
        <div className="header-actions">
          <LanguageSelector className="header-language-selector" />
        </div>
      </div>
    </header>
  );
};

export default Header;