import { render, screen, waitFor, fireEvent } from '../test-utils';

// Mock useParams, useNavigate and useLocation before importing component
const mockNavigate = jest.fn();
let mockLocation = { key: 'default', pathname: '/documents/1' };
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useParams: () => ({ id: '1' }),
  useNavigate: () => mockNavigate,
  useLocation: () => mockLocation,
}));

// Mock react-markdown to avoid ESM issues
jest.mock('react-markdown', () => {
  return function MockReactMarkdown({ children }) {
    return <div data-testid="markdown-content">{children}</div>;
  };
});

// Mock remark-gfm
jest.mock('remark-gfm', () => () => {});

// Mock syntax highlighter
jest.mock('react-syntax-highlighter', () => ({
  Prism: ({ children }) => <pre>{children}</pre>,
}));

jest.mock('react-syntax-highlighter/dist/esm/styles/prism', () => ({
  tomorrow: {},
}));

// Mock obsidianRenderer
jest.mock('../utils/obsidianRenderer', () => ({
  extractFrontmatter: jest.fn((content) => ({
    metadata: {},
    content: content,
  })),
  processInternalLinks: jest.fn((content) => content),
  processHashtags: jest.fn((content) => content),
}));

// Mock the api modules
jest.mock('../services/api', () => ({
  __esModule: true,
  default: {
    get: jest.fn(),
    post: jest.fn(),
    put: jest.fn(),
    delete: jest.fn(),
  },
  documentService: {
    getDocument: jest.fn(),
    updateDocument: jest.fn(),
    deleteDocument: jest.fn(),
  },
}));

// Import after mocks are set up
import DocumentView from './DocumentView';
import api, { documentService } from '../services/api';
import { extractFrontmatter } from '../utils/obsidianRenderer';

