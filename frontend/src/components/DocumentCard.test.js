import { render, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import DocumentCard from './DocumentCard';

const renderWithRouter = (component) => {
  return render(<BrowserRouter>{component}</BrowserRouter>);
};

describe('DocumentCard', () => {
  const mockDoc = (overrides = {}) => ({
    id: 1,
    title: 'Test Document',
    tags: [],
    updated_at: '2025-04-23T12:00:00Z',
    author: null,
    processing_status: null,
    markdown_content: null,
    ...overrides,
  });

  it('renders card with title and link', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc()} />);
    expect(container.querySelector('h3.document-title')).toHaveTextContent('Test Document');
    expect(container.querySelector('a.document-link')).toHaveAttribute('href', '/documents/1');
  });

  it('renders author when provided', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ author: 'Alice' })} />
    );
    expect(container.querySelector('.document-author')).toHaveTextContent('Alice');
  });

  it('does not render author when absent', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc()} />);
    expect(container.querySelector('.document-author')).not.toBeInTheDocument();
  });

  it('renders tags when provided', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ tags: ['rust', 'axum'] })} />
    );
    const tagEls = container.querySelectorAll('.document-tag');
    expect(tagEls.length).toBe(2);
    expect(tagEls[0]).toHaveTextContent('rust');
    expect(tagEls[1]).toHaveTextContent('axum');
  });

  it('renders tag overflow indicator beyond 3 tags', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ tags: ['a', 'b', 'c', 'd', 'e'] })} />
    );
    const overflow = container.querySelector('.document-tag-overflow');
    expect(overflow).toBeInTheDocument();
    expect(overflow).toHaveTextContent('+2');
  });

  it('renders no tags block when tags is empty', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc({ tags: [] })} />);
    expect(container.querySelector('.document-tags')).not.toBeInTheDocument();
  });

  it('renders no tags block when tags is null', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc({ tags: null })} />);
    expect(container.querySelector('.document-tags')).not.toBeInTheDocument();
  });

  it('renders tags from Rust tag_names when tags is absent', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ tags: undefined, tag_names: ['rag', 'pgvector'] })} />
    );
    const tagEls = container.querySelectorAll('.document-tag');
    expect(tagEls.length).toBe(2);
    expect(tagEls[0]).toHaveTextContent('rag');
    expect(tagEls[1]).toHaveTextContent('pgvector');
  });

  it('renders excerpt as plain-text preview without markdown syntax', () => {
    const { container } = renderWithRouter(
      <DocumentCard
        document={mockDoc({
          excerpt:
            '# 샘플 문서\n\nUI 테스트용 문서입니다.\n\n- pgvector 임베딩\n\n```rust\nfn main() {}\n```',
        })}
      />
    );
    const preview = container.querySelector('.document-preview');
    expect(preview).toBeInTheDocument();
    expect(preview.textContent).toContain('샘플 문서');
    expect(preview.textContent).toContain('UI 테스트용 문서입니다.');
    expect(preview.textContent).not.toContain('#');
    expect(preview.textContent).not.toContain('```');
  });

  it('does not render preview block when excerpt is absent and showPreview is false', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc()} />);
    expect(container.querySelector('.document-preview')).not.toBeInTheDocument();
  });

  it('renders pending processing badge', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ processing_status: 'pending' })} />
    );
    expect(container.querySelector('.processing-badge--pending')).toBeInTheDocument();
    expect(container.querySelector('.processing-badge--pending')).toHaveTextContent('Pending');
  });

  it('renders failed processing badge without onReprocess', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ processing_status: 'failed' })} />
    );
    const badge = container.querySelector('.processing-badge--failed');
    expect(badge).toBeInTheDocument();
    expect(badge.tagName).toBe('SPAN');
    expect(badge).toHaveTextContent('Failed');
  });

  it('renders failed processing badge as button with onReprocess and fires callback', () => {
    const onReprocess = jest.fn();
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ processing_status: 'failed' })} onReprocess={onReprocess} />
    );
    const btn = container.querySelector('button.processing-badge--failed');
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);
    expect(onReprocess).toHaveBeenCalledWith(1);
  });

  it('renders completed (Embedded) processing badge', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ processing_status: 'completed' })} />
    );
    expect(container.querySelector('.processing-badge--completed')).toHaveTextContent('Embedded');
  });

  it('renders no processing badge when status is null', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc()} />);
    expect(container.querySelector('.processing-badge')).not.toBeInTheDocument();
  });

  it('renders preview text when showPreview is true and content exists', () => {
    const { container } = renderWithRouter(
      <DocumentCard
        document={mockDoc({ markdown_content: 'Hello world preview content' })}
        showPreview
      />
    );
    expect(container.querySelector('.document-preview')).toHaveTextContent(
      'Hello world preview content'
    );
  });

  it('truncates preview text beyond 150 chars with ellipsis', () => {
    const long = 'x'.repeat(200);
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ markdown_content: long })} showPreview />
    );
    const preview = container.querySelector('.document-preview');
    expect(preview.textContent).toHaveLength(153); // 150 + '...'
  });

  it('does not render preview block when showPreview is false', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ markdown_content: 'Some content' })} showPreview={false} />
    );
    expect(container.querySelector('.document-preview')).not.toBeInTheDocument();
  });

  it('does not render preview block when markdown_content is null', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ markdown_content: null })} showPreview />
    );
    expect(container.querySelector('.document-preview')).not.toBeInTheDocument();
  });

  it('uses custom formatDate when provided', () => {
    const customFormat = jest.fn(() => 'CUSTOM-DATE');
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc()} formatDate={customFormat} />
    );
    expect(customFormat).toHaveBeenCalledWith('2025-04-23T12:00:00Z');
    expect(container.querySelector('.document-meta')).toHaveTextContent('CUSTOM-DATE');
  });

  it('handles tag objects with name property', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ tags: [{ name: 'typescript' }, { name: 'react' }] })} />
    );
    const tagEls = container.querySelectorAll('.document-tag');
    expect(tagEls[0]).toHaveTextContent('typescript');
    expect(tagEls[1]).toHaveTextContent('react');
  });

  it('renders seo score and readability when seoInfo provided', () => {
    const { container } = renderWithRouter(
      <DocumentCard document={mockDoc({ seoInfo: { seoScore: 87, readability: 'Easy' } })} />
    );
    const score = container.querySelector('.seo-score');
    const readability = container.querySelector('.seo-readability');
    expect(score).toBeInTheDocument();
    expect(score).toHaveTextContent('SEO 87');
    expect(readability).toBeInTheDocument();
    expect(readability).toHaveTextContent('Easy');
  });

  it('renders seo block alongside processing badge', () => {
    const { container } = renderWithRouter(
      <DocumentCard
        document={mockDoc({
          processing_status: 'completed',
          seoInfo: { seoScore: 42, readability: 'Hard' },
        })}
      />
    );
    expect(container.querySelector('.processing-badge--completed')).toHaveTextContent('Embedded');
    expect(container.querySelector('.seo-score')).toHaveTextContent('SEO 42');
    expect(container.querySelector('.seo-readability')).toHaveTextContent('Hard');
  });

  it('does not render seo block when seoInfo is undefined', () => {
    const { container } = renderWithRouter(<DocumentCard document={mockDoc()} />);
    expect(container.querySelector('.seo-score')).not.toBeInTheDocument();
    expect(container.querySelector('.seo-readability')).not.toBeInTheDocument();
  });
});
