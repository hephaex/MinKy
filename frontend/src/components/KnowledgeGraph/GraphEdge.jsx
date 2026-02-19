import React from 'react';
import PropTypes from 'prop-types';
import { edgeMidpoint } from './graphLayout';

/**
 * A single edge (connection) in the knowledge graph.
 * Renders as an SVG line with optional weight-based thickness.
 */
function GraphEdge({ edge, sourceNode, targetNode, isHighlighted }) {
  if (!sourceNode || !targetNode) return null;

  const mid = edgeMidpoint(sourceNode, targetNode);
  const weight = edge.weight !== undefined ? edge.weight : 1;
  const strokeWidth = Math.max(1, Math.min(weight * 2, 6));
  const opacity = isHighlighted ? 0.9 : 0.35;

  const strokeColor = isHighlighted ? '#F39C12' : '#7F8C8D';

  return (
    <g>
      <line
        x1={sourceNode.x}
        y1={sourceNode.y}
        x2={targetNode.x}
        y2={targetNode.y}
        stroke={strokeColor}
        strokeWidth={strokeWidth}
        opacity={opacity}
        strokeLinecap="round"
      />

      {/* Edge label (similarity score) shown when highlighted */}
      {isHighlighted && edge.label && (
        <text
          x={mid.x}
          y={mid.y}
          dy="-4"
          textAnchor="middle"
          fill="#F39C12"
          fontSize={9}
          fontWeight="600"
          style={{ pointerEvents: 'none', userSelect: 'none' }}
        >
          {edge.label}
        </text>
      )}
    </g>
  );
}

GraphEdge.propTypes = {
  edge: PropTypes.shape({
    id: PropTypes.string.isRequired,
    source: PropTypes.string.isRequired,
    target: PropTypes.string.isRequired,
    weight: PropTypes.number,
    label: PropTypes.string,
  }).isRequired,
  sourceNode: PropTypes.shape({
    x: PropTypes.number,
    y: PropTypes.number,
  }),
  targetNode: PropTypes.shape({
    x: PropTypes.number,
    y: PropTypes.number,
  }),
  isHighlighted: PropTypes.bool.isRequired,
};

GraphEdge.defaultProps = {
  sourceNode: null,
  targetNode: null,
};

export default GraphEdge;