describe('DocumentView', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockNavigate.mockClear();
    mockLocation = { key: 'default', pathname: '/documents/1' };
  });

  describe('loading state', () => {
    it('shows loading message initially', () => {
      documentService.getDocument.mockImplementation(() => new Promise(() => {}));
      render(<DocumentView />);
      expect(screen.getByText('Loading document...')).toBeInTheDocument();
    });
  });

  describe('error state', () => {
    it('shows error message when fetch fails', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentView />);

      await waitFor(() => {
        expect(screen.getByText('Failed to fetch document')).toBeInTheDocument();
      });
    });

    it('shows back link in error state', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentView />);

      await waitFor(() => {
        expect(screen.getByText('Back to Documents')).toBeInTheDocument();
      });
    });

    it('back link has correct href in error state', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentView />);

      await waitFor(() => {
        const backLink = screen.getByText('Back to Documents');
        expect(backLink).toHaveAttribute('href', '/');
      });
    });
  });

  describe('api calls', () => {
    it('calls getDocument with document id', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Test'));

      render(<DocumentView />);

      await waitFor(() => {
        expect(documentService.getDocument).toHaveBeenCalledWith('1');
      });
    });
  });

  describe('back navigation', () => {
    it('goes back to the previous view when opened from within the app', async () => {
      // In-app navigation gives the location a non-default key.
      mockLocation = { key: 'abc123', pathname: '/documents/1' };
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentView />);

      const backLink = await screen.findByText('Back to Documents');
      fireEvent.click(backLink);

      expect(mockNavigate).toHaveBeenCalledWith(-1);
    });

    it('falls back to all documents when opened directly (no history)', async () => {
      // Direct load: React Router gives the initial entry the 'default' key.
      mockLocation = { key: 'default', pathname: '/documents/1' };
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentView />);

      const backLink = await screen.findByText('Back to Documents');
      // The link still points at '/', so a normal click navigates there.
      expect(backLink).toHaveAttribute('href', '/');
      fireEvent.click(backLink);

      expect(mockNavigate).not.toHaveBeenCalledWith(-1);
    });
  });

  describe('auto-tagging trigger', () => {
    beforeEach(() => {
      extractFrontmatter.mockReturnValue({ metadata: {}, content: '# hello' });
    });

    it('does not call suggest-tags when the document already has tag_names (Rust API shape)', async () => {
      // The Rust single-document API returns `tag_names` (plain string array),
      // not `tags` — a document with tag_names populated already has tags and
      // must not be treated as untagged.
      documentService.getDocument.mockResolvedValue({
        id: 1,
        title: 'Doc One',
        markdown_content: '# hello',
        tag_names: ['rust', 'axum'],
        processing_status: 'completed',
        author: null,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      });

      render(<DocumentView />);

      await screen.findByText('Doc One');
      expect(api.post).not.toHaveBeenCalledWith('/ai/suggest-tags', expect.anything());
    });

    it('calls suggest-tags when the document has no tags and no tag_names', async () => {
      documentService.getDocument.mockResolvedValue({
        id: 1,
        title: 'Doc One',
        markdown_content: '# hello',
        tag_names: [],
        processing_status: 'completed',
        author: null,
        created_at: '2024-01-01T00:00:00Z',
        updated_at: '2024-01-01T00:00:00Z',
      });
      api.post.mockResolvedValue({ data: { success: true, suggested_tags: [] } });

      render(<DocumentView />);

      await screen.findByText('Doc One');
      await waitFor(() => {
        expect(api.post).toHaveBeenCalledWith(
          '/ai/suggest-tags',
          expect.objectContaining({ title: 'Doc One' })
        );
      });
    });
  });

  describe('table of contents sidebar', () => {
    const loadedDoc = {
      id: 1,
      title: 'Doc One',
      markdown_content: '# hello',
      tags: [{ name: 'keep' }],
      processing_status: 'completed',
      author: null,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    };

    beforeEach(() => {
      extractFrontmatter.mockReturnValue({ metadata: {}, content: '# hello' });
      documentService.getDocument.mockResolvedValue(loadedDoc);
    });

    it('renders the TOC panel before the document content (left side)', async () => {
      const { container } = render(<DocumentView />);
      await screen.findByText('Doc One');

      const body = container.querySelector('.document-body');
      const children = Array.from(body.children);
      const tocIndex = children.findIndex((el) => el.classList.contains('document-toc'));
      const contentIndex = children.findIndex((el) => el.classList.contains('document-content'));
      expect(tocIndex).toBeGreaterThanOrEqual(0);
      expect(tocIndex).toBeLessThan(contentIndex);
    });

    it('hides the TOC panel when the toggle button is clicked', async () => {
      const { container } = render(<DocumentView />);
      await screen.findByText('Doc One');
      expect(container.querySelector('.document-toc')).toBeInTheDocument();

      fireEvent.click(screen.getByRole('button', { name: /목차 숨기기/ }));

      expect(container.querySelector('.document-toc')).not.toBeInTheDocument();
    });

    it('shows the TOC panel again when toggled back on', async () => {
      const { container } = render(<DocumentView />);
      await screen.findByText('Doc One');

      fireEvent.click(screen.getByRole('button', { name: /목차 숨기기/ }));
      fireEvent.click(screen.getByRole('button', { name: /목차 보기/ }));

      expect(container.querySelector('.document-toc')).toBeInTheDocument();
    });

    it('does not render the TOC panel while viewing raw markdown source', async () => {
      const { container } = render(<DocumentView />);
      await screen.findByText('Doc One');

      fireEvent.click(screen.getByText('Show Markdown'));

      expect(container.querySelector('.document-toc')).not.toBeInTheDocument();
    });
  });

  describe('delete navigation', () => {
    const loadedDoc = {
      id: 1,
      title: 'Doc One',
      markdown_content: '# hello',
      tags: [{ name: 'keep' }], // non-empty avoids the auto-tagging API path
      processing_status: 'completed',
      author: null,
      created_at: '2024-01-01T00:00:00Z',
      updated_at: '2024-01-01T00:00:00Z',
    };

    beforeEach(() => {
      // CRA's resetMocks wipes factory implementations, so re-establish the
      // frontmatter parser used while rendering the loaded document.
      extractFrontmatter.mockReturnValue({ metadata: {}, content: '# hello' });
    });

    it('returns to the previous list after deleting (in-app)', async () => {
      mockLocation = { key: 'abc123', pathname: '/documents/1' };
      window.confirm = jest.fn(() => true);
      documentService.getDocument.mockResolvedValue(loadedDoc);
      api.delete.mockResolvedValue({});

      render(<DocumentView />);

      const deleteBtn = await screen.findByText('Delete');
      fireEvent.click(deleteBtn);

      await waitFor(() => {
        expect(api.delete).toHaveBeenCalledWith('/documents/1');
      });
      expect(mockNavigate).toHaveBeenCalledWith(-1);
    });

    it('goes to all documents after deleting when opened directly', async () => {
      mockLocation = { key: 'default', pathname: '/documents/1' };
      window.confirm = jest.fn(() => true);
      documentService.getDocument.mockResolvedValue(loadedDoc);
      api.delete.mockResolvedValue({});

      render(<DocumentView />);

      const deleteBtn = await screen.findByText('Delete');
      fireEvent.click(deleteBtn);

      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith('/');
      });
    });
  });
});
