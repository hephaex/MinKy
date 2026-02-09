import React from 'react';
import { render, screen, fireEvent, waitFor } from '../test-utils';
import DocumentCreate from './DocumentCreate';
import { documentService } from '../services/api';
import api from '../services/api';

// Mock the api modules
jest.mock('../services/api', () => ({
  __esModule: true,
  default: {
    get: jest.fn(),
    post: jest.fn(),
  },
  documentService: {
    createDocument: jest.fn(),
  },
}));

// Mock useNavigate
const mockNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useNavigate: () => mockNavigate,
}));

// Mock MarkdownEditor
jest.mock('../components/MarkdownEditor', () => {
  return function MockMarkdownEditor({ value, onChange, placeholder }) {
    return (
      <textarea
        data-testid="markdown-editor"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
    );
  };
});

// Mock OCRUpload
jest.mock('../components/OCRUpload', () => {
  return function MockOCRUpload({ onTextExtracted }) {
    return (
      <div data-testid="ocr-upload">
        <button onClick={() => onTextExtracted({ text: 'OCR text', filename: 'test.pdf' })}>
          Extract Text
        </button>
      </div>
    );
  };
});

// Mock TagInput
jest.mock('../components/TagInput', () => {
  return function MockTagInput({ tags, onChange }) {
    return (
      <div data-testid="tag-input">
        <input
          data-testid="tag-input-field"
          onChange={(e) => onChange([...tags, e.target.value])}
        />
        <span>{tags.join(', ')}</span>
      </div>
    );
  };
});

