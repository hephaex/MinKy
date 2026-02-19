import React from 'react';
import PropTypes from 'prop-types';
import { nodeColor, nodeRadius } from './graphLayout';

/**
 * A single node in the knowledge graph SVG.
 * Renders a circle with label and handles hover/click interactions.
 */
function GraphNode({ node, degree, isSelected, isHighlighted, onSelect, onHover }) {
  const radius = nodeRadius(degree);
  const color = nodeColor(node.type);
  const labelMaxLength = 18;
  const label =
    node.label.length > labelMaxLength
      ? node.label.slice(0, labelMaxLength - 3) + '...'
      : node.label;

  const opacity = isHighlighted ? 1.0 : 0.7;
  const strokeWidth = isSelected ? 3 : 1.5;
  const strokeColor = isSelected ? '#FFD700' : '#FFFFFF';

  const handleClick = () => onSelect(node);
  const handleMouseEnter = () => onHover(node);
  const handleMouseLeave = () => onHover(null);

  return (
    <g
      transform={`translate(${node.x}, ${node.y})`}
      style={{ cursor: 'pointer' }}
      onClick={handleClick}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      aria-label={`Node: ${node.label}`}
      role="button"
      tabIndex={0}
      onKeyDown={e => e.key === 'Enter' && handleClick()}
    >
      {/* Shadow for depth */}
      <circle
        cx={2}
        cy={2}
        r={radius}
        fill="rgba(0, 0, 0, 0.2)"
        style={{ pointerEvents: 'none' }}
      />

      {/* Main circle */}
      <circle
        r={radius}
        fill={color}
        opacity={opacity}
        stroke={strokeColor}
        strokeWidth={strokeWidth}
      />

      {/* Pulse ring when selected */}
      {isSelected && (
        <circle
          r={radius + 6}
          fill="none"
          stroke="#FFD700"
          strokeWidth={1.5}
          opacity={0.5}
        />
      )}

      {/* Node label */}
      <text
        dy="0.35em"
        textAnchor="middle"
        fill="#FFFFFF"
        fontSize={radius > 30 ? 11 : 9}
        fontWeight="600"
        style={{ pointerEvents: 'none', userSelect: 'none' }}
      >
        {label}
      </text>

      {/* Document count badge */}
      {node.documentCount > 0 && (
        <>
          <circle
            cx={radius - 5}
            cy={-(radius - 5)}
            r={9}
            fill="#E74C3C"
            stroke="#FFFFFF"
            strokeWidth={1.5}
          />
          <text
            x={radius - 5}
            y={-(radius - 5)}
            dy="0.35em"
            textAnchor="middle"
            fill="#FFFFFF"
            fontSize={8}
            fontWeight="700"
            style={{ pointerEvents: 'none' }}
          >
            {node.documentCount > 99 ? '99+' : node.documentCount}
          </text>
        </>
      )}
    </g>
  );
}

GraphNode.propTypes = {
  node: PropTypes.shape({
    id: PropTypes.string.isRequired,
    label: PropTypes.string.isRequired,
    type: PropTypes.string,
    documentCount: PropTypes.number,
    x: PropTypes.number,
    y: PropTypes.number,
  }).isRequired,
  degree: PropTypes.number.isRequired,
  isSelected: PropTypes.bool.isRequired,
  isHighlighted: PropTypes.bool.isRequired,
  onSelect: PropTypes.func.isRequired,
  onHover: PropTypes.func.isRequired,
};

export default GraphNode;
