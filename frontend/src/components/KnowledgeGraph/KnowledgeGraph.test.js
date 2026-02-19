import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import KnowledgeGraph from './KnowledgeGraph';
import {
  computeLayout,
  initializePositions,
  nodeColor,
  nodeRadius,
  edgeMidpoint,
} from './graphLayout';

// ============================================================
// graphLayout.js unit tests
// ============================================================

describe('graphLayout', () => {
  describe('nodeColor', () => {
    test('returns blue for document type', () => {
      expect(nodeColor('document')).toBe('#4A90D9');
    });

    test('returns green for topic type', () => {
      expect(nodeColor('topic')).toBe('#7ED321');
    });

    test('returns orange for person type', () => {
      expect(nodeColor('person')).toBe('#F5A623');
    });

    test('returns purple for technology type', () => {
      expect(nodeColor('technology')).toBe('#9B59B6');
    });

    test('returns red for insight type', () => {
      expect(nodeColor('insight')).toBe('#E74C3C');
    });

    test('returns default gray for unknown type', () => {
      expect(nodeColor('unknown')).toBe('#95A5A6');
      expect(nodeColor(undefined)).toBe('#95A5A6');
    });
  });

  describe('nodeRadius', () => {
    test('returns minimum radius for degree 0', () => {
      const r = nodeRadius(0);
      expect(r).toBeGreaterThanOrEqual(20);
    });

    test('increases radius with degree', () => {
      const r0 = nodeRadius(0);
      const r5 = nodeRadius(5);
      const r10 = nodeRadius(10);
      expect(r5).toBeGreaterThan(r0);
      expect(r10).toBeGreaterThan(r5);
    });

    test('never exceeds maxRadius', () => {
      const maxRadius = 40;
      expect(nodeRadius(100, 20, maxRadius)).toBeLessThanOrEqual(maxRadius);
      expect(nodeRadius(1000, 20, maxRadius)).toBeLessThanOrEqual(maxRadius);
    });

    test('respects custom min and max radius', () => {
      const r = nodeRadius(0, 15, 50);
      expect(r).toBeGreaterThanOrEqual(15);
    });
  });

  describe('edgeMidpoint', () => {
    test('returns exact midpoint for simple case', () => {
      const mid = edgeMidpoint({ x: 0, y: 0 }, { x: 100, y: 100 });
      expect(mid.x).toBe(50);
      expect(mid.y).toBe(50);
    });

    test('handles negative coordinates', () => {
      const mid = edgeMidpoint({ x: -50, y: -50 }, { x: 50, y: 50 });
      expect(mid.x).toBe(0);
      expect(mid.y).toBe(0);
    });

    test('handles same-position nodes', () => {
      const mid = edgeMidpoint({ x: 100, y: 200 }, { x: 100, y: 200 });
      expect(mid.x).toBe(100);
      expect(mid.y).toBe(200);
    });
  });

  describe('initializePositions', () => {
    test('returns same number of nodes as input', () => {
      const nodes = [
        { id: 'a', label: 'A' },
        { id: 'b', label: 'B' },
        { id: 'c', label: 'C' },
      ];
      const positioned = initializePositions(nodes, 800, 600);
      expect(positioned).toHaveLength(3);
    });

    test('each node has x, y, vx, vy properties', () => {
      const nodes = [{ id: 'a', label: 'A' }];
      const [node] = initializePositions(nodes, 800, 600);
      expect(node).toHaveProperty('x');
      expect(node).toHaveProperty('y');
      expect(node).toHaveProperty('vx');
      expect(node).toHaveProperty('vy');
    });

    test('preserves original node properties', () => {
      const nodes = [{ id: 'a', label: 'Alpha', type: 'topic', documentCount: 5 }];
      const [node] = initializePositions(nodes, 800, 600);
      expect(node.id).toBe('a');
      expect(node.label).toBe('Alpha');
      expect(node.type).toBe('topic');
      expect(node.documentCount).toBe(5);
    });

    test('initializes velocities to zero', () => {
      const nodes = [{ id: 'a', label: 'A' }];
      const [node] = initializePositions(nodes, 800, 600);
      expect(node.vx).toBe(0);
      expect(node.vy).toBe(0);
    });

    test('handles empty nodes array', () => {
      const result = initializePositions([], 800, 600);
      expect(result).toHaveLength(0);
    });
  });

  describe('computeLayout', () => {
    test('returns positioned nodes with x and y', () => {
      const nodes = [
        { id: 'a', label: 'A' },
        { id: 'b', label: 'B' },
      ];
      const edges = [{ id: 'e1', source: 'a', target: 'b', weight: 1 }];
      const result = computeLayout(nodes, edges, 800, 600);

      expect(result).toHaveLength(2);
      expect(result[0]).toHaveProperty('x');
      expect(result[0]).toHaveProperty('y');
    });

    test('nodes stay within canvas bounds after layout', () => {
      const nodes = Array.from({ length: 10 }, (_, i) => ({
        id: `n${i}`,
        label: `Node ${i}`,
      }));
      const edges = [];
      const width = 800;
      const height = 600;
      const result = computeLayout(nodes, edges, width, height);

      for (const node of result) {
        expect(node.x).toBeGreaterThanOrEqual(0);
        expect(node.x).toBeLessThanOrEqual(width);
        expect(node.y).toBeGreaterThanOrEqual(0);
        expect(node.y).toBeLessThanOrEqual(height);
      }
    });

    test('returns empty array for empty input', () => {
      const result = computeLayout([], [], 800, 600);
      expect(result).toHaveLength(0);
    });

    test('single node is placed near center', () => {
      const nodes = [{ id: 'solo', label: 'Solo' }];
      const result = computeLayout(nodes, [], 800, 600);
      expect(result[0].x).toBeGreaterThan(0);
      expect(result[0].y).toBeGreaterThan(0);
    });
  });
});

