import { useState } from 'react';
import PropTypes from 'prop-types';
import { useI18n } from '../i18n/i18n';
import './SearchBar.css';

const SearchBar = ({ onSearch, initialValue = '' }) => {
  const { t } = useI18n();
  const [query, setQuery] = useState(initialValue);

  const handleSubmit = (e) => {
    e.preventDefault();
    onSearch(query.trim());
  };

  const handleClear = () => {
    setQuery('');
    onSearch('');
  };

  return (
    <form className="search-bar" onSubmit={handleSubmit}>
      <div className="search-input-group">
        <input
          type="text"
          className="search-input"
          name="document-search"
          placeholder={t('document.search_documents')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label="Search documents"
        />
        {query && (
          <button
            type="button"
            className="clear-button"
            onClick={handleClear}
            aria-label="Clear search"
          >
            ×
          </button>
        )}
        <button type="submit" className="search-button" aria-label="Submit search">
          🔍
        </button>
      </div>
    </form>
  );
};

SearchBar.propTypes = {
  onSearch: PropTypes.func.isRequired,
  initialValue: PropTypes.string,
};

export default SearchBar;
