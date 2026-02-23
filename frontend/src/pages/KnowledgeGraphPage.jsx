import React, { useState, useEffect, useCallback } from 'react';
import KnowledgeGraph from '../components/KnowledgeGraph';
import './KnowledgeGraphPage.css';

/**
 * Sample data generator for demonstration when the API is not available.
 * Builds a realistic-looking knowledge graph from document metadata.
 */
function buildSampleGraph() {
  const nodes = [
    { id: 'doc1', label: 'RAG Architecture', type: 'document', documentId: 'doc1', documentCount: 0, summary: 'Retrieval-Augmented Generation patterns for knowledge systems', topics: ['RAG', 'AI', 'Search'] },
    { id: 'doc2', label: 'pgvector Setup', type: 'document', documentId: 'doc2', documentCount: 0, summary: 'Setting up pgvector for semantic search in PostgreSQL', topics: ['PostgreSQL', 'Embeddings', 'Database'] },
    { id: 'doc3', label: 'Embedding APIs', type: 'document', documentId: 'doc3', documentCount: 0, summary: 'Comparison of OpenAI and Voyage AI embedding APIs', topics: ['OpenAI', 'Embeddings', 'API'] },
    { id: 't1', label: 'Embeddings', type: 'topic', documentCount: 3, topics: ['Vector Search', 'Semantic'] },
    { id: 't2', label: 'PostgreSQL', type: 'technology', documentCount: 2, topics: ['Database', 'SQL'] },
    { id: 't3', label: 'RAG', type: 'topic', documentCount: 2, topics: ['LLM', 'Search'] },
    { id: 't4', label: 'Claude API', type: 'technology', documentCount: 1, topics: ['Anthropic', 'LLM'] },
    { id: 't5', label: 'Knowledge Graph', type: 'insight', documentCount: 1, summary: 'Connecting related knowledge automatically', topics: ['Graph', 'Visualization'] },
    { id: 'p1', label: 'Team Dev', type: 'person', documentCount: 3 },
  ];

  const edges = [
    { id: 'e1', source: 'doc1', target: 't3', weight: 0.95, label: '95%' },
    { id: 'e2', source: 'doc1', target: 't1', weight: 0.8, label: '80%' },
    { id: 'e3', source: 'doc2', target: 't2', weight: 0.9, label: '90%' },
    { id: 'e4', source: 'doc2', target: 't1', weight: 0.85, label: '85%' },
    { id: 'e5', source: 'doc3', target: 't1', weight: 0.92, label: '92%' },
    { id: 'e6', source: 'doc1', target: 'doc2', weight: 0.75, label: '75%' },
    { id: 'e7', source: 'doc1', target: 'doc3', weight: 0.7, label: '70%' },
    { id: 'e8', source: 't3', target: 't4', weight: 0.6 },
    { id: 'e9', source: 't1', target: 't5', weight: 0.65 },
    { id: 'e10', source: 'p1', target: 'doc1', weight: 0.5 },
    { id: 'e11', source: 'p1', target: 'doc2', weight: 0.5 },
  ];

  return { nodes, edges };
}

/**
 * Filter controls for the knowledge graph.
 */
