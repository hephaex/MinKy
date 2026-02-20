/**
 * Force-directed graph layout using Fruchterman-Reingold algorithm
 * Pure JavaScript implementation, no external dependencies required.
 */

const REPULSION = 5000;
const ATTRACTION = 0.05;
const DAMPING = 0.85;
const MIN_DISTANCE = 40;
const MAX_ITERATIONS = 200;

/**
 * Initialize node positions randomly within the canvas bounds.
 * Returns new node objects (immutable pattern).
 */
export function initializePositions(nodes, width, height) {
  const centerX = width / 2;
  const centerY = height / 2;
  const radius = Math.min(width, height) * 0.35;

  return nodes.map((node, index) => {
    const angle = (2 * Math.PI * index) / nodes.length;
    return {
      ...node,
      x: centerX + radius * Math.cos(angle) + (Math.random() - 0.5) * 20,
      y: centerY + radius * Math.sin(angle) + (Math.random() - 0.5) * 20,
      vx: 0,
      vy: 0,
    };
  });
}

/**
 * Run one iteration of the force-directed layout.
 * Returns new positions without mutating the input.
 */
function runIteration(nodes, edges, width, height) {
  const nodeCount = nodes.length;
  const forces = nodes.map(() => ({ fx: 0, fy: 0 }));

  // Repulsion forces between all node pairs
  for (let i = 0; i < nodeCount; i++) {
    for (let j = i + 1; j < nodeCount; j++) {
      const dx = nodes[i].x - nodes[j].x;
      const dy = nodes[i].y - nodes[j].y;
      const distSq = dx * dx + dy * dy;
      const dist = Math.max(Math.sqrt(distSq), MIN_DISTANCE);

      const force = REPULSION / distSq;
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;

      forces[i].fx += fx;
      forces[i].fy += fy;
      forces[j].fx -= fx;
      forces[j].fy -= fy;
    }
  }

  // Attraction forces along edges
  for (const edge of edges) {
    const sourceIndex = nodes.findIndex(n => n.id === edge.source);
    const targetIndex = nodes.findIndex(n => n.id === edge.target);

    if (sourceIndex === -1 || targetIndex === -1) continue;

    const dx = nodes[targetIndex].x - nodes[sourceIndex].x;
    const dy = nodes[targetIndex].y - nodes[sourceIndex].y;
    const dist = Math.max(Math.sqrt(dx * dx + dy * dy), 1);
    const strength = edge.weight !== undefined ? edge.weight : 1;

    // Normalize force by distance
    const fx = (dx / dist) * ATTRACTION * strength * dist;
    const fy = (dy / dist) * ATTRACTION * strength * dist;

    forces[sourceIndex].fx += fx;
    forces[sourceIndex].fy += fy;
    forces[targetIndex].fx -= fx;
    forces[targetIndex].fy -= fy;
  }

  // Apply forces with damping and boundary constraints
  return nodes.map((node, i) => {
    const newVx = (node.vx + forces[i].fx) * DAMPING;
    const newVy = (node.vy + forces[i].fy) * DAMPING;

    const padding = 60;
    const newX = Math.max(padding, Math.min(width - padding, node.x + newVx));
    const newY = Math.max(padding, Math.min(height - padding, node.y + newVy));

    return {
      ...node,
      x: newX,
      y: newY,
      vx: newVx,
      vy: newVy,
    };
  });
}

/**
 * Compute the full force-directed layout for the given graph.
 * Returns positioned nodes after convergence.
 */
export function computeLayout(nodes, edges, width, height) {
  if (nodes.length === 0) return [];

  let positioned = initializePositions(nodes, width, height);

  for (let i = 0; i < MAX_ITERATIONS; i++) {
    positioned = runIteration(positioned, edges, width, height);
  }

  return positioned;
}

/**
 * Calculate the midpoint between two points (for edge labels).
 */
export function edgeMidpoint(source, target) {
  return {
    x: (source.x + target.x) / 2,
    y: (source.y + target.y) / 2,
  };
}

/**
 * Determine node radius based on connection count (degree).
 */
export function nodeRadius(degree, minRadius = 20, maxRadius = 45) {
  const base = minRadius + Math.sqrt(degree) * 5;
  return Math.min(base, maxRadius);
}

/**
 * Assign colors to nodes based on their type/category.
 */
export function nodeColor(type) {
  const colorMap = {
    document: '#4A90D9',
    topic: '#7ED321',
    person: '#F5A623',
    technology: '#9B59B6',
    insight: '#E74C3C',
    default: '#95A5A6',
  };
  return colorMap[type] || colorMap.default;
}