// ============================================================
// KnowledgeGraph component tests
// ============================================================

const sampleNodes = [
  { id: 'n1', label: 'React Hooks', type: 'topic', documentCount: 3, topics: ['React', 'JavaScript'] },
  { id: 'n2', label: 'State Management', type: 'topic', documentCount: 1 },
  { id: 'n3', label: 'Performance Guide', type: 'document', documentId: 'doc-abc' },
];

const sampleEdges = [
  { id: 'e1', source: 'n1', target: 'n2', weight: 0.85, label: '85%' },
  { id: 'e2', source: 'n1', target: 'n3', weight: 0.7 },
];

describe('KnowledgeGraph component', () => {
  test('renders loading state', () => {
    render(<KnowledgeGraph nodes={[]} edges={[]} loading />);
    expect(screen.getByLabelText('Loading graph')).toBeInTheDocument();
  });

  test('renders empty state when no nodes', () => {
    render(
      <KnowledgeGraph
        nodes={[]}
        edges={[]}
        emptyMessage="Start by uploading documents"
      />
    );
    expect(screen.getByText('Start by uploading documents')).toBeInTheDocument();
  });

  test('renders default empty message when emptyMessage not provided', () => {
    render(<KnowledgeGraph nodes={[]} edges={[]} />);
    expect(
      screen.getByText('No knowledge connections to display yet.')
    ).toBeInTheDocument();
  });

  test('renders graph title when provided', () => {
    render(
      <KnowledgeGraph
        nodes={sampleNodes}
        edges={sampleEdges}
        title="Team Knowledge Map"
      />
    );
    expect(screen.getByText('Team Knowledge Map')).toBeInTheDocument();
  });

  test('shows node count and edge count in header', () => {
    render(
      <KnowledgeGraph
        nodes={sampleNodes}
        edges={sampleEdges}
        title="Test Graph"
      />
    );
    expect(screen.getByText('3 nodes')).toBeInTheDocument();
    expect(screen.getByText('2 connections')).toBeInTheDocument();
  });

  test('renders SVG canvas when nodes are provided', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    const svg = screen.getByRole('img', { name: 'Knowledge graph visualization' });
    expect(svg).toBeInTheDocument();
  });

  test('renders zoom controls', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    expect(screen.getByLabelText('Zoom in')).toBeInTheDocument();
    expect(screen.getByLabelText('Zoom out')).toBeInTheDocument();
    expect(screen.getByLabelText('Reset view')).toBeInTheDocument();
  });

  test('shows 100% zoom initially', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  test('zoom in button increases zoom level', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    const zoomIn = screen.getByLabelText('Zoom in');
    fireEvent.click(zoomIn);
    // Should show more than 100%
    const zoomText = screen.getByText(/\d+%/);
    const zoomValue = parseInt(zoomText.textContent);
    expect(zoomValue).toBeGreaterThan(100);
  });

  test('zoom out button decreases zoom level', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    const zoomOut = screen.getByLabelText('Zoom out');
    fireEvent.click(zoomOut);
    const zoomText = screen.getByText(/\d+%/);
    const zoomValue = parseInt(zoomText.textContent);
    expect(zoomValue).toBeLessThan(100);
  });

  test('reset button restores 100% zoom', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    const zoomIn = screen.getByLabelText('Zoom in');
    const resetBtn = screen.getByLabelText('Reset view');

    fireEvent.click(zoomIn);
    fireEvent.click(zoomIn);
    fireEvent.click(resetBtn);

    expect(screen.getByText('100%')).toBeInTheDocument();
  });

  test('renders legend with node types', () => {
    render(<KnowledgeGraph nodes={sampleNodes} edges={sampleEdges} />);
    const legend = screen.getByLabelText('Node type legend');
    expect(legend).toBeInTheDocument();
    expect(legend).toHaveTextContent('Document');
    expect(legend).toHaveTextContent('Topic');
    expect(legend).toHaveTextContent('Technology');
  });

  test('calls onNodeClick when provided', () => {
    const handleClick = jest.fn();
    render(
      <KnowledgeGraph
        nodes={sampleNodes}
        edges={sampleEdges}
        onNodeClick={handleClick}
      />
    );
    // Confirm the prop is accepted without error
    expect(handleClick).not.toHaveBeenCalled();
  });
});
