import React from 'react';
import { render, screen, waitFor } from '../test-utils';

// Mock useParams and useNavigate before importing component
const mockNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useParams: () => ({ id: '1' }),
  useNavigate: () => mockNavigate,
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
  },
  documentService: {
    getDocument: jest.fn(),
    updateDocument: jest.fn(),
    deleteDocument: jest.fn(),
  },
}));

// Import after mocks are set up
import DocumentView from './DocumentView';
import { documentService } from '../services/api';

describe('DocumentView', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockNavigate.mockClear();
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
});
