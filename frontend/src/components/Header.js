import React from 'react';
import { Link } from 'react-router-dom';
import './Header.css';

const Header = () => {
  return (
    <header className="header">
      <div className="header-content">
        <Link to="/" className="logo">
          <h1>Minky</h1>
          <span>Markdown Document Manager</span>
        </Link>
        <nav>
          <Link to="/" className="nav-link">Documents</Link>
        </nav>
      </div>
    </header>
  );
};

export default Header;