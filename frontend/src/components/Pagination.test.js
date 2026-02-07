import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import Pagination from './Pagination';

describe('Pagination', () => {
  const mockOnPageChange = jest.fn();
  const basePagination = {
    pages: 10,
    has_prev: true,
    has_next: true,
    total: 100,
  };

  beforeEach(() => {
    mockOnPageChange.mockClear();
  });

  it('renders nothing when pages <= 1', () => {
    const { container } = render(
      <Pagination
        pagination={{ ...basePagination, pages: 1 }}
        currentPage={1}
        onPageChange={mockOnPageChange}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it('displays current page and total info', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    expect(screen.getByText(/Showing page 5 of 10/)).toBeInTheDocument();
    expect(screen.getByText(/100 total documents/)).toBeInTheDocument();
  });

  it('disables Previous button when has_prev is false', () => {
    render(
      <Pagination
        pagination={{ ...basePagination, has_prev: false }}
        currentPage={1}
        onPageChange={mockOnPageChange}
      />
    );
    expect(screen.getByText('Previous')).toBeDisabled();
  });

  it('disables Next button when has_next is false', () => {
    render(
      <Pagination
        pagination={{ ...basePagination, has_next: false }}
        currentPage={10}
        onPageChange={mockOnPageChange}
      />
    );
    expect(screen.getByText('Next')).toBeDisabled();
  });

  it('calls onPageChange with previous page when Previous is clicked', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    fireEvent.click(screen.getByText('Previous'));
    expect(mockOnPageChange).toHaveBeenCalledWith(4);
  });

  it('calls onPageChange with next page when Next is clicked', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    fireEvent.click(screen.getByText('Next'));
    expect(mockOnPageChange).toHaveBeenCalledWith(6);
  });

  it('calls onPageChange when a page number is clicked', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    fireEvent.click(screen.getByText('3'));
    expect(mockOnPageChange).toHaveBeenCalledWith(3);
  });

  it('marks current page button as active', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    const currentPageButton = screen.getByText('5');
    expect(currentPageButton).toHaveClass('active');
  });

  it('shows ellipsis for large page ranges', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    const dots = screen.getAllByText('...');
    expect(dots.length).toBeGreaterThan(0);
  });

  it('always shows first and last page', () => {
    render(
      <Pagination
        pagination={basePagination}
        currentPage={5}
        onPageChange={mockOnPageChange}
      />
    );
    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
  });
});
