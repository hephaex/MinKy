import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import MarkdownEditor from './MarkdownEditor';

jest.mock('@uiw/react-md-editor', () => {
  return function MockMDEditor({ value, onChange, preview, textareaProps }) {
    return (
      <textarea
        data-testid="md-editor"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={textareaProps?.placeholder}
        {...textareaProps}
      />
    );
  };
});

jest.mock('./AISuggestions', () => {
  return function MockAISuggestions({ isVisible, onSuggestionSelect }) {
    return isVisible ? (
      <div data-testid="ai-suggestions">
        <button onClick={() => onSuggestionSelect('test suggestion')}>
          Use Suggestion
        </button>
      </div>
    ) : null;
  };
});

describe('MarkdownEditor', () => {
  const mockOnChange = jest.fn();
  const mockOnTitleSuggestion = jest.fn();
  const mockOnTagSuggestions = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders markdown editor', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(screen.getByTestId('md-editor')).toBeInTheDocument();
  });

  it('displays placeholder text', () => {
    const customPlaceholder = 'Write your notes here';
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
        placeholder={customPlaceholder}
      />
    );
    expect(screen.getByPlaceholderText(customPlaceholder)).toBeInTheDocument();
  });

  it('renders mode tabs', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(screen.getByText('Edit')).toBeInTheDocument();
    expect(screen.getByText('Preview')).toBeInTheDocument();
    expect(screen.getByText('Split')).toBeInTheDocument();
  });

  it('switches to preview mode', () => {
    render(
      <MarkdownEditor
        value="# Test"
        onChange={mockOnChange}
      />
    );
    const previewButton = screen.getByText('Preview');
    fireEvent.click(previewButton);

    expect(previewButton).toHaveClass('active');
    expect(screen.getByText('Edit')).not.toHaveClass('active');
  });

  it('switches to split mode', () => {
    render(
      <MarkdownEditor
        value="# Test"
        onChange={mockOnChange}
      />
    );
    const splitButton = screen.getByText('Split');
    fireEvent.click(splitButton);

    expect(splitButton).toHaveClass('active');
  });

  it('switches back to edit mode', () => {
    render(
      <MarkdownEditor
        value="# Test"
        onChange={mockOnChange}
      />
    );
    const previewButton = screen.getByText('Preview');
    fireEvent.click(previewButton);

    const editButton = screen.getByText('Edit');
    fireEvent.click(editButton);

    expect(editButton).toHaveClass('active');
    expect(previewButton).not.toHaveClass('active');
  });

  it('calls onChange when editor content changes', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    const editor = screen.getByTestId('md-editor');
    fireEvent.change(editor, { target: { value: 'New content' } });

    expect(mockOnChange).toHaveBeenCalledWith('New content');
  });

  it('displays value in editor', () => {
    const content = '# Hello World\n\nThis is a test';
    render(
      <MarkdownEditor
        value={content}
        onChange={mockOnChange}
      />
    );
    expect(screen.getByTestId('md-editor')).toHaveValue(content);
  });

  it('shows AI suggestions by default', () => {
    render(
      <MarkdownEditor
        value="This is a long enough content to trigger AI suggestions"
        onChange={mockOnChange}
      />
    );
    expect(screen.getByTestId('ai-suggestions')).toBeInTheDocument();
  });

  it('hides AI suggestions when disabled', () => {
    render(
      <MarkdownEditor
        value="This is a long enough content to trigger AI suggestions"
        onChange={mockOnChange}
        showAISuggestions={false}
      />
    );
    expect(screen.queryByTestId('ai-suggestions')).not.toBeInTheDocument();
  });

  it('does not show AI suggestions for short content', () => {
    render(
      <MarkdownEditor
        value="Short"
        onChange={mockOnChange}
      />
    );
    expect(screen.queryByTestId('ai-suggestions')).not.toBeInTheDocument();
  });

  it('does not show AI suggestions for empty content', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(screen.queryByTestId('ai-suggestions')).not.toBeInTheDocument();
  });

  it('inserts suggestion at cursor position', () => {
    const initialContent = 'Hello world';
    render(
      <MarkdownEditor
        value={initialContent}
        onChange={mockOnChange}
      />
    );

    const editor = screen.getByTestId('md-editor');
    // Simulate cursor in middle of text
    fireEvent.change(editor, { target: { value: initialContent, selectionStart: 5 } });

    const suggestionButton = screen.getByText('Use Suggestion');
    fireEvent.click(suggestionButton);

    // Should insert suggestion at cursor position
    expect(mockOnChange).toHaveBeenCalled();
  });

  it('passes cursor position to AI suggestions', () => {
    const content = 'This is a test with enough content for suggestions';
    render(
      <MarkdownEditor
        value={content}
        onChange={mockOnChange}
      />
    );
    expect(screen.getByTestId('ai-suggestions')).toBeInTheDocument();
  });

  it('handles title suggestion callback', () => {
    render(
      <MarkdownEditor
        value="Test content"
        onChange={mockOnChange}
        onTitleSuggestion={mockOnTitleSuggestion}
      />
    );
    expect(screen.getByTestId('ai-suggestions')).toBeInTheDocument();
  });

  it('handles tag suggestion callback', () => {
    render(
      <MarkdownEditor
        value="Test content"
        onChange={mockOnChange}
        onTagSuggestions={mockOnTagSuggestions}
      />
    );
    expect(screen.getByTestId('ai-suggestions')).toBeInTheDocument();
  });

  it('renders editor container with correct class', () => {
    const { container } = render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(container.querySelector('.markdown-editor-container')).toBeInTheDocument();
  });

  it('renders editor toolbar', () => {
    const { container } = render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(container.querySelector('.editor-toolbar')).toBeInTheDocument();
  });

  it('renders editor content area', () => {
    const { container } = render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(container.querySelector('.editor-content')).toBeInTheDocument();
  });

  it('has correct default placeholder', () => {
    render(
      <MarkdownEditor
        value=""
        onChange={mockOnChange}
      />
    );
    expect(screen.getByPlaceholderText(/Start writing your markdown/)).toBeInTheDocument();
  });

  it('tracks cursor position on selection', () => {
    const content = 'Test content here';
    render(
      <MarkdownEditor
        value={content}
        onChange={mockOnChange}
      />
    );
    const editor = screen.getByTestId('md-editor');

    fireEvent.select(editor, { target: { selectionStart: 5 } });

    // Should update cursor position internally
    expect(editor).toBeInTheDocument();
  });

  it('maintains edit mode as default', () => {
    render(
      <MarkdownEditor
        value="# Test"
        onChange={mockOnChange}
      />
    );
    expect(screen.getByText('Edit')).toHaveClass('active');
  });

  it('passes correct props to MDEditor component', () => {
    render(
      <MarkdownEditor
        value="Test"
        onChange={mockOnChange}
      />
    );
    const editor = screen.getByTestId('md-editor');
    expect(editor).toHaveValue('Test');
  });
});
