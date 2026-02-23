import React, { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import PropTypes from 'prop-types';
import { computeLayout } from './graphLayout';
import GraphNode from './GraphNode';
import GraphEdge from './GraphEdge';
import NodeDetailPanel from './NodeDetailPanel';
import './KnowledgeGraph.css';

const DEFAULT_WIDTH = 900;
const DEFAULT_HEIGHT = 600;
const ZOOM_MIN = 0.3;
const ZOOM_MAX = 3.0;
const ZOOM_STEP = 0.15;

/**
 * KnowledgeGraph - Interactive force-directed knowledge graph visualization.
 *
 * Accepts nodes and edges, runs layout, and renders an SVG-based
 * interactive graph with zoom, pan, node selection, and detail panel.
 *
 * @param {object[]} nodes - Graph nodes with id, label, type, etc.
 * @param {object[]} edges - Graph edges with source, target, weight
 * @param {string}   title - Graph title (optional)
 * @param {boolean}  loading - Show loading state
 * @param {string}   emptyMessage - Message shown when no data
 */
function KnowledgeGraph({
  nodes,
  edges,
  title,
  loading,
  emptyMessage,
  onNodeClick,
  pathMode,
  pathSource,
  pathTarget,
  pathResult,
  clusterMode,
  clusterData,
}) {
  const containerRef = useRef(null);
  const svgRef = useRef(null);

  const [dimensions, setDimensions] = useState({
    width: DEFAULT_WIDTH,
    height: DEFAULT_HEIGHT,
  });
  const [positionedNodes, setPositionedNodes] = useState([]);
  const [selectedNode, setSelectedNode] = useState(null);
  const [hoveredNode, setHoveredNode] = useState(null);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [layoutDone, setLayoutDone] = useState(false);

  // Respond to container size changes
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const observer = new ResizeObserver(entries => {
      const { width, height } = entries[0].contentRect;
      setDimensions({
        width: width || DEFAULT_WIDTH,
        height: Math.max(height, 400),
      });
    });

    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Recompute layout when nodes/edges/dimensions change
  useEffect(() => {
    if (nodes.length === 0) {
      setPositionedNodes([]);
      setLayoutDone(true);
      return;
    }

    setLayoutDone(false);

    // Run layout in a timeout to avoid blocking the UI
    const timer = setTimeout(() => {
      const laid = computeLayout(nodes, edges, dimensions.width, dimensions.height);
      setPositionedNodes(laid);
      setLayoutDone(true);
    }, 50);

    return () => clearTimeout(timer);
  }, [nodes, edges, dimensions]);

  // Degree map: number of edges per node
  const degreeMap = useMemo(() => {
    const map = {};
    for (const node of nodes) {
      map[node.id] = 0;
    }
    for (const edge of edges) {
      if (map[edge.source] !== undefined) map[edge.source]++;
      if (map[edge.target] !== undefined) map[edge.target]++;
    }
    return map;
  }, [nodes, edges]);

  // Path edge and node IDs
  const pathEdgeIds = useMemo(() => {
    if (!pathResult?.found) return new Set();
    return new Set((pathResult.edges || []).map(e => e.id));
  }, [pathResult]);

  const pathNodeIds = useMemo(() => {
    if (!pathResult?.found) return new Set();
    return new Set(pathResult.node_ids || []);
  }, [pathResult]);

  // Cluster color map: node ID -> cluster color
  const clusterColorMap = useMemo(() => {
    if (!clusterMode || !clusterData?.clusters) return {};
    const map = {};
    for (const cluster of clusterData.clusters) {
      for (const nodeId of cluster.node_ids) {
        map[nodeId] = cluster.color;
      }
    }
    return map;
  }, [clusterMode, clusterData]);

  // Edges connected to the hovered/selected node
  const highlightedEdgeIds = useMemo(() => {
    // If path is found, highlight path edges
    if (pathResult?.found && pathEdgeIds.size > 0) {
      return pathEdgeIds;
    }

    const focus = hoveredNode || selectedNode;
    if (!focus) return new Set();
    return new Set(
      edges
        .filter(e => e.source === focus.id || e.target === focus.id)
        .map(e => e.id)
    );
  }, [hoveredNode, selectedNode, edges, pathResult, pathEdgeIds]);

  // Edges connected to the selected node (for detail panel)
  const selectedNodeEdges = useMemo(() => {
    if (!selectedNode) return [];
    return edges.filter(
      e => e.source === selectedNode.id || e.target === selectedNode.id
    );
  }, [selectedNode, edges]);

  const handleNodeSelect = useCallback(
    node => {
      setSelectedNode(prev => (prev?.id === node.id ? null : node));
      if (onNodeClick) onNodeClick(node);
    },
    [onNodeClick]
  );

  const handleNodeHover = useCallback(node => {
    setHoveredNode(node);
  }, []);

  const handlePanelClose = useCallback(() => {
    setSelectedNode(null);
  }, []);

  const handleNavigateToNode = useCallback(node => {
    setSelectedNode(node);
  }, []);

  // Zoom controls
  const handleZoomIn = () => setZoom(z => Math.min(z + ZOOM_STEP, ZOOM_MAX));
  const handleZoomOut = () => setZoom(z => Math.max(z - ZOOM_STEP, ZOOM_MIN));
  const handleZoomReset = () => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  };

  // Pan via mouse drag
  const handleMouseDown = useCallback(e => {
    if (e.target.closest('.graph-node')) return;
    setIsDragging(true);
    setDragStart({ x: e.clientX, y: e.clientY });
  }, []);

  const handleMouseMove = useCallback(
    e => {
      if (!isDragging) return;
      const dx = e.clientX - dragStart.x;
      const dy = e.clientY - dragStart.y;
      setPan(prev => ({ x: prev.x + dx, y: prev.y + dy }));
      setDragStart({ x: e.clientX, y: e.clientY });
    },
    [isDragging, dragStart]
  );

  const handleMouseUp = useCallback(() => {
    setIsDragging(false);
  }, []);

  // Zoom via scroll wheel
  const handleWheel = useCallback(e => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
    setZoom(z => Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z + delta)));
  }, []);

  if (loading) {
    return (
      <div className="knowledge-graph knowledge-graph--loading">
        <div className="knowledge-graph__spinner" aria-label="Loading graph" />
        <p>Building knowledge graph...</p>
      </div>
    );
  }

  if (nodes.length === 0) {
    return (
      <div className="knowledge-graph knowledge-graph--empty">
        <div className="knowledge-graph__empty-icon">
          <svg viewBox="0 0 64 64" width="64" height="64" aria-hidden="true">
            <circle cx="16" cy="32" r="8" fill="#BDC3C7" />
            <circle cx="48" cy="16" r="8" fill="#BDC3C7" />
            <circle cx="48" cy="48" r="8" fill="#BDC3C7" />
            <line x1="24" y1="29" x2="40" y2="19" stroke="#BDC3C7" strokeWidth="2" />
            <line x1="24" y1="35" x2="40" y2="45" stroke="#BDC3C7" strokeWidth="2" />
          </svg>
        </div>
        <p className="knowledge-graph__empty-message">
          {emptyMessage || 'No knowledge connections to display yet.'}
        </p>
        <p className="knowledge-graph__empty-hint">
          Upload and analyze documents to build your knowledge graph.
        </p>
      </div>
    );
  }

  return (
    <div className="knowledge-graph" ref={containerRef}>
      {/* Header */}
      {title && (
        <div className="knowledge-graph__header">
          <h2 className="knowledge-graph__title">{title}</h2>
          <div className="knowledge-graph__stats">
            <span>{nodes.length} nodes</span>
            <span>{edges.length} connections</span>
          </div>
        </div>
      )}

      <div className="knowledge-graph__body">
        {/* SVG Canvas */}
        <div
          className="knowledge-graph__canvas-wrapper"
          style={{ cursor: isDragging ? 'grabbing' : 'grab' }}
          onMouseDown={handleMouseDown}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
          onWheel={handleWheel}
        >
          {!layoutDone && (
            <div className="knowledge-graph__layout-indicator">
              Computing layout...
            </div>
          )}

          <svg
            ref={svgRef}
            width="100%"
            height={dimensions.height}
            aria-label="Knowledge graph visualization"
            role="img"
          >
            <defs>
              <marker
                id="arrowhead"
                markerWidth="10"
                markerHeight="7"
                refX="10"
                refY="3.5"
                orient="auto"
              >
                <polygon points="0 0, 10 3.5, 0 7" fill="#7F8C8D" />
              </marker>
            </defs>

            <g transform={`translate(${pan.x}, ${pan.y}) scale(${zoom})`}>
              {/* Edges (drawn below nodes) */}
              {edges.map(edge => {
                const src = positionedNodes.find(n => n.id === edge.source);
                const tgt = positionedNodes.find(n => n.id === edge.target);
                return (
                  <GraphEdge
                    key={edge.id}
                    edge={edge}
                    sourceNode={src}
                    targetNode={tgt}
                    isHighlighted={highlightedEdgeIds.has(edge.id)}
                  />
                );
              })}

              {/* Nodes */}
              {positionedNodes.map(node => {
                const degree = degreeMap[node.id] || 0;
                const isSelected = selectedNode?.id === node.id;
                const isPathSource = pathSource === node.id;
                const isPathTarget = pathTarget === node.id;
                const isInPath = pathNodeIds.has(node.id);
                const clusterColor = clusterColorMap[node.id] || null;

                // Highlight logic
                let isHighlighted = true;
                if (pathMode && pathResult?.found) {
                  // In path mode with result, only highlight path nodes
                  isHighlighted = isInPath;
                } else if (hoveredNode) {
                  isHighlighted =
                    hoveredNode.id === node.id ||
                    Array.from(highlightedEdgeIds).some(eid => {
                      const edge = edges.find(e => e.id === eid);
                      return edge && (edge.source === node.id || edge.target === node.id);
                    });
                }

                return (
                  <GraphNode
                    key={node.id}
                    node={node}
                    degree={degree}
                    isSelected={isSelected || isPathSource || isPathTarget}
                    isHighlighted={isHighlighted}
                    isPathNode={isInPath}
                    isPathEndpoint={isPathSource || isPathTarget}
                    clusterColor={clusterColor}
                    onSelect={handleNodeSelect}
                    onHover={handleNodeHover}
                  />
                );
              })}
            </g>
          </svg>
        </div>

        {/* Detail Panel */}
        {selectedNode && (
          <NodeDetailPanel
            node={selectedNode}
            relatedEdges={selectedNodeEdges}
            allNodes={positionedNodes}
            onClose={handlePanelClose}
            onNavigate={handleNavigateToNode}
          />
        )}
      </div>

      {/* Controls */}
      <div className="knowledge-graph__controls" aria-label="Graph controls">
        <button
          className="knowledge-graph__control-btn"
          onClick={handleZoomIn}
          aria-label="Zoom in"
          title="Zoom in"
        >
          +
        </button>
        <button
          className="knowledge-graph__control-btn"
          onClick={handleZoomOut}
          aria-label="Zoom out"
          title="Zoom out"
        >
          -
        </button>
        <button
          className="knowledge-graph__control-btn knowledge-graph__control-btn--reset"
          onClick={handleZoomReset}
          aria-label="Reset view"
          title="Reset zoom and pan"
        >
          Reset
        </button>
        <span className="knowledge-graph__zoom-level">
          {Math.round(zoom * 100)}%
        </span>
      </div>

      {/* Legend */}
      <div className="knowledge-graph__legend" aria-label="Node type legend">
        {[
          { type: 'document', label: 'Document' },
          { type: 'topic', label: 'Topic' },
          { type: 'person', label: 'Person' },
          { type: 'technology', label: 'Technology' },
          { type: 'insight', label: 'Insight' },
        ].map(({ type, label }) => (
          <div key={type} className="knowledge-graph__legend-item">
            <span
              className="knowledge-graph__legend-dot"
              style={{
                backgroundColor: {
                  document: '#4A90D9',
                  topic: '#7ED321',
                  person: '#F5A623',
                  technology: '#9B59B6',
                  insight: '#E74C3C',
                }[type],
              }}
            />
            <span>{label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

KnowledgeGraph.propTypes = {
  nodes: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.string.isRequired,
      label: PropTypes.string.isRequired,
      type: PropTypes.string,
      documentCount: PropTypes.number,
      documentId: PropTypes.string,
      summary: PropTypes.string,
      topics: PropTypes.arrayOf(PropTypes.string),
    })
  ),
  edges: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.string.isRequired,
      source: PropTypes.string.isRequired,
      target: PropTypes.string.isRequired,
      weight: PropTypes.number,
      label: PropTypes.string,
    })
  ),
  title: PropTypes.string,
  loading: PropTypes.bool,
  emptyMessage: PropTypes.string,
  onNodeClick: PropTypes.func,
  pathMode: PropTypes.bool,
  pathSource: PropTypes.string,
  pathTarget: PropTypes.string,
  pathResult: PropTypes.shape({
    found: PropTypes.bool,
    node_ids: PropTypes.arrayOf(PropTypes.string),
    edges: PropTypes.array,
    length: PropTypes.number,
  }),
  clusterMode: PropTypes.bool,
  clusterData: PropTypes.shape({
    clusters: PropTypes.arrayOf(
      PropTypes.shape({
        id: PropTypes.number,
        node_ids: PropTypes.arrayOf(PropTypes.string),
        size: PropTypes.number,
        color: PropTypes.string,
        label: PropTypes.string,
      })
    ),
    cluster_count: PropTypes.number,
    node_cluster_map: PropTypes.object,
  }),
};

KnowledgeGraph.defaultProps = {
  nodes: [],
  edges: [],
  title: null,
  loading: false,
  emptyMessage: null,
  onNodeClick: null,
  pathMode: false,
  pathSource: null,
  pathTarget: null,
  pathResult: null,
  clusterMode: false,
  clusterData: null,
};

export default KnowledgeGraph;
