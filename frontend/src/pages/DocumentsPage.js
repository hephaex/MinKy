import React from 'react';
import { Link } from 'react-router-dom';
import { useI18n } from '../i18n/i18n';
import './SectionPage.css';

const DocumentsPage = () => {
  const { t } = useI18n();

  const documentFeatures = [
    {
      title: t('navigation.document_list'),
      description: t('documents.document_list_desc'),
      path: '/',
      icon: '📄'
    },
    {
      title: t('navigation.new_document'),
      description: t('documents.new_document_desc'),
      path: '/documents/new',
      icon: '✏️'
    },
    {
      title: t('navigation.upload_md'),
      description: t('documents.upload_md_desc'),
      path: '/',
      icon: '📁'
    },
    {
      title: t('navigation.ocr'),
      description: t('documents.ocr_desc'),
      path: '/ocr',
      icon: '🔍'
    }
  ];

  return (
    <div className="section-page">
      <div className="section-header">
        <h1>{t('navigation.documents')}</h1>
        <p>{t('documents.section_description')}</p>
      </div>

      <div className="features-grid">
        {documentFeatures.map((feature, index) => (
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

export default DocumentsPage;