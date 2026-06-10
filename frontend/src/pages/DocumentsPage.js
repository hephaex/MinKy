import { useState, useEffect } from 'react';
import TreeSidebar from '../components/TreeSidebar';
import DocumentList from './DocumentList';
import './DocumentsPage.css';

const MOBILE_BREAKPOINT = 768;

const DocumentsPage = () => {
  // On mobile the sidebar overlays the list, so start it hidden there and let
  // the user open it via the toggle; on desktop it stays open alongside the list.
  const [isMobile, setIsMobile] = useState(() => window.innerWidth <= MOBILE_BREAKPOINT);
  const [sidebarVisible, setSidebarVisible] = useState(() => window.innerWidth > MOBILE_BREAKPOINT);

  useEffect(() => {
    const checkMobile = () => {
      setIsMobile(window.innerWidth <= MOBILE_BREAKPOINT);
    };

    checkMobile();
    window.addEventListener('resize', checkMobile);
    return () => window.removeEventListener('resize', checkMobile);
  }, []);

  const toggleSidebar = () => {
    setSidebarVisible((prev) => !prev);
  };

  return (
    <div className="documents-page">
      <TreeSidebar isVisible={sidebarVisible} onToggle={toggleSidebar} />

      {sidebarVisible && isMobile && (
        <div
          className="sidebar-overlay"
          onClick={toggleSidebar}
          onKeyDown={(e) => e.key === 'Escape' && toggleSidebar()}
          role="button"
          tabIndex={0}
          aria-label="Close sidebar"
        />
      )}

      <div className={`documents-main ${sidebarVisible ? 'with-sidebar' : ''}`}>
        <div className="documents-header">
          {!sidebarVisible && (
            <button className="sidebar-toggle-btn" onClick={toggleSidebar}>
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <path d="M2.5 12a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5zm0-4a.5.5 0 0 1 .5-.5h10a.5.5 0 0 1 0 1H3a.5.5 0 0 1-.5-.5z" />
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
