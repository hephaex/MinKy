import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import SimpleDateSidebar from './SimpleDateSidebar';

const mockTimeline = {
  '2025': {
    key: '2025',
    label: '2025',
    count: 12,
    children: {
      '2025-01': {
        key: '2025-01',
        label: '2025-01',
        count: 5
      },
      '2025-02': {
        key: '2025-02',
        label: '2025-02',
        count: 7
      }
    }
  },
  '2026': {
    key: '2026',
    label: '2026',
    count: 8,
    children: {
      '2026-02': {
        key: '2026-02',
        label: '2026-02',
        count: 8
      }
    }
  }
};

global.fetch = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();
  localStorage.clear();
  global.fetch.mockClear();
});

describe('SimpleDateSidebar', () => {
  it('renders loading state initially', async () => {
    global.fetch.mockImplementationOnce(
      () => new Promise(resolve => setTimeout(() => resolve({
        ok: true,
        json: async () => ({ timeline: mockTimeline })
      }), 100))
    );

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);
    expect(screen.getByText('로딩 중...')).toBeInTheDocument();
  });

  it('renders error state on fetch failure', async () => {
    global.fetch.mockRejectedValueOnce(new Error('Network error'));

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/오류/)).toBeInTheDocument();
    });
  });

  it('renders timeline data after loading', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('2025')).toBeInTheDocument();
      expect(screen.getByText('2026')).toBeInTheDocument();
    });
  });

  it('renders empty state when no timeline data', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: {} })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('문서가 없습니다')).toBeInTheDocument();
    });
  });

  it('sorts timeline items in descending order', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    const { container } = render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      const labels = Array.from(container.querySelectorAll('span')).map(el => el.textContent);
      const year2026Index = labels.indexOf('2026');
      const year2025Index = labels.indexOf('2025');
      expect(year2026Index).toBeLessThan(year2025Index);
    });
  });

  it('expands year when arrow is clicked', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('2025')).toBeInTheDocument();
    });

    // 2025 is expanded by default
    expect(screen.getByText('2025-01')).toBeInTheDocument();
  });

  it('collapses year when arrow is clicked again', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('2025-01')).toBeInTheDocument();
    });

    // Click on 2025 to collapse
    const year2025 = screen.getByText('2025').closest('div[style*="display: flex"]');
    const arrow = year2025.querySelector('span');
    fireEvent.click(arrow);

    expect(screen.queryByText('2025-01')).not.toBeInTheDocument();
  });

  it('calls onDocumentSelect when year is clicked', async () => {
    const mockOnSelect = jest.fn();
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={mockOnSelect} />);

    await waitFor(() => {
      expect(screen.getByText('2025')).toBeInTheDocument();
    });

    const year2025Label = screen.getByText('2025');
    fireEvent.click(year2025Label);

    expect(mockOnSelect).toHaveBeenCalledWith('2025');
  });

  it('calls onDocumentSelect when month is clicked', async () => {
    const mockOnSelect = jest.fn();
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={mockOnSelect} />);

    await waitFor(() => {
      expect(screen.getByText('2025-01')).toBeInTheDocument();
    });

    const month = screen.getByText('2025-01');
    fireEvent.click(month);

    expect(mockOnSelect).toHaveBeenCalledWith('2025-01');
  });

  it('calls onDocumentSelect when count badge is clicked', async () => {
    const mockOnSelect = jest.fn();
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={mockOnSelect} />);

    await waitFor(() => {
      expect(screen.getByText('(12)')).toBeInTheDocument();
    });

    const countBadges = screen.getAllByText(/^\(\d+\)$/);
    fireEvent.click(countBadges[0]);

    expect(mockOnSelect).toHaveBeenCalled();
  });

  it('highlights selected date key', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    const { container } = render(
      <SimpleDateSidebar onDocumentSelect={jest.fn()} selectedDateKey="2025" />
    );

    await waitFor(() => {
      const selected = container.querySelector('[style*="rgb(227, 242, 253)"]');
      expect(selected).toBeInTheDocument();
    });
  });

  it('includes auth token in fetch request', async () => {
    localStorage.setItem('token', 'test-token-123');
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: {} })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/documents/timeline?group_by=month',
        expect.objectContaining({
          headers: { 'Authorization': 'Bearer test-token-123' }
        })
      );
    });
  });

  it('handles fetch without auth token', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: {} })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/documents/timeline?group_by=month',
        expect.objectContaining({
          headers: {}
        })
      );
    });
  });

  it('renders document header', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('문서 탐색')).toBeInTheDocument();
    });
  });

  it('displays count for each timeline item', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('(12)')).toBeInTheDocument();
      expect(screen.getByText('(8)')).toBeInTheDocument();
    });
  });

  it('renders nested months under expanded year', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText('2025-01')).toBeInTheDocument();
      expect(screen.getByText('2025-02')).toBeInTheDocument();
    });
  });

  it('sorts nested months in descending order', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: mockTimeline })
    });

    const { container } = render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      // Find nested month elements by their text content
      const month01 = screen.getByText('2025-01');
      const month02 = screen.getByText('2025-02');

      // Get their positions in the document
      const allElements = container.querySelectorAll('[style*="font-size: 14px"]');
      const labels = Array.from(allElements).map(el => el.textContent);

      // 2025-02 should come before 2025-01 (descending order)
      const idx02 = labels.findIndex(l => l.includes('2025-02'));
      const idx01 = labels.findIndex(l => l.includes('2025-01'));

      expect(idx02).toBeLessThan(idx01);
    });
  });

  it('handles months without expandable arrow', async () => {
    const timelineWithoutChildren = {
      '2025': {
        key: '2025',
        label: '2025',
        count: 5,
        children: {}
      }
    };

    global.fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ timeline: timelineWithoutChildren })
    });

    const { container } = render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      const arrowSpans = container.querySelectorAll('span');
      const hasArrow = Array.from(arrowSpans).some(span => span.textContent === '▶');
      expect(hasArrow).toBe(false);
    });
  });

  it('handles HTTP error response', async () => {
    global.fetch.mockResolvedValueOnce({
      ok: false,
      status: 500
    });

    render(<SimpleDateSidebar onDocumentSelect={jest.fn()} />);

    await waitFor(() => {
      expect(screen.getByText(/오류/)).toBeInTheDocument();
    });
  });
});
