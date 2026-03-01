import React, { useMemo } from 'react';
import PropTypes from 'prop-types';
import { nodeColor } from './graphLayout';
import { formatRelativeTime, formatDate } from '../../utils/dateUtils';

/**
 * Side panel showing details of the selected graph node.
 * Displays metadata, statistics, cluster info, and quick actions.
 */
function NodeDetailPanel({
  node,
  relatedEdges,
  allNodes,
  onClose,
  onNavigate,
  clusterMode,
  clusterData,
  onSetPathSource,
  onFilterToNode,
  onExportConnections,
}) {
  // Calculate node statistics (hook must be called unconditionally)
  const stats = useMemo(() => {
    if (!node) return { degree: 0, avgWeight: 0 };
    const degree = relatedEdges.length;
    const avgWeight =
      degree > 0 ? relatedEdges.reduce((sum, e) => sum + (e.weight || 0), 0) / degree : 0;
    return { degree, avgWeight };
  }, [node, relatedEdges]);

  // Get cluster information for this node (hook must be called unconditionally)
  const clusterInfo = useMemo(() => {
    if (!node || !clusterMode || !clusterData?.node_cluster_map) return null;
    const clusterId = clusterData.node_cluster_map[node.id];
    if (clusterId === undefined) return null;
    const cluster = clusterData.clusters?.find((c) => c.id === clusterId);
    return cluster || null;
  }, [node, clusterMode, clusterData]);

  // Compute related nodes (hook must be called unconditionally)
  const relatedNodes = useMemo(() => {
    if (!node) return [];
    return relatedEdges
      .map((edge) => {
        const otherId = edge.source === node.id ? edge.target : edge.source;
        const otherNode = allNodes.find((n) => n.id === otherId);
        return otherNode ? { ...otherNode, weight: edge.weight, edgeLabel: edge.label } : null;
      })
      .filter(Boolean)
      .sort((a, b) => (b.weight || 0) - (a.weight || 0));
  }, [node, relatedEdges, allNodes]);

  // Early return after hooks
  if (!node) return null;

  const color = nodeColor(node.type);

  return (
    <div className="knowledge-graph__panel">
      {/* Header */}
      <div className="knowledge-graph__panel-header" style={{ borderLeft: `4px solid ${color}` }}>
        <div>
          <span className="knowledge-graph__panel-type">{node.type || 'node'}</span>
          <h3 className="knowledge-graph__panel-title">{node.label}</h3>
        </div>
        <button
          className="knowledge-graph__panel-close"
          onClick={onClose}
          aria-label="Close detail panel"
        >
          x
        </button>
      </div>

      {/* Summary */}
      {node.summary && (
        <div className="knowledge-graph__panel-section">
          <p className="knowledge-graph__panel-summary">{node.summary}</p>
        </div>
      )}

      {/* Created date */}
      {node.created_at && (
        <div className="knowledge-graph__panel-section">
          <span className="knowledge-graph__panel-date" title={formatDate(node.created_at)}>
            Created {formatRelativeTime(node.created_at)}
          </span>
        </div>
      )}

      {/* Statistics */}
      <div className="knowledge-graph__panel-section knowledge-graph__panel-stats">
        <div className="knowledge-graph__panel-stat-item">
          <span className="knowledge-graph__panel-stat-value">{stats.degree}</span>
          <span className="knowledge-graph__panel-stat-label">connections</span>
        </div>
        {stats.degree > 0 && (
          <div className="knowledge-graph__panel-stat-item">
            <span className="knowledge-graph__panel-stat-value">
              {(stats.avgWeight * 100).toFixed(0)}%
            </span>
            <span className="knowledge-graph__panel-stat-label">avg strength</span>
          </div>
        )}
        {node.documentCount > 0 && (
          <div className="knowledge-graph__panel-stat-item">
            <span className="knowledge-graph__panel-stat-value">{node.documentCount}</span>
            <span className="knowledge-graph__panel-stat-label">documents</span>
          </div>
        )}
      </div>

      {/* Cluster info */}
      {clusterInfo && (
        <div className="knowledge-graph__panel-section">
          <h4 className="knowledge-graph__panel-section-title">Cluster</h4>
          <div className="knowledge-graph__panel-cluster">
            <span
              className="knowledge-graph__panel-cluster-badge"
              style={{ backgroundColor: clusterInfo.color }}
            />
            <span className="knowledge-graph__panel-cluster-label">{clusterInfo.label}</span>
            <span className="knowledge-graph__panel-cluster-size">({clusterInfo.size} nodes)</span>
          </div>
        </div>
      )}

      {/* Topics */}
      {node.topics && node.topics.length > 0 && (
        <div className="knowledge-graph__panel-section">
          <h4 className="knowledge-graph__panel-section-title">Topics</h4>
          <div className="knowledge-graph__panel-tags">
            {node.topics.map((topic) => (
              <span key={topic} className="knowledge-graph__panel-tag">
                {topic}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Related nodes */}
      {relatedNodes.length > 0 && (
        <div className="knowledge-graph__panel-section">
          <h4 className="knowledge-graph__panel-section-title">
            Connected ({relatedNodes.length})
          </h4>
          <ul className="knowledge-graph__panel-related">
            {relatedNodes.slice(0, 8).map((related) => (
              <li key={related.id} className="knowledge-graph__panel-related-item">
                <button
                  className="knowledge-graph__panel-related-btn"
                  onClick={() => onNavigate(related)}
                  style={{ borderLeft: `3px solid ${nodeColor(related.type)}` }}
                >
                  <span className="knowledge-graph__panel-related-label">{related.label}</span>
                  {related.weight !== undefined && (
                    <span className="knowledge-graph__panel-related-weight">
                      {(related.weight * 100).toFixed(0)}% match
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Document link */}
      {node.documentId && (
        <div className="knowledge-graph__panel-section">
          <a href={`/documents/${node.documentId}`} className="knowledge-graph__panel-link">
            View document
          </a>
        </div>
      )}

      {/* Quick actions */}
      <div className="knowledge-graph__panel-section knowledge-graph__panel-actions">
        <h4 className="knowledge-graph__panel-section-title">Actions</h4>
        <div className="knowledge-graph__panel-action-buttons">
          {onSetPathSource && (
            <button
              className="knowledge-graph__panel-action-btn"
              onClick={() => onSetPathSource(node)}
              title="Set this node as the starting point for path finding"
            >
              Find path from here
            </button>
          )}
          {onFilterToNode && stats.degree > 0 && (
            <button
              className="knowledge-graph__panel-action-btn"
              onClick={() => onFilterToNode(node)}
              title="Show only this node and its direct connections"
            >
              Show connections only
            </button>
          )}
          {onExportConnections && stats.degree > 0 && (
            <button
              className="knowledge-graph__panel-action-btn"
              onClick={() => onExportConnections(node, relatedNodes)}
              title="Download this node's connections as JSON"
            >
              Export connections
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

NodeDetailPanel.propTypes = {
  node: PropTypes.shape({
    id: PropTypes.string.isRequired,
    label: PropTypes.string.isRequired,
    type: PropTypes.string,
    summary: PropTypes.string,
    documentCount: PropTypes.number,
    documentId: PropTypes.string,
    topics: PropTypes.arrayOf(PropTypes.string),
    created_at: PropTypes.string,
  }),
  relatedEdges: PropTypes.arrayOf(
    PropTypes.shape({
      id: PropTypes.string.isRequired,
      source: PropTypes.string.isRequired,
      target: PropTypes.string.isRequired,
      weight: PropTypes.number,
      label: PropTypes.string,
    })
  ).isRequired,
  allNodes: PropTypes.array.isRequired,
  onClose: PropTypes.func.isRequired,
  onNavigate: PropTypes.func.isRequired,
  clusterMode: PropTypes.bool,
  clusterData: PropTypes.shape({
    clusters: PropTypes.array,
    node_cluster_map: PropTypes.object,
  }),
  onSetPathSource: PropTypes.func,
  onFilterToNode: PropTypes.func,
  onExportConnections: PropTypes.func,
};

NodeDetailPanel.defaultProps = {
  node: null,
  clusterMode: false,
  clusterData: null,
  onSetPathSource: null,
  onFilterToNode: null,
  onExportConnections: null,
};

export default NodeDetailPanel;
