import React, { useState, useCallback } from 'react';
import { AskQuestion } from '../components/Knowledge';
import { SearchBar, SearchResults } from '../components/Search';
import { searchService } from '../services/api';
import useAsync from '../hooks/useAsync';
import './KnowledgeSearch.css';

const TABS = [
  { id: 'ask', label: 'AI에게 질문', description: 'RAG 기반 자연어 질문' },
  { id: 'semantic', label: '유사 문서 검색', description: '의미 기반 문서 검색' },
];

const KnowledgeSearch = () => {
  const [activeTab, setActiveTab] = useState('ask');
  const [semanticQuery, setSemanticQuery] = useState('');

  const fetchSemantic = useCallback(
    (query) => searchService.semantic(query),
    []
  );

  const { execute: runSemanticSearch, loading, error, data, reset } = useAsync(fetchSemantic);

  const handleSemanticSearch = useCallback(async (query) => {
    setSemanticQuery(query);
    try {
      await runSemanticSearch(query);
    } catch (_) {
      // error handled by useAsync
    }
  }, [runSemanticSearch]);

  const handleTabChange = useCallback((tab) => {
    setActiveTab(tab);
    if (tab === 'ask') {
      reset();
      setSemanticQuery('');
    }
  }, [reset]);

  const semanticResults = data?.results || data?.data || [];
  const totalCount = data?.total || data?.meta?.total || null;

  return (
    <div className="kb-page">
      <div className="kb-page-header">
        <h1 className="kb-page-title">지식 검색</h1>
        <p className="kb-page-desc">
          팀의 문서에서 필요한 지식을 자연어로 찾거나 AI에게 직접 질문하세요.
        </p>
      </div>

      <nav className="kb-tabs" role="tablist" aria-label="검색 모드">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            type="button"
            className={`kb-tab ${activeTab === tab.id ? 'kb-tab--active' : ''}`}
            aria-selected={activeTab === tab.id}
            onClick={() => handleTabChange(tab.id)}
          >
            <span className="kb-tab-label">{tab.label}</span>
            <span className="kb-tab-desc">{tab.description}</span>
          </button>
        ))}
      </nav>

      <div className="kb-tab-content" role="tabpanel">
        {activeTab === 'ask' && (
          <AskQuestion />
        )}

        {activeTab === 'semantic' && (
          <div className="kb-semantic">
            <SearchBar
              onSearch={handleSemanticSearch}
              mode="semantic"
              loading={loading}
            />
            <SearchResults
              results={semanticResults}
              query={semanticQuery}
              loading={loading}
              error={error}
              totalCount={totalCount}
              onRetry={() => semanticQuery && handleSemanticSearch(semanticQuery)}
            />
          </div>
        )}
      </div>
    </div>
  );
};

export default KnowledgeSearch;
