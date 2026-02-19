import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import FileUpload from './FileUpload';
import { documentService } from '../services/api';

jest.mock('../services/api');

describe('FileUpload', () => {
  const mockOnUploadSuccess = jest.fn();
  const mockOnUploadError = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    mockOnUploadSuccess.mockClear();
    mockOnUploadError.mockClear();
  });

  const getFileInput = (container) => container.querySelector('input[type="file"]');

  it('renders file upload component', () => {
    render(<FileUpload />);
    expect(screen.getByText(/Click to select/)).toBeInTheDocument();
  });

  it('renders upload hint text', () => {
    render(<FileUpload />);
    expect(screen.getByText(/\.md files only/)).toBeInTheDocument();
    expect(screen.getByText(/max 10MB/)).toBeInTheDocument();
  });

  it('accepts .md files in input', () => {
    const { container } = render(<FileUpload />);
    const input = getFileInput(container);
    expect(input).toHaveAttribute('accept', '.md');
  });

  it('allows multiple file selection', () => {
    const { container } = render(<FileUpload />);
    const input = getFileInput(container);
    expect(input).toHaveAttribute('multiple');
  });

  it('rejects non-markdown files with error', async () => {
    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} />
    );

    const input = getFileInput(container);
    const file = new File(['content'], 'test.txt', { type: 'text/plain' });

    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockOnUploadError).toHaveBeenCalledWith(
        expect.stringContaining('not a markdown file')
      );
    });
  });

  it('rejects files larger than 10MB', async () => {
    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} />
    );

    const input = getFileInput(container);
    // Create a mock file with size > 10MB
    const largeFile = new File(['x'], 'large.md');
    Object.defineProperty(largeFile, 'size', { value: 11 * 1024 * 1024 });

    fireEvent.change(input, { target: { files: [largeFile] } });

    await waitFor(() => {
      expect(mockOnUploadError).toHaveBeenCalledWith(
        expect.stringContaining('too large')
      );
    });
  });

  it('successfully uploads valid markdown file', async () => {
    documentService.uploadDocument.mockResolvedValueOnce({
      document: { id: 1, name: 'test.md' }
    });

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });

    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockOnUploadSuccess).toHaveBeenCalled();
    });
  });

  it('handles upload error gracefully', async () => {
    documentService.uploadDocument.mockRejectedValueOnce(
      new Error('Network error')
    );

    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} />
    );

    const input = getFileInput(container);
    const file = new File(['# Test'], 'test.md');

    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockOnUploadError).toHaveBeenCalled();
    });
  });

  it('handles multiple file uploads', async () => {
    documentService.uploadDocument.mockResolvedValue({
      document: { id: 1 }
    });

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const files = [
      new File(['# Test 1'], 'test1.md'),
      new File(['# Test 2'], 'test2.md')
    ];

    fireEvent.change(input, { target: { files } });

    await waitFor(() => {
      expect(mockOnUploadSuccess).toHaveBeenCalled();
    }, { timeout: 5000 });
  });

  it('filters out non-markdown files from multiple selection', async () => {
    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} />
    );

    const input = getFileInput(container);
    const files = [
      new File(['# Test'], 'test.md'),
      new File(['content'], 'test.txt')
    ];

    fireEvent.change(input, { target: { files } });

    await waitFor(() => {
      expect(mockOnUploadError).toHaveBeenCalledWith(
        expect.stringContaining('markdown files')
      );
    });
  });

  it('shows uploading state', async () => {
    documentService.uploadDocument.mockImplementationOnce(
      () => new Promise(resolve => setTimeout(() => resolve({ document: { id: 1 } }), 200))
    );

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const files = [
      new File(['# Test 1'], 'test1.md'),
      new File(['# Test 2'], 'test2.md')
    ];

    fireEvent.change(input, { target: { files } });

    expect(screen.getByText(/Uploading/)).toBeInTheDocument();
  });

  it('disables input while uploading', async () => {
    documentService.uploadDocument.mockImplementationOnce(
      () => new Promise(resolve => setTimeout(() => resolve({ document: { id: 1 } }), 200))
    );

    const { container } = render(<FileUpload />);

    const input = getFileInput(container);
    const files = [
      new File(['# Test 1'], 'test1.md'),
      new File(['# Test 2'], 'test2.md')
    ];

    fireEvent.change(input, { target: { files } });

    expect(input).toBeDisabled();
  });

  it('handles drag and drop', async () => {
    documentService.uploadDocument.mockResolvedValueOnce({
      document: { id: 1 }
    });

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const dropzone = container.querySelector('.file-upload-dropzone');
    const file = new File(['# Test'], 'test.md');
    const dataTransfer = {
      files: [file],
      items: [{ kind: 'file', type: 'text/markdown', getAsFile: () => file }],
      types: ['Files']
    };

    fireEvent.dragOver(dropzone, { dataTransfer });
    expect(container.querySelector('.file-upload-container')).toHaveClass('drag-over');

    fireEvent.drop(dropzone, { dataTransfer });

    await waitFor(() => {
      expect(mockOnUploadSuccess).toHaveBeenCalled();
    });
  });

  it('updates drag-over state on drag enter', () => {
    const { container } = render(<FileUpload />);
    const dropzone = container.querySelector('.file-upload-dropzone');

    fireEvent.dragOver(dropzone);

    expect(container.querySelector('.file-upload-container')).toHaveClass('drag-over');
  });

  it('removes drag-over state on drag leave', () => {
    const { container } = render(<FileUpload />);
    const dropzone = container.querySelector('.file-upload-dropzone');

    fireEvent.dragOver(dropzone);
    fireEvent.dragLeave(dropzone);

    expect(container.querySelector('.file-upload-container')).not.toHaveClass('drag-over');
  });

  it('handles API error response', async () => {
    documentService.uploadDocument.mockRejectedValueOnce({
      response: {
        status: 400,
        data: { error: 'Invalid file format' }
      }
    });

    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} />
    );

    const input = getFileInput(container);
    const file = new File(['# Test'], 'test.md');

    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockOnUploadError).toHaveBeenCalledWith(
        expect.stringContaining('Invalid file format')
      );
    });
  });

  it('shows progress for multiple file uploads', async () => {
    documentService.uploadDocument.mockImplementation(
      () => new Promise(resolve => setTimeout(() => resolve({ document: { id: 1 } }), 100))
    );

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const files = [
      new File(['# Test 1'], 'test1.md'),
      new File(['# Test 2'], 'test2.md')
    ];

    fireEvent.change(input, { target: { files } });

    await waitFor(() => {
      expect(screen.getByText(/Uploading.*files/)).toBeInTheDocument();
    });
  });

  it('handles upload response with 201 status', async () => {
    documentService.uploadDocument.mockRejectedValueOnce({
      response: {
        status: 201,
        data: { document: { id: 1 } }
      }
    });

    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const file = new File(['# Test'], 'test.md');

    fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockOnUploadSuccess).toHaveBeenCalled();
    });
  });

  it('does not handle null/empty files', async () => {
    const { container } = render(
      <FileUpload onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);

    fireEvent.change(input, { target: { files: [] } });

    expect(mockOnUploadSuccess).not.toHaveBeenCalled();
  });

  it('reports upload results for multiple files', async () => {
    documentService.uploadDocument
      .mockResolvedValueOnce({ document: { id: 1 } })
      .mockRejectedValueOnce(new Error('Upload failed'));

    const { container } = render(
      <FileUpload onUploadError={mockOnUploadError} onUploadSuccess={mockOnUploadSuccess} />
    );

    const input = getFileInput(container);
    const files = [
      new File(['# Test 1'], 'test1.md'),
      new File(['# Test 2'], 'test2.md')
    ];

    fireEvent.change(input, { target: { files } });

    // Wait for both uploads to complete
    await waitFor(() => {
      expect(mockOnUploadSuccess).toHaveBeenCalled();
    }, { timeout: 5000 });
  });
});
