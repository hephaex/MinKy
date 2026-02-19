import React from 'react';
import { render, screen } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import DocumentCard from './DocumentCard';

const mockDocument = {
  id: 1,
  title: 'Test Document',
  author: 'John Doe',
  tags: ['tag1', 'tag2', 'tag3'],
  updated_at: '2026-02-19T10:00:00Z',
  markdown_content: 'This is a test document with some content.'
};

const renderWithRouter = (component) => {
  return render(<BrowserRouter>{component}</BrowserRouter>);
};

describe('DocumentCard', () => {
  it('renders document title', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    expect(screen.getByText('Test Document')).toBeInTheDocument();
  });

  it('renders document author', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    expect(screen.getByText('John Doe')).toBeInTheDocument();
  });

  it('renders updated date', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    expect(screen.getByText(/Updated/)).toBeInTheDocument();
  });

  it('renders tags up to max limit', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    expect(screen.getByText('tag1')).toBeInTheDocument();
    expect(screen.getByText('tag2')).toBeInTheDocument();
    expect(screen.getByText('tag3')).toBeInTheDocument();
  });

  it('shows overflow indicator when tags exceed limit', () => {
    const docWithManyTags = {
      ...mockDocument,
      tags: ['tag1', 'tag2', 'tag3', 'tag4', 'tag5']
    };
    renderWithRouter(<DocumentCard document={docWithManyTags} />);
    expect(screen.getByText('+2')).toBeInTheDocument();
  });

  it('does not show overflow when tags equal limit', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    expect(screen.queryByText(/^\+/)).not.toBeInTheDocument();
  });

  it('renders preview when showPreview is true', () => {
    renderWithRouter(
      <DocumentCard document={mockDocument} showPreview={true} />
    );
    expect(screen.getByText(/This is a test document/)).toBeInTheDocument();
  });

  it('does not render preview when showPreview is false', () => {
    renderWithRouter(
      <DocumentCard document={mockDocument} showPreview={false} />
    );
    expect(screen.queryByText(/This is a test document/)).not.toBeInTheDocument();
  });

  it('renders preview with truncation indicator', () => {
    const docWithLongContent = {
      ...mockDocument,
      markdown_content: 'A'.repeat(200)
    };
    renderWithRouter(
      <DocumentCard document={docWithLongContent} showPreview={true} />
    );
    expect(screen.getByText(/\.\.\./)).toBeInTheDocument();
  });

  it('links to document detail page', () => {
    renderWithRouter(<DocumentCard document={mockDocument} />);
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/documents/1');
  });

  it('handles author as array', () => {
    const docWithArrayAuthor = {
      ...mockDocument,
      author: ['Jane Smith', 'John Doe']
    };
    renderWithRouter(<DocumentCard document={docWithArrayAuthor} />);
    expect(screen.getByText('Jane Smith')).toBeInTheDocument();
  });

  it('handles author as JSON string', () => {
    const docWithJsonAuthor = {
      ...mockDocument,
      author: '["Alice"]'
    };
    renderWithRouter(<DocumentCard document={docWithJsonAuthor} />);
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('removes wiki link brackets from author', () => {
    const docWithWikiAuthor = {
      ...mockDocument,
      author: '[[John Doe]]'
    };
    renderWithRouter(<DocumentCard document={docWithWikiAuthor} />);
    expect(screen.getByText('John Doe')).toBeInTheDocument();
    expect(screen.queryByText(/\[\[/)).not.toBeInTheDocument();
  });

  it('removes quotes from author', () => {
    const docWithQuotedAuthor = {
      ...mockDocument,
      author: '"John Doe"'
    };
    renderWithRouter(<DocumentCard document={docWithQuotedAuthor} />);
    expect(screen.getByText('John Doe')).toBeInTheDocument();
  });

  it('handles missing author gracefully', () => {
    const docNoAuthor = {
      ...mockDocument,
      author: null
    };
    renderWithRouter(<DocumentCard document={docNoAuthor} />);
    expect(screen.queryByText('•')).not.toBeInTheDocument();
  });

  it('handles empty author string', () => {
    const docEmptyAuthor = {
      ...mockDocument,
      author: ''
    };
    renderWithRouter(<DocumentCard document={docEmptyAuthor} />);
    expect(screen.queryByText('•')).not.toBeInTheDocument();
  });

  it('renders tags as objects with name property', () => {
    const docWithTagObjects = {
      ...mockDocument,
      tags: [
        { name: 'react' },
        { name: 'javascript' },
        { name: 'frontend' }
      ]
    };
    renderWithRouter(<DocumentCard document={docWithTagObjects} />);
    expect(screen.getByText('react')).toBeInTheDocument();
    expect(screen.getByText('javascript')).toBeInTheDocument();
  });

  it('handles missing tags gracefully', () => {
    const docNoTags = {
      ...mockDocument,
      tags: null
    };
    renderWithRouter(<DocumentCard document={docNoTags} />);
    expect(screen.queryByText('tag')).not.toBeInTheDocument();
  });

  it('handles empty tags array', () => {
    const docEmptyTags = {
      ...mockDocument,
      tags: []
    };
    renderWithRouter(<DocumentCard document={docEmptyTags} />);
    expect(screen.queryByText('tag')).not.toBeInTheDocument();
  });

  it('uses custom formatDate function if provided', () => {
    const customFormatter = jest.fn(() => 'Custom Date');
    renderWithRouter(
      <DocumentCard document={mockDocument} formatDate={customFormatter} />
    );
    expect(screen.getByText(/Custom Date/)).toBeInTheDocument();
  });

  it('highlights search query in title', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDocument} searchQuery="Test" />
    );
    // When searchQuery is provided, text may be split into multiple spans for highlighting
    const titleElement = container.querySelector('.document-title');
    expect(titleElement).toHaveTextContent('Test Document');
  });

  it('highlights search query in author', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDocument} searchQuery="John" />
    );
    // Author text may be split due to highlighting
    const authorElement = container.querySelector('.document-author');
    expect(authorElement).toHaveTextContent('John Doe');
  });

  it('handles missing markdown_content in preview', () => {
    const docNoContent = {
      ...mockDocument,
      markdown_content: null
    };
    renderWithRouter(
      <DocumentCard document={docNoContent} showPreview={true} />
    );
    expect(screen.queryByText(/content/)).not.toBeInTheDocument();
  });

  it('renders document icon', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDocument} />
    );
    const icon = container.querySelector('.document-icon');
    expect(icon).toBeInTheDocument();
  });

  it('applies correct CSS classes', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDocument} />
    );
    expect(container.querySelector('.document-card')).toBeInTheDocument();
    expect(container.querySelector('.document-title')).toBeInTheDocument();
    expect(container.querySelector('.document-meta')).toBeInTheDocument();
  });
});
