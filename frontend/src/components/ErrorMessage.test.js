import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import ErrorMessage from './ErrorMessage';

describe('ErrorMessage', () => {
  it('renders with string error', () => {
    render(<ErrorMessage error="Something went wrong" />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders with error object', () => {
    render(<ErrorMessage error={{ message: 'Network error' }} />);
    expect(screen.getByText('Network error')).toBeInTheDocument();
  });

  it('shows default error message when error has no message', () => {
    render(<ErrorMessage error={{}} />);
    expect(screen.getByText('알 수 없는 오류가 발생했습니다.')).toBeInTheDocument();
  });

  it('displays default title', () => {
    render(<ErrorMessage error="Error" />);
    expect(screen.getByText('오류가 발생했습니다')).toBeInTheDocument();
  });

  it('displays custom title when provided', () => {
    render(<ErrorMessage error="Error" title="Custom Title" />);
    expect(screen.getByText('Custom Title')).toBeInTheDocument();
  });

  it('renders retry button when onRetry is provided', () => {
    const mockRetry = jest.fn();
    render(<ErrorMessage error="Error" onRetry={mockRetry} />);
    expect(screen.getByText('다시 시도')).toBeInTheDocument();
  });

  it('calls onRetry when retry button is clicked', () => {
    const mockRetry = jest.fn();
    render(<ErrorMessage error="Error" onRetry={mockRetry} />);
    fireEvent.click(screen.getByText('다시 시도'));
    expect(mockRetry).toHaveBeenCalledTimes(1);
  });

  it('does not render retry button when onRetry is not provided', () => {
    render(<ErrorMessage error="Error" />);
    expect(screen.queryByText('다시 시도')).not.toBeInTheDocument();
  });

  it('shows error details when showDetails is true and error has stack', () => {
    const error = {
      message: 'Test error',
      stack: 'Error: Test error\n    at Component',
    };
    render(<ErrorMessage error={error} showDetails />);
    expect(screen.getByText('상세 정보')).toBeInTheDocument();
  });

  it('does not show error details when showDetails is false', () => {
    const error = {
      message: 'Test error',
      stack: 'Error: Test error\n    at Component',
    };
    render(<ErrorMessage error={error} showDetails={false} />);
    expect(screen.queryByText('상세 정보')).not.toBeInTheDocument();
  });

  it('renders error icon', () => {
    const { container } = render(<ErrorMessage error="Error" />);
    expect(container.querySelector('.error-icon')).toBeInTheDocument();
    expect(container.querySelector('svg')).toBeInTheDocument();
  });
});