function GraphFilters({ activeTypes, onToggleType }) {
  const types = [
    { value: 'document', label: 'Documents' },
    { value: 'topic', label: 'Topics' },
    { value: 'technology', label: 'Technologies' },
    { value: 'person', label: 'People' },
    { value: 'insight', label: 'Insights' },
  ];

  return (
    <div className="kg-page__filters">
      <span className="kg-page__filters-label">Show:</span>
      {types.map(({ value, label }) => (
        <button
          key={value}
          className={`kg-page__filter-btn${activeTypes.has(value) ? ' kg-page__filter-btn--active' : ''}`}
          onClick={() => onToggleType(value)}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

/**
 * KnowledgeGraphPage - Full-page knowledge graph explorer.
 *
 * Displays the knowledge graph with filter controls.
 * Falls back to sample data when the API is not available.
 */
function KnowledgeGraphPage() {
  const [graphData, setGraphData] = useState({ nodes: [], edges: [] });
  const [loading, setLoading] = useState(true);
  const [apiAvailable, setApiAvailable] = useState(false);
  const [activeTypes, setActiveTypes] = useState(
    new Set(['document', 'topic', 'technology', 'person', 'insight'])
  );
  const [searchQuery, setSearchQuery] = useState('');

  // Path finding state
  const [pathMode, setPathMode] = useState(false);
  const [pathSource, setPathSource] = useState(null);
  const [pathTarget, setPathTarget] = useState(null);
  const [pathResult, setPathResult] = useState(null);
  const [pathLoading, setPathLoading] = useState(false);

  useEffect(() => {
    const loadGraphData = async () => {
      setLoading(true);
      try {
        const response = await fetch('/api/knowledge/graph', {
          signal: AbortSignal.timeout(3000),
        });

        if (response.ok) {
          const data = await response.json();
          setGraphData(data);
          setApiAvailable(true);
        } else {
          setGraphData(buildSampleGraph());
          setApiAvailable(false);
        }
      } catch {
        // API not available, use sample data
        setGraphData(buildSampleGraph());
        setApiAvailable(false);
      } finally {
        setLoading(false);
      }
    };

    loadGraphData();
  }, []);

  const handleToggleType = useCallback(type => {
    setActiveTypes(prev => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  }, []);

  // Find path between two nodes
  const findPath = useCallback(async (from, to) => {
    if (!from || !to || from === to) return;

    setPathLoading(true);
    try {
      const response = await fetch(
        `/api/knowledge/path?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}&max_depth=10`,
        { signal: AbortSignal.timeout(5000) }
      );

      if (response.ok) {
        const data = await response.json();
        setPathResult(data.data || data);
      } else {
        setPathResult({ found: false, node_ids: [], edges: [], length: 0 });
      }
    } catch {
      setPathResult({ found: false, node_ids: [], edges: [], length: 0 });
    } finally {
      setPathLoading(false);
    }
  }, []);

  const handleNodeClick = useCallback(node => {
    if (pathMode) {
      // In path mode, set source or target
      if (!pathSource) {
        setPathSource(node.id);
        setPathResult(null);
      } else if (!pathTarget && node.id !== pathSource) {
        setPathTarget(node.id);
        findPath(pathSource, node.id);
      } else {
        // Reset and start new selection
        setPathSource(node.id);
        setPathTarget(null);
        setPathResult(null);
      }
    } else if (node.documentId) {
      // Navigate to document (optional)
    }
  }, [pathMode, pathSource, pathTarget, findPath]);

  const togglePathMode = useCallback(() => {
    setPathMode(prev => !prev);
    setPathSource(null);
    setPathTarget(null);
    setPathResult(null);
  }, []);

  const clearPath = useCallback(() => {
    setPathSource(null);
    setPathTarget(null);
    setPathResult(null);
  }, []);

  // Filter nodes by type and search query
  const filteredNodes = graphData.nodes.filter(node => {
    const matchesType = activeTypes.has(node.type || 'document');
    const matchesSearch =
      !searchQuery ||
      node.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
      (node.topics || []).some(t => t.toLowerCase().includes(searchQuery.toLowerCase()));
    return matchesType && matchesSearch;
  });

  // Only include edges where both endpoints are visible
  const visibleNodeIds = new Set(filteredNodes.map(n => n.id));
  const filteredEdges = graphData.edges.filter(
    edge => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)
  );

  return (
    <div className="kg-page">
      {/* Page header */}
      <div className="kg-page__header">
        <div className="kg-page__header-left">
          <h1 className="kg-page__title">Knowledge Graph</h1>
          <p className="kg-page__subtitle">
            Explore how your team's knowledge connects
          </p>
        </div>

        {!apiAvailable && !loading && (
          <div className="kg-page__demo-badge">
            Demo data - connect API for real graph
          </div>
        )}
      </div>

      {/* Toolbar */}
      <div className="kg-page__toolbar">
        <div className="kg-page__search">
          <input
            type="text"
            className="kg-page__search-input"
            placeholder="Filter nodes by name or topic..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            aria-label="Filter knowledge graph nodes"
            disabled={pathMode}
          />
          {searchQuery && (
            <button
              className="kg-page__search-clear"
              onClick={() => setSearchQuery('')}
              aria-label="Clear search"
            >
              x
            </button>
          )}
        </div>

        <GraphFilters
          activeTypes={activeTypes}
          onToggleType={handleToggleType}
        />

        {/* Path Mode Controls */}
        <div className="kg-page__path-controls">
          <button
            className={`kg-page__path-btn${pathMode ? ' kg-page__path-btn--active' : ''}`}
            onClick={togglePathMode}
            title={pathMode ? 'Exit path mode' : 'Find path between two nodes'}
          >
            {pathMode ? 'Exit Path Mode' : 'Find Path'}
          </button>

          {pathMode && (
            <div className="kg-page__path-status">
              {pathLoading ? (
                <span className="kg-page__path-loading">Finding path...</span>
              ) : pathResult?.found ? (
                <span className="kg-page__path-found">
                  Path found: {pathResult.length} step{pathResult.length !== 1 ? 's' : ''}
                </span>
              ) : pathResult ? (
                <span className="kg-page__path-not-found">No path found</span>
              ) : pathSource ? (
                <span className="kg-page__path-hint">
                  Now click the target node
                </span>
              ) : (
                <span className="kg-page__path-hint">
                  Click the source node
                </span>
              )}

              {(pathSource || pathTarget) && (
                <button
                  className="kg-page__path-clear"
                  onClick={clearPath}
                  title="Clear selection"
                >
                  Clear
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Graph */}
      <div className="kg-page__graph-wrapper">
        <KnowledgeGraph
          nodes={filteredNodes}
          edges={filteredEdges}
          title={null}
          loading={loading}
          emptyMessage={
            searchQuery
              ? `No nodes match "${searchQuery}"`
              : 'No knowledge connections yet. Upload documents to build the graph.'
          }
          onNodeClick={handleNodeClick}
          pathMode={pathMode}
          pathSource={pathSource}
          pathTarget={pathTarget}
          pathResult={pathResult}
        />
      </div>
    </div>
  );
}

export default KnowledgeGraphPage;
