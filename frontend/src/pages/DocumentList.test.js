import React from 'react';
import { render, screen, fireEvent, waitFor } from '../test-utils';
import DocumentList from './DocumentList';
import api from '../services/api';

// Mock the api module
jest.mock('../services/api', () => ({
  __esModule: true,
  default: {
    get: jest.fn(),
    post: jest.fn(),
  },
  documentService: {
    getDocuments: jest.fn(),
  },
}));

// Mock useNavigate
const mockNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useNavigate: () => mockNavigate,
}));

describe('DocumentList', () => {
  const mockDocuments = [
    {
      id: 1,
      title: 'Test Document 1',
      author: 'Author 1',
      created_at: '2024-01-15T10:00:00Z',
      updated_at: '2024-01-15T10:00:00Z',
      tags: [{ name: 'react' }],
    },
    {
      id: 2,
      title: 'Test Document 2',
      author: 'Author 2',
      created_at: '2024-01-16T10:00:00Z',
      updated_at: '2024-01-16T12:00:00Z',
      tags: [],
    },
  ];

  const mockPagination = {
    total: 2,
    page: 1,
    per_page: 10,
    total_pages: 1,
  };

  const mockCategories = [
    { id: 1, path: 'Category A' },
    { id: 2, path: 'Category B' },
  ];

  beforeEach(() => {
    jest.clearAllMocks();
    api.get.mockImplementation((url) => {
      if (url.includes('/categories')) {
        return Promise.resolve({ data: { data: { categories: mockCategories } } });
      }
      if (url.includes('/documents')) {
        return Promise.resolve({
          data: {
            documents: mockDocuments,
            pagination: mockPagination,
          },
        });
      }
      return Promise.reject(new Error('Unknown URL'));
    });
  });

  describe('loading state', () => {
    it('shows loading message initially', () => {
      render(<DocumentList />);
      expect(screen.getByText('Loading documents...')).toBeInTheDocument();
    });
  });

  describe('document display', () => {
    it('renders documents after loading', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Test Document 1')).toBeInTheDocument();
      });
      expect(screen.getByText('Test Document 2')).toBeInTheDocument();
    });

    it('renders page title', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByRole('heading', { name: 'Documents' })).toBeInTheDocument();
      });
    });

    it('renders New Document button', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('+ New Document')).toBeInTheDocument();
      });
    });

    it('renders Upload button', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText(/Upload/)).toBeInTheDocument();
      });
    });

    it('renders Import button', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText(/Import/)).toBeInTheDocument();
      });
    });
  });

  describe('error state', () => {
    it('shows error message when fetch fails', async () => {
      api.get.mockRejectedValue(new Error('Network error'));

      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Failed to fetch documents')).toBeInTheDocument();
      });
    });
  });

  describe('empty state', () => {
    it('shows empty message when no documents', async () => {
      api.get.mockImplementation((url) => {
        if (url.includes('/categories')) {
          return Promise.resolve({ data: { data: { categories: [] } } });
        }
        return Promise.resolve({
          data: {
            documents: [],
            pagination: { total: 0, page: 1, per_page: 10, total_pages: 0 },
          },
        });
      });

      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('No documents yet')).toBeInTheDocument();
      });
      expect(screen.getByText('Get started by creating your first document')).toBeInTheDocument();
    });

    it('shows no results message when search returns empty', async () => {
      api.get.mockImplementation((url) => {
        if (url.includes('/categories')) {
          return Promise.resolve({ data: { data: { categories: [] } } });
        }
        if (url.includes('search=test')) {
          return Promise.resolve({
            data: {
              documents: [],
              pagination: { total: 0, page: 1, per_page: 10, total_pages: 0 },
            },
          });
        }
        return Promise.resolve({
          data: {
            documents: mockDocuments,
            pagination: mockPagination,
          },
        });
      });

      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Test Document 1')).toBeInTheDocument();
      });

      // Perform a search
      const searchInput = screen.getByRole('textbox');
      fireEvent.change(searchInput, { target: { value: 'test' } });
      fireEvent.submit(searchInput.closest('form') || searchInput);

      await waitFor(() => {
        expect(screen.getByText('No documents found')).toBeInTheDocument();
      });
    });
  });

  describe('search functionality', () => {
    it('renders search bar', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByRole('textbox')).toBeInTheDocument();
      });
    });

    it('triggers search when form submitted', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Test Document 1')).toBeInTheDocument();
      });

      const searchInput = screen.getByRole('textbox');
      fireEvent.change(searchInput, { target: { value: 'search query' } });
      fireEvent.submit(searchInput.closest('form') || searchInput);

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith(
          expect.stringContaining('search=search')
        );
      });
    });
  });

  describe('category filter', () => {
    it('renders category dropdown', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByRole('combobox')).toBeInTheDocument();
      });
      expect(screen.getByText('All Categories')).toBeInTheDocument();
    });

    it('shows categories in dropdown', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Category A')).toBeInTheDocument();
      });
      expect(screen.getByText('Category B')).toBeInTheDocument();
    });

    it('filters by category when selected', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByRole('combobox')).toBeInTheDocument();
      });

      const categorySelect = screen.getByRole('combobox');
      fireEvent.change(categorySelect, { target: { value: '1' } });

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith(
          expect.stringContaining('category_id=1')
        );
      });
    });
  });

  describe('upload section', () => {
    it('shows upload section when upload button clicked', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText(/Upload/)).toBeInTheDocument();
      });

      const uploadButton = screen.getByText(/Upload/);
      fireEvent.click(uploadButton);

      await waitFor(() => {
        expect(screen.getByText('Upload Markdown File')).toBeInTheDocument();
      });
    });

    it('hides upload section when close button clicked', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText(/Upload/)).toBeInTheDocument();
      });

      fireEvent.click(screen.getByText(/Upload/));

      await waitFor(() => {
        expect(screen.getByText('Upload Markdown File')).toBeInTheDocument();
      });

      // Find close button in upload section
      const closeButtons = screen.getAllByText('×');
      fireEvent.click(closeButtons[0]);

      await waitFor(() => {
        expect(screen.queryByText('Upload Markdown File')).not.toBeInTheDocument();
      });
    });
  });

  describe('pagination', () => {
    it('renders pagination component', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        expect(screen.getByText('Test Document 1')).toBeInTheDocument();
      });

      // Pagination should be rendered when documents exist
      expect(document.querySelector('.pagination')).toBeInTheDocument();
    });
  });

  describe('navigation links', () => {
    it('New Document link points to /documents/new', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        const newDocLink = screen.getByText('+ New Document');
        expect(newDocLink.closest('a')).toHaveAttribute('href', '/documents/new');
      });
    });

    it('Import link points to /import', async () => {
      render(<DocumentList />);

      await waitFor(() => {
        const importLink = screen.getByText(/Import/);
        expect(importLink.closest('a')).toHaveAttribute('href', '/import');
      });
    });
  });
});
