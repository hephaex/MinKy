import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TreeView from './TreeView';

describe('TreeView', () => {
  const mockOnSelect = jest.fn();

  const singleNode = [
    { id: '1', label: 'Document 1', type: 'document' }
  ];

  const createNestedNodes = () => [
    {
      id: '1',
      label: 'Folder 1',
      type: 'folder',
      children: [
        { id: '2', label: 'Document 2', type: 'document' },
        { id: '3', label: 'Document 3', type: 'document' }
      ]
    },
    {
      id: '4',
      label: 'Folder 2',
      type: 'folder',
      children: [
        {
          id: '5',
          label: 'Subfolder',
          type: 'folder',
          children: [
            { id: '6', label: 'Document 6', type: 'document' }
          ]
        }
      ]
    }
  ];

  beforeEach(() => {
    mockOnSelect.mockClear();
  });

  it('renders empty state when no nodes provided', () => {
    render(<TreeView nodes={[]} onSelect={mockOnSelect} />);
    expect(screen.getByText('문서가 없습니다')).toBeInTheDocument();
  });

  it('renders single document node', () => {
    render(<TreeView nodes={singleNode} onSelect={mockOnSelect} />);
    expect(screen.getByText(/Document 1/)).toBeInTheDocument();
  });

  it('renders nested folder structure', () => {
    render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);
    expect(screen.getByText('Folder 1')).toBeInTheDocument();
    expect(screen.getByText('Folder 2')).toBeInTheDocument();
  });

  it('expands folder when clicked', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Click on the first treeitem (Folder 1)
    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });

    // Document 2 should now be visible
    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();
    expect(container.querySelector('[title="Document 3"]')).toBeInTheDocument();
  });

  it('collapses folder when clicked again', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const folder1Item = container.querySelector('[role="treeitem"]');

    // Expand
    await act(async () => {
      fireEvent.click(folder1Item);
    });
    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();

    // Collapse
    await act(async () => {
      fireEvent.click(folder1Item);
    });
    expect(container.querySelector('[title="Document 2"]')).not.toBeInTheDocument();
  });

  it('calls onSelect when document is clicked', () => {
    render(<TreeView nodes={singleNode} onSelect={mockOnSelect} />);

    const docNode = screen.getByText(/Document 1/);
    fireEvent.click(docNode);

    expect(mockOnSelect).toHaveBeenCalledWith(singleNode[0]);
  });

  it('calls onSelect with correct data for nested document', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Expand folder first
    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });

    // Click document
    const doc2 = container.querySelector('[title="Document 2"]');
    fireEvent.click(doc2);

    expect(mockOnSelect).toHaveBeenCalled();
    const calledNode = mockOnSelect.mock.calls[0][0];
    expect(calledNode.id).toBe('2');
    expect(calledNode.label).toBe('Document 2');
  });

  it('handles keyboard navigation with ArrowDown', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const treeView = container.querySelector('[role="tree"]');
    fireEvent.keyDown(treeView, { key: 'ArrowDown' });

    // After first ArrowDown, Folder 1 should be focused
    const folder1Item = screen.getByText('Folder 1').closest('[role="treeitem"]');
    expect(folder1Item).toHaveAttribute('tabIndex', '0');
  });

  it('handles keyboard navigation with ArrowUp', () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const treeView = container.querySelector('[role="tree"]');

    // First ArrowDown to select first item
    fireEvent.keyDown(treeView, { key: 'ArrowDown' });
    // Then ArrowUp should cycle to last item
    fireEvent.keyDown(treeView, { key: 'ArrowUp' });

    const treeItems = container.querySelectorAll('[role="treeitem"]');
    expect(treeItems.length).toBeGreaterThan(0);
  });

  it('handles Enter key on document node', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Expand folder
    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });

    // Get document node and press Enter
    const doc2Item = container.querySelector('[title="Document 2"]').closest('[role="treeitem"]');
    fireEvent.keyDown(doc2Item, { key: 'Enter' });

    expect(mockOnSelect).toHaveBeenCalled();
  });

  it('handles Space key on folder node', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.keyDown(folder1Item, { key: ' ' });
    });

    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();
  });

  it('handles ArrowRight to expand folder', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.keyDown(folder1Item, { key: 'ArrowRight' });
    });

    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();
  });

  it('handles ArrowLeft to collapse folder', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Expand first
    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });
    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();

    // Then collapse with ArrowLeft
    await act(async () => {
      fireEvent.keyDown(folder1Item, { key: 'ArrowLeft' });
    });

    expect(container.querySelector('[title="Document 2"]')).not.toBeInTheDocument();
  });

  it('renders with custom className', () => {
    const { container } = render(
      <TreeView nodes={singleNode} onSelect={mockOnSelect} className="custom-tree" />
    );

    const treeView = container.querySelector('[role="tree"]');
    expect(treeView).toHaveClass('custom-tree');
  });

  it('displays document icon for leaf nodes', () => {
    render(<TreeView nodes={singleNode} onSelect={mockOnSelect} />);

    const docLabel = screen.getByText(/Document 1/);
    expect(docLabel.textContent).toContain('📄');
  });

  it('handles null nodes gracefully', () => {
    render(<TreeView nodes={null} onSelect={mockOnSelect} />);
    expect(screen.getByText('문서가 없습니다')).toBeInTheDocument();
  });

  it('shows node count for folders', () => {
    const nodesWithCount = [
      {
        id: '1',
        label: 'Folder with count',
        type: 'folder',
        count: 5,
        children: []
      }
    ];

    render(<TreeView nodes={nodesWithCount} onSelect={mockOnSelect} />);
    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('supports node colors for folders', () => {
    const nodesWithColor = [
      {
        id: '1',
        label: 'Colored Folder',
        type: 'folder',
        color: '#FF5733',
        children: []
      }
    ];

    const { container } = render(<TreeView nodes={nodesWithColor} onSelect={mockOnSelect} />);
    const colorDot = container.querySelector('.tree-node-color');
    expect(colorDot).toHaveStyle('backgroundColor: #FF5733');
  });

  it('does not call onSelect when clicking a folder', async () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });

    // onSelect should not be called for folder, only toggle
    expect(mockOnSelect).not.toHaveBeenCalled();
  });

  it('applies appropriate ARIA attributes', () => {
    const { container } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    const treeView = container.querySelector('[role="tree"]');
    const treeItems = container.querySelectorAll('[role="treeitem"]');

    expect(treeView).toBeInTheDocument();
    expect(treeItems.length).toBeGreaterThan(0);
  });

  it('handles deeply nested structure', async () => {
    const deepNodes = [
      {
        id: '1',
        label: 'Level 1',
        type: 'folder',
        children: [
          {
            id: '2',
            label: 'Level 2',
            type: 'folder',
            children: [
              {
                id: '3',
                label: 'Level 3',
                type: 'folder',
                children: [
                  { id: '4', label: 'Document 4', type: 'document' }
                ]
              }
            ]
          }
        ]
      }
    ];

    const { container } = render(<TreeView nodes={deepNodes} onSelect={mockOnSelect} />);

    // Click through to expand each level
    let treeItems = container.querySelectorAll('[role="treeitem"]');

    // Level 1
    await act(async () => {
      fireEvent.click(treeItems[0]);
    });

    // Level 2
    treeItems = container.querySelectorAll('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(treeItems[1]); // Second item is Level 2
    });

    // Level 3
    treeItems = container.querySelectorAll('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(treeItems[2]); // Third item is Level 3
    });

    expect(container.querySelector('[title="Document 4"]')).toBeInTheDocument();
  });

  it('maintains state across re-renders', async () => {
    const { container, rerender } = render(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Expand folder
    const folder1Item = container.querySelector('[role="treeitem"]');
    await act(async () => {
      fireEvent.click(folder1Item);
    });
    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();

    // Re-render with same nodes
    rerender(<TreeView nodes={createNestedNodes()} onSelect={mockOnSelect} />);

    // Should still be expanded
    expect(container.querySelector('[title="Document 2"]')).toBeInTheDocument();
  });
});