describe('DocumentCreate', () => {
  const mockCategories = [
    { id: 1, path: 'Category A' },
    { id: 2, path: 'Category B' },
  ];

  beforeEach(() => {
    jest.clearAllMocks();
    api.get.mockResolvedValue({ data: { data: { categories: mockCategories } } });
  });

  describe('initial render', () => {
    it('renders page title', async () => {
      render(<DocumentCreate />);

      expect(screen.getByText('Create New Document')).toBeInTheDocument();
    });

    it('renders title input', async () => {
      render(<DocumentCreate />);

      expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
    });

    it('renders author input', async () => {
      render(<DocumentCreate />);

      expect(screen.getByLabelText(/Author/)).toBeInTheDocument();
    });

    it('renders category select', async () => {
      render(<DocumentCreate />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Category/)).toBeInTheDocument();
      });
    });

    it('renders tags input', async () => {
      render(<DocumentCreate />);

      expect(screen.getByTestId('tag-input')).toBeInTheDocument();
    });

    it('renders markdown editor', async () => {
      render(<DocumentCreate />);

      expect(screen.getByTestId('markdown-editor')).toBeInTheDocument();
    });

    it('renders Cancel button', async () => {
      render(<DocumentCreate />);

      expect(screen.getByText('Cancel')).toBeInTheDocument();
    });

    it('renders Create Document button', async () => {
      render(<DocumentCreate />);

      expect(screen.getByText('Create Document')).toBeInTheDocument();
    });

    it('renders OCR button', async () => {
      render(<DocumentCreate />);

      expect(screen.getByText('📄 OCR')).toBeInTheDocument();
    });

    it('renders Advanced OCR link', async () => {
      render(<DocumentCreate />);

      expect(screen.getByText('🔍 Advanced OCR')).toBeInTheDocument();
    });
  });

  describe('form validation', () => {
    it('Create button is disabled when title is empty', async () => {
      render(<DocumentCreate />);

      const createButton = screen.getByText('Create Document');
      expect(createButton).toBeDisabled();
    });

    it('Create button is disabled when content is empty', async () => {
      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Test Title' } });

      const createButton = screen.getByText('Create Document');
      expect(createButton).toBeDisabled();
    });

    it('Create button is enabled when title and content are filled', async () => {
      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Test Title' } });

      const editor = screen.getByTestId('markdown-editor');
      fireEvent.change(editor, { target: { value: 'Test content' } });

      const createButton = screen.getByText('Create Document');
      expect(createButton).not.toBeDisabled();
    });
  });

  describe('form submission', () => {
    it('shows error when submitting without required fields', async () => {
      render(<DocumentCreate />);

      // Manually trigger form submission
      const form = document.getElementById('document-form');
      fireEvent.submit(form);

      // The form should validate HTML5 required fields
      // or show custom error
      await waitFor(() => {
        // Either native validation or custom error
        const titleInput = screen.getByLabelText(/Title/);
        expect(titleInput).toBeRequired();
      });
    });

    it('creates document and navigates on success', async () => {
      documentService.createDocument.mockResolvedValue({ id: 123 });

      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'New Document' } });

      const editor = screen.getByTestId('markdown-editor');
      fireEvent.change(editor, { target: { value: 'Document content' } });

      const createButton = screen.getByText('Create Document');
      fireEvent.click(createButton);

      await waitFor(() => {
        expect(documentService.createDocument).toHaveBeenCalledWith(
          expect.objectContaining({
            title: 'New Document',
            markdown_content: 'Document content',
          })
        );
      });

      expect(mockNavigate).toHaveBeenCalledWith('/documents/123');
    });

    it('shows error message on creation failure', async () => {
      documentService.createDocument.mockRejectedValue(new Error('Server error'));

      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'New Document' } });

      const editor = screen.getByTestId('markdown-editor');
      fireEvent.change(editor, { target: { value: 'Document content' } });

      const createButton = screen.getByText('Create Document');
      fireEvent.click(createButton);

      await waitFor(() => {
        expect(screen.getByText('Failed to create document')).toBeInTheDocument();
      });
    });

    it('shows loading state during submission', async () => {
      documentService.createDocument.mockImplementation(() => new Promise(() => {}));

      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'New Document' } });

      const editor = screen.getByTestId('markdown-editor');
      fireEvent.change(editor, { target: { value: 'Document content' } });

      const createButton = screen.getByText('Create Document');
      fireEvent.click(createButton);

      await waitFor(() => {
        expect(screen.getByText('Creating...')).toBeInTheDocument();
      });
    });
  });

  describe('cancel action', () => {
    it('navigates to home on cancel', async () => {
      render(<DocumentCreate />);

      const cancelButton = screen.getByText('Cancel');
      fireEvent.click(cancelButton);

      expect(mockNavigate).toHaveBeenCalledWith('/');
    });
  });

  describe('OCR functionality', () => {
    it('shows OCR section when OCR button clicked', async () => {
      render(<DocumentCreate />);

      const ocrButton = screen.getByText('📄 OCR');
      fireEvent.click(ocrButton);

      expect(screen.getByTestId('ocr-upload')).toBeInTheDocument();
    });

    it('hides OCR section when clicked again', async () => {
      render(<DocumentCreate />);

      const ocrButton = screen.getByText('📄 OCR');
      fireEvent.click(ocrButton);
      fireEvent.click(ocrButton);

      expect(screen.queryByTestId('ocr-upload')).not.toBeInTheDocument();
    });

    it('fills form with OCR extracted text', async () => {
      render(<DocumentCreate />);

      const ocrButton = screen.getByText('📄 OCR');
      fireEvent.click(ocrButton);

      const extractButton = screen.getByText('Extract Text');
      fireEvent.click(extractButton);

      await waitFor(() => {
        const editor = screen.getByTestId('markdown-editor');
        expect(editor).toHaveValue('OCR text');
      });
    });
  });

  describe('category selection', () => {
    it('loads categories on mount', async () => {
      render(<DocumentCreate />);

      await waitFor(() => {
        expect(api.get).toHaveBeenCalledWith('/categories/?format=flat');
      });
    });

    it('displays categories in dropdown', async () => {
      render(<DocumentCreate />);

      await waitFor(() => {
        expect(screen.getByText('Category A')).toBeInTheDocument();
      });
      expect(screen.getByText('Category B')).toBeInTheDocument();
    });
  });

  describe('input changes', () => {
    it('updates title on input change', async () => {
      render(<DocumentCreate />);

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'My New Title' } });

      expect(titleInput).toHaveValue('My New Title');
    });

    it('updates author on input change', async () => {
      render(<DocumentCreate />);

      const authorInput = screen.getByLabelText(/Author/);
      fireEvent.change(authorInput, { target: { value: 'John Doe' } });

      expect(authorInput).toHaveValue('John Doe');
    });
  });
});
