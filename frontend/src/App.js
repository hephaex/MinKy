import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import { I18nProvider } from './i18n/i18n';
import Header from './components/Header';
import ErrorBoundary from './components/ErrorBoundary';
import DocumentList from './pages/DocumentList';
import DocumentView from './pages/DocumentView';
import DocumentCreate from './pages/DocumentCreate';
import DocumentEdit from './pages/DocumentEdit';
import TagList from './pages/TagList';
import TagDetail from './pages/TagDetail';
import DateExplorer from './pages/DateExplorer';
import DateExplorerByDate from './pages/DateExplorerByDate';
import Settings from './pages/Settings';
import AnalyticsDashboard from './pages/AnalyticsDashboard';
import AdminPanel from './pages/AdminPanel';
import CategoryManager from './pages/CategoryManager';
import OCRPage from './pages/OCRPage';
import ImportPage from './pages/ImportPage';
import DocumentsPage from './pages/DocumentsPage';
import ExplorePage from './pages/ExplorePage';
import ConfigPage from './pages/ConfigPage';
import './App.css';

function App() {
  return (
    <I18nProvider>
      <Router>
        <div className="App">
          <Header />
          <main className="main-content">
            <ErrorBoundary>
              <Routes>
                <Route path="/" element={<DocumentsPage />} />
                <Route path="/documents/new" element={<DocumentCreate />} />
                <Route path="/documents/:id" element={<DocumentView />} />
                <Route path="/documents/:id/edit" element={<DocumentEdit />} />
                <Route path="/tags" element={<TagList />} />
                <Route path="/tags/:slug" element={<TagDetail />} />
                <Route path="/categories" element={<CategoryManager />} />
                <Route path="/explore-date" element={<DateExplorerByDate />} />
                <Route path="/analytics" element={<AnalyticsDashboard />} />
                <Route path="/settings" element={<Settings />} />
                <Route path="/admin" element={<AdminPanel />} />
                <Route path="/ocr" element={<OCRPage />} />
                <Route path="/import" element={<ImportPage />} />
                <Route path="/documents" element={<DocumentsPage />} />
                <Route path="/explore" element={<ExplorePage />} />
                <Route path="/config" element={<ConfigPage />} />
              </Routes>
            </ErrorBoundary>
          </main>
        </div>
      </Router>
    </I18nProvider>
  );
}

export default App;