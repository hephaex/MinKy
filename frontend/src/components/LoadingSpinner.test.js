import React from 'react';
import { render, screen } from '@testing-library/react';
import LoadingSpinner from './LoadingSpinner';

describe('LoadingSpinner', () => {
  it('renders with default props', () => {
    const { container } = render(<LoadingSpinner />);
    expect(container.querySelector('.loading-spinner')).toBeInTheDocument();
    expect(container.querySelector('.spinner')).toBeInTheDocument();
  });

  it('applies medium size class by default', () => {
    const { container } = render(<LoadingSpinner />);
    expect(container.querySelector('.spinner-medium')).toBeInTheDocument();
  });

  it('applies small size class when size is small', () => {
    const { container } = render(<LoadingSpinner size="small" />);
    expect(container.querySelector('.spinner-small')).toBeInTheDocument();
  });

  it('applies large size class when size is large', () => {
    const { container } = render(<LoadingSpinner size="large" />);
    expect(container.querySelector('.spinner-large')).toBeInTheDocument();
  });

  it('displays message when provided', () => {
    render(<LoadingSpinner message="Loading data..." />);
    expect(screen.getByText('Loading data...')).toBeInTheDocument();
  });

  it('does not display message element when not provided', () => {
    const { container } = render(<LoadingSpinner />);
    expect(container.querySelector('.loading-message')).not.toBeInTheDocument();
  });

  it('wraps in fullscreen container when fullScreen is true', () => {
    const { container } = render(<LoadingSpinner fullScreen />);
    expect(container.querySelector('.loading-fullscreen')).toBeInTheDocument();
  });

  it('does not wrap in fullscreen container by default', () => {
    const { container } = render(<LoadingSpinner />);
    expect(container.querySelector('.loading-fullscreen')).not.toBeInTheDocument();
  });

  it('renders with all props combined', () => {
    const { container } = render(
      <LoadingSpinner size="large" message="Please wait..." fullScreen />
    );
    expect(container.querySelector('.loading-fullscreen')).toBeInTheDocument();
    expect(container.querySelector('.spinner-large')).toBeInTheDocument();
    expect(screen.getByText('Please wait...')).toBeInTheDocument();
  });
});
