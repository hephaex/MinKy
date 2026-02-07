import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TagInput from './TagInput';

// Mock the tagService
jest.mock('../services/api', () => ({
  tagService: {
    suggestTags: jest.fn().mockResolvedValue({
      suggestions: [
        { name: 'react', slug: 'react' },
        { name: 'javascript', slug: 'javascript' },
      ],
    }),
  },
}));

describe('TagInput', () => {
  const mockOnChange = jest.fn();
  const defaultProps = {
    tags: [],
    onChange: mockOnChange,
  };

  beforeEach(() => {
    mockOnChange.mockClear();
  });

  it('renders input field', () => {
    render(<TagInput {...defaultProps} />);
    expect(screen.getByPlaceholderText('Add tags...')).toBeInTheDocument();
  });

  it('displays existing tags as chips', () => {
    render(<TagInput {...defaultProps} tags={['react', 'javascript']} />);
    expect(screen.getByText('react')).toBeInTheDocument();
    expect(screen.getByText('javascript')).toBeInTheDocument();
  });

  it('hides placeholder when tags exist', () => {
    render(<TagInput {...defaultProps} tags={['react']} />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('placeholder', '');
  });

  it('adds tag on Enter key', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} />);

    const input = screen.getByPlaceholderText('Add tags...');
    await user.type(input, 'newtag{Enter}');

    expect(mockOnChange).toHaveBeenCalledWith(['newtag']);
  });

  it('adds tag on comma key', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} />);

    const input = screen.getByPlaceholderText('Add tags...');
    await user.type(input, 'newtag,');

    expect(mockOnChange).toHaveBeenCalledWith(['newtag']);
  });

  it('removes tag when remove button is clicked', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} tags={['react', 'javascript']} />);

    const removeButton = screen.getByLabelText('Remove tag react');
    await user.click(removeButton);

    expect(mockOnChange).toHaveBeenCalledWith(['javascript']);
  });

  it('does not add duplicate tags (case insensitive)', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} tags={['React']} />);

    const input = screen.getByRole('textbox');
    await user.type(input, 'react{Enter}');

    // Should not call onChange for duplicate
    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it('trims whitespace from tags', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} />);

    const input = screen.getByPlaceholderText('Add tags...');
    await user.type(input, '  newtag  {Enter}');

    expect(mockOnChange).toHaveBeenCalledWith(['newtag']);
  });

  it('ignores empty tags', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} />);

    const input = screen.getByPlaceholderText('Add tags...');
    await user.type(input, '   {Enter}');

    expect(mockOnChange).not.toHaveBeenCalled();
  });

  it('removes last tag on Backspace when input is empty', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} tags={['react', 'javascript']} />);

    const input = screen.getByRole('textbox');
    await user.click(input);
    await user.keyboard('{Backspace}');

    expect(mockOnChange).toHaveBeenCalledWith(['react']);
  });

  it('clears input after adding tag', async () => {
    const user = userEvent.setup();
    render(<TagInput {...defaultProps} />);

    const input = screen.getByPlaceholderText('Add tags...');
    await user.type(input, 'newtag{Enter}');

    expect(input).toHaveValue('');
  });

  it('has correct container class', () => {
    const { container } = render(<TagInput {...defaultProps} />);
    expect(container.querySelector('.tag-input-container')).toBeInTheDocument();
  });

  it('displays AI suggestions when suggestedTags are provided', () => {
    render(
      <TagInput {...defaultProps} suggestedTags={['ai-tag-1', 'ai-tag-2']} />
    );
    expect(screen.getByText('ai-tag-1')).toBeInTheDocument();
    expect(screen.getByText('ai-tag-2')).toBeInTheDocument();
  });

  it('shows AI suggestions panel header', () => {
    render(
      <TagInput {...defaultProps} suggestedTags={['ai-tag']} />
    );
    expect(screen.getByText(/AI Tags Auto-Applied/)).toBeInTheDocument();
  });
});
