import React from 'react';
import { Link } from 'react-router-dom';
import { useI18n } from '../i18n/i18n';
import './SectionPage.css';

const ExplorePage = () => {
  const { t } = useI18n();

  const exploreFeatures = [
    {
      title: t('navigation.date_explorer'),
      description: t('explore.date_explorer_desc'),
      path: '/explore-date',
      icon: '📅'
    },
    {
      title: 'Advanced Search',
      description: 'Search documents with advanced filters and full-text search powered by OpenSearch',
      path: '/search',
      icon: '🔍'
    },
    {
      title: 'Document Analytics',
      description: 'Analyze document patterns, trends, and insights with OpenSearch analytics',
      path: '/analytics',
      icon: '📊'
    },
    {
      title: 'Search Recommendations',
      description: 'Get intelligent search suggestions and discover related content',
      path: '/search/recommendations',
      icon: '💡'
    },
    {
      title: 'Full-Text Search',
      description: 'Powerful semantic search across all your documents and content',
      path: '/search/full-text',
      icon: '📖'
    },
    {
      title: 'Search Indices',
      description: 'Manage and optimize OpenSearch indices for better search performance',
      path: '/search/indices',
      icon: '⚙️'
    }
  ];

  return (
    <div className="section-page">
      <div className="section-header">
        <h1>{t('navigation.explore')}</h1>
        <p>{t('explore.section_description')}</p>
      </div>

      <div className="features-grid">
        {exploreFeatures.map((feature, index) => (
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

export default ExplorePage;