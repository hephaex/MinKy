import React from 'react';
import { Link } from 'react-router-dom';
import { useI18n } from '../i18n/i18n';
import './SectionPage.css';

const ConfigPage = () => {
  const { t } = useI18n();

  const configFeatures = [
    {
      title: t('navigation.settings'),
      description: t('config.settings_desc'),
      path: '/settings',
      icon: '⚙️'
    },
    {
      title: t('navigation.user_management'),
      description: t('config.user_management_desc'),
      path: '/admin',
      icon: '👥'
    },
    {
      title: t('navigation.system_overview'),
      description: t('config.system_overview_desc'),
      path: '/admin',
      icon: '📋'
    },
    {
      title: t('navigation.maintenance'),
      description: t('config.maintenance_desc'),
      path: '/admin',
      icon: '🔧'
    },
    {
      title: t('navigation.sync_git'),
      description: t('config.sync_git_desc'),
      path: '/sync',
      icon: '🔄'
    }
  ];

  return (
    <div className="section-page">
      <div className="section-header">
        <h1>{t('navigation.config')}</h1>
        <p>{t('config.section_description')}</p>
      </div>

      <div className="features-grid">
        {configFeatures.map((feature, index) => (
          <Link to={feature.path} key={index} className="feature-card">
            <div className="feature-icon">{feature.icon}</div>
            <h3>{feature.title}</h3>
            <p>{feature.description}</p>
          </Link>
        ))}
      </div>
    </div>
  );
};

export default ConfigPage;