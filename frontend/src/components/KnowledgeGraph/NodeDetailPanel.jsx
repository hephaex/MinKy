import React from 'react';
import PropTypes from 'prop-types';
import { nodeColor } from './graphLayout';

/**
 * Side panel showing details of the selected graph node.
 * Displays metadata and links to related documents.
 */
function NodeDetailPanel({ node, relatedEdges, allNodes, onClose, onNavigate }) {
  if (!node) return null;

  const color = nodeColor(node.type);

  const relatedNodes = relatedEdges
    .map(edge => {
      const otherId = edge.source === node.id ? edge.target : edge.source;
      const otherNode = allNodes.find(n => n.id === otherId);
      return otherNode
        ? { ...otherNode, weight: edge.weight, edgeLabel: edge.label }
        : null;
    })
    .filter(Boolean)
    .sort((a, b) => (b.weight || 0) - (a.weight || 0));

  return (
    <div className="knowledge-graph__panel">
      {/* Header */}
      <div
        className="knowledge-graph__panel-header"
        style={{ borderLeft: `4px solid ${color}` }}
      >
        <div>
          <span className="knowledge-graph__panel-type">
            {node.type || 'node'}
          </span>
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

      {/* Document count */}
      {node.documentCount > 0 && (
        <div className="knowledge-graph__panel-section">
          <span className="knowledge-graph__panel-stat">
            {node.documentCount} document{node.documentCount !== 1 ? 's' : ''}
          </span>
        </div>
      )}

      {/* Topics */}
      {node.topics && node.topics.length > 0 && (
        <div className="knowledge-graph__panel-section">
          <h4 className="knowledge-graph__panel-section-title">Topics</h4>
          <div className="knowledge-graph__panel-tags">
            {node.topics.map(topic => (
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
            {relatedNodes.slice(0, 8).map(related => (
              <li key={related.id} className="knowledge-graph__panel-related-item">
                <button
                  className="knowledge-graph__panel-related-btn"
                  onClick={() => onNavigate(related)}
                  style={{ borderLeft: `3px solid ${nodeColor(related.type)}` }}
                >
                  <span className="knowledge-graph__panel-related-label">
                    {related.label}
                  </span>
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
          <a
            href={`/documents/${node.documentId}`}
            className="knowledge-graph__panel-link"
          >
            View document
          </a>
        </div>
      )}
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
};

NodeDetailPanel.defaultProps = {
  node: null,
};

export default NodeDetailPanel;
