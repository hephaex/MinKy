import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Header from './components/Header';
import DocumentList from './pages/DocumentList';
import DocumentView from './pages/DocumentView';
import DocumentCreate from './pages/DocumentCreate';
import DocumentEdit from './pages/DocumentEdit';
import TagList from './pages/TagList';
import TagDetail from './pages/TagDetail';
import DateExplorer from './pages/DateExplorer';
import DateExplorerByDate from './pages/DateExplorerByDate';
import Settings from './pages/Settings';
import './App.css';

function App() {
  return (
    <Router>
      <div className="App">
        <Header />
        <main className="main-content">
          <Routes>
            <Route path="/" element={<DocumentList />} />
            <Route path="/documents/new" element={<DocumentCreate />} />
            <Route path="/documents/:id" element={<DocumentView />} />
            <Route path="/documents/:id/edit" element={<DocumentEdit />} />
            <Route path="/tags" element={<TagList />} />
            <Route path="/tags/:slug" element={<TagDetail />} />
            <Route path="/explore" element={<DateExplorer />} />
            <Route path="/explore-date" element={<DateExplorerByDate />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;