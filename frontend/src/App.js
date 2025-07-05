import React from 'react';
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Header from './components/Header';
import DocumentList from './pages/DocumentList';
import DocumentView from './pages/DocumentView';
import DocumentCreate from './pages/DocumentCreate';
import DocumentEdit from './pages/DocumentEdit';
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
          </Routes>
        </main>
      </div>
    </Router>
  );
}

export default App;