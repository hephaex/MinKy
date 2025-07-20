import React, { useState, useEffect } from 'react';
import DocumentsSidebar from '../components/DocumentsSidebar';
import DocumentList from './DocumentList';
import './DocumentsPage.css';

const DocumentsPage = () => {
  const [sidebarVisible, setSidebarVisible] = useState(true);
  const [isMobile, setIsMobile] = useState(false);

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth <= 768);
    };
    
    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const toggleSidebar = () => {
    console.log('DocumentsPage: toggleSidebar called, current state:', sidebarVisible);
    setSidebarVisible(!sidebarVisible);
  };

  console.log('DocumentsPage: Rendering with sidebarVisible =', sidebarVisible);

  return (
    <div className="documents-page">
      <DocumentsSidebar 
        isVisible={sidebarVisible} 
        onToggle={toggleSidebar} 
      />
      
      {sidebarVisible && isMobile && (
        <div className="sidebar-overlay" onClick={toggleSidebar} />
      )}
      
      <div className={`documents-main ${sidebarVisible ? 'with-sidebar' : ''}`}>
        <div className="documents-header">
          {!sidebarVisible && (
            <button className="sidebar-toggle-btn" onClick={toggleSidebar}>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <path d="M2.5 12a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5z"/>
              </svg>
            </button>
          )}
          <h1>Documents</h1>
        </div>
        
        <div className="documents-content">
          <DocumentList />
        </div>
      </div>
    </div>
  );
};

export default DocumentsPage;