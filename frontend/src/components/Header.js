import React from 'react';
import { Link } from 'react-router-dom';
import { useI18n, LanguageSelector } from '../i18n/i18n';
import './Header.css';

const Header = () => {
  const { t } = useI18n();

  return (
    <header className="header">
      <div className="header-content">
        <Link to="/" className="logo">
          <h1>{t('common.app_name')}</h1>
          <span>{t('common.app_description')}</span>
        </Link>
        <nav className="main-nav">
          <Link to="/documents" className="nav-link">
            {t('navigation.documents')}
          </Link>
          <Link to="/explore" className="nav-link">
            {t('navigation.explore')}
          </Link>
          <Link to="/config" className="nav-link">
            {t('navigation.config')}
          </Link>
        </nav>
        <div className="header-actions">
          <LanguageSelector className="header-language-selector" />
        </div>
      </div>
    </header>
  );
};

export default Header;