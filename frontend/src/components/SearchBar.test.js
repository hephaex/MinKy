import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import SearchBar from './SearchBar';

describe('SearchBar', () => {
  const mockOnSearch = jest.fn();

  beforeEach(() => {
    mockOnSearch.mockClear();
  });

  it('renders with placeholder text', () => {
    render(<SearchBar onSearch={mockOnSearch} />);
    expect(screen.getByPlaceholderText('Search documents...')).toBeInTheDocument();
  });

  it('renders with initial value', () => {
    render(<SearchBar onSearch={mockOnSearch} initialValue="test query" />);
    expect(screen.getByDisplayValue('test query')).toBeInTheDocument();
  });

  it('calls onSearch with trimmed query on form submit', async () => {
    const user = userEvent.setup();
    render(<SearchBar onSearch={mockOnSearch} />);

    const input = screen.getByPlaceholderText('Search documents...');
    await user.type(input, '  hello world  ');
    await user.click(screen.getByRole('button', { name: /🔍/i }));

    expect(mockOnSearch).toHaveBeenCalledWith('hello world');
  });

  it('calls onSearch on Enter key press', async () => {
    const user = userEvent.setup();
    render(<SearchBar onSearch={mockOnSearch} />);

    const input = screen.getByPlaceholderText('Search documents...');
    await user.type(input, 'test{Enter}');

    expect(mockOnSearch).toHaveBeenCalledWith('test');
  });

  it('shows clear button when input has value', async () => {
    const user = userEvent.setup();
    render(<SearchBar onSearch={mockOnSearch} />);

    const input = screen.getByPlaceholderText('Search documents...');
    expect(screen.queryByLabelText('Clear search')).not.toBeInTheDocument();

    await user.type(input, 'test');
    expect(screen.getByLabelText('Clear search')).toBeInTheDocument();
  });

  it('clears input and calls onSearch with empty string on clear button click', async () => {
    const user = userEvent.setup();
    render(<SearchBar onSearch={mockOnSearch} initialValue="test" />);

    await user.click(screen.getByLabelText('Clear search'));

    expect(screen.getByPlaceholderText('Search documents...')).toHaveValue('');
    expect(mockOnSearch).toHaveBeenCalledWith('');
  });

  it('updates input value on typing', async () => {
    const user = userEvent.setup();
    render(<SearchBar onSearch={mockOnSearch} />);

    const input = screen.getByPlaceholderText('Search documents...');
    await user.type(input, 'new query');

    expect(input).toHaveValue('new query');
  });
});
