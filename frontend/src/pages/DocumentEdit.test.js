import React from 'react';
import { render, screen, fireEvent, waitFor } from '../test-utils';
import DocumentEdit from './DocumentEdit';
import { documentService } from '../services/api';

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
  },
}));

// Mock useParams and useNavigate
const mockNavigate = jest.fn();
jest.mock('react-router-dom', () => ({
  ...jest.requireActual('react-router-dom'),
  useParams: () => ({ id: '1' }),
  useNavigate: () => mockNavigate,
}));

// Mock CollaborativeEditor
jest.mock('../components/CollaborativeEditor', () => {
  return function MockCollaborativeEditor({ initialValue, onChange, placeholder }) {
    return (
      <textarea
        data-testid="collaborative-editor"
        value={initialValue}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
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
        <span data-testid="tags-display">{tags.join(', ')}</span>
      </div>
    );
  };
});

describe('DocumentEdit', () => {
  const mockDocument = {
    id: 1,
    title: 'Original Title',
    author: 'Original Author',
    markdown_content: 'Original content',
    category_id: null,
    tags: [{ name: 'tag1' }, { name: 'tag2' }],
  };

  beforeEach(() => {
    jest.clearAllMocks();
    documentService.getDocument.mockResolvedValue(mockDocument);
  });

  describe('loading state', () => {
    it('shows loading message initially', () => {
      documentService.getDocument.mockImplementation(() => new Promise(() => {}));
      render(<DocumentEdit />);
      expect(screen.getByText('Loading document...')).toBeInTheDocument();
    });
  });

  describe('document loaded', () => {
    it('renders page title', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Edit Document')).toBeInTheDocument();
      });
    });

    it('populates title field with document title', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const titleInput = screen.getByLabelText(/Title/);
        expect(titleInput).toHaveValue('Original Title');
      });
    });

    it('populates author field with document author', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const authorInput = screen.getByLabelText(/Author/);
        expect(authorInput).toHaveValue('Original Author');
      });
    });

    it('populates content with document markdown', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const editor = screen.getByTestId('collaborative-editor');
        expect(editor).toHaveValue('Original content');
      });
    });

    it('loads document tags', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const tagsDisplay = screen.getByTestId('tags-display');
        expect(tagsDisplay).toHaveTextContent('tag1, tag2');
      });
    });
  });

  describe('error state', () => {
    it('shows error message when fetch fails', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Failed to fetch document')).toBeInTheDocument();
      });
    });

    it('shows back button in error state', async () => {
      documentService.getDocument.mockRejectedValue(new Error('Network error'));

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Back to Documents')).toBeInTheDocument();
      });
    });
  });

  describe('action buttons', () => {
    it('renders Cancel button', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Cancel')).toBeInTheDocument();
      });
    });

    it('renders Save Changes button', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Save Changes')).toBeInTheDocument();
      });
    });
  });

  describe('form validation', () => {
    it('Save button is disabled when no changes made', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const saveButton = screen.getByText('Save Changes');
        expect(saveButton).toBeDisabled();
      });
    });

    it('Save button is enabled when changes are made', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Modified Title' } });

      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).not.toBeDisabled();
    });

    it('Save button is disabled when title is empty', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: '' } });

      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).toBeDisabled();
    });
  });

  describe('form submission', () => {
    it('updates document and navigates on success', async () => {
      documentService.updateDocument.mockResolvedValue({ id: 1 });

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Updated Title' } });

      const saveButton = screen.getByText('Save Changes');
      fireEvent.click(saveButton);

      await waitFor(() => {
        expect(documentService.updateDocument).toHaveBeenCalledWith(
          '1',
          expect.objectContaining({
            title: 'Updated Title',
          })
        );
      });

      expect(mockNavigate).toHaveBeenCalledWith('/documents/1');
    });

    it('shows error message on update failure', async () => {
      documentService.updateDocument.mockRejectedValue(new Error('Server error'));

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Updated Title' } });

      const saveButton = screen.getByText('Save Changes');
      fireEvent.click(saveButton);

      await waitFor(() => {
        expect(screen.getByText('Failed to update document')).toBeInTheDocument();
      });
    });

    it('shows saving state during submission', async () => {
      documentService.updateDocument.mockImplementation(() => new Promise(() => {}));

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Updated Title' } });

      const saveButton = screen.getByText('Save Changes');
      fireEvent.click(saveButton);

      await waitFor(() => {
        expect(screen.getByText('Saving...')).toBeInTheDocument();
      });
    });
  });

  describe('cancel action', () => {
    it('navigates to document view on cancel without changes', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByText('Cancel')).toBeInTheDocument();
      });

      const cancelButton = screen.getByText('Cancel');
      fireEvent.click(cancelButton);

      expect(mockNavigate).toHaveBeenCalledWith('/documents/1');
    });

    it('shows confirmation when cancelling with unsaved changes', async () => {
      const confirmSpy = jest.spyOn(window, 'confirm').mockReturnValue(false);

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Changed Title' } });

      const cancelButton = screen.getByText('Cancel');
      fireEvent.click(cancelButton);

      expect(confirmSpy).toHaveBeenCalledWith(
        'You have unsaved changes. Are you sure you want to leave?'
      );
      confirmSpy.mockRestore();
    });

    it('navigates when confirm is accepted', async () => {
      const confirmSpy = jest.spyOn(window, 'confirm').mockReturnValue(true);

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Changed Title' } });

      const cancelButton = screen.getByText('Cancel');
      fireEvent.click(cancelButton);

      expect(mockNavigate).toHaveBeenCalledWith('/documents/1');
      confirmSpy.mockRestore();
    });

    it('stays on page when confirm is rejected', async () => {
      const confirmSpy = jest.spyOn(window, 'confirm').mockReturnValue(false);

      render(<DocumentEdit />);

      await waitFor(() => {
        expect(screen.getByLabelText(/Title/)).toBeInTheDocument();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'Changed Title' } });

      mockNavigate.mockClear();
      const cancelButton = screen.getByText('Cancel');
      fireEvent.click(cancelButton);

      expect(mockNavigate).not.toHaveBeenCalled();
      confirmSpy.mockRestore();
    });
  });

  describe('change detection', () => {
    it('detects title changes', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const saveButton = screen.getByText('Save Changes');
        expect(saveButton).toBeDisabled();
      });

      const titleInput = screen.getByLabelText(/Title/);
      fireEvent.change(titleInput, { target: { value: 'New Title' } });

      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).not.toBeDisabled();
    });

    it('detects author changes', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const saveButton = screen.getByText('Save Changes');
        expect(saveButton).toBeDisabled();
      });

      const authorInput = screen.getByLabelText(/Author/);
      fireEvent.change(authorInput, { target: { value: 'New Author' } });

      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).not.toBeDisabled();
    });

    it('detects content changes', async () => {
      render(<DocumentEdit />);

      await waitFor(() => {
        const saveButton = screen.getByText('Save Changes');
        expect(saveButton).toBeDisabled();
      });

      const editor = screen.getByTestId('collaborative-editor');
      fireEvent.change(editor, { target: { value: 'New content' } });

      const saveButton = screen.getByText('Save Changes');
      expect(saveButton).not.toBeDisabled();
    });
  });

  describe('handles null author', () => {
    it('handles document with null author', async () => {
      documentService.getDocument.mockResolvedValue({
        ...mockDocument,
        author: null,
      });

      render(<DocumentEdit />);

      await waitFor(() => {
        const authorInput = screen.getByLabelText(/Author/);
        expect(authorInput).toHaveValue('');
      });
    });
  });
});
