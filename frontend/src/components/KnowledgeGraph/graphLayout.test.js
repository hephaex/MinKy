import { computeLayout } from './graphLayout';

describe('graphLayout', () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it('never produces NaN coordinates when nodes start coincident', () => {
    // With jitter zeroed and a 0x0 canvas every node initializes at the exact
    // same point — distSq becomes 0 and REPULSION/distSq must not explode into
    // Infinity (0 * Infinity = NaN propagated to every edge on the real graph).
    jest.spyOn(Math, 'random').mockReturnValue(0.5);

    const nodes = [
      { id: 'a', label: 'A', type: 'document' },
      { id: 'b', label: 'B', type: 'document' },
      { id: 'c', label: 'C', type: 'topic' },
    ];
    const edges = [
      { id: 'e1', source: 'a', target: 'b', weight: 0.99 },
      { id: 'e2', source: 'b', target: 'c', weight: 0.99 },
    ];

    const positioned = computeLayout(nodes, edges, 0, 0);

    for (const node of positioned) {
      expect(Number.isFinite(node.x)).toBe(true);
      expect(Number.isFinite(node.y)).toBe(true);
    }
  });

  it('keeps distinct starting positions finite on a normal canvas', () => {
    const nodes = Array.from({ length: 20 }, (_, i) => ({
      id: `n${i}`,
      label: `N${i}`,
      type: 'document',
    }));
    const edges = nodes.slice(1).map((n, i) => ({
      id: `e${i}`,
      source: 'n0',
      target: n.id,
      weight: 1,
    }));

    const positioned = computeLayout(nodes, edges, 1200, 800);

    for (const node of positioned) {
      expect(Number.isFinite(node.x)).toBe(true);
      expect(Number.isFinite(node.y)).toBe(true);
    }
  });
});
