import React from 'react';
import { render, screen } from '../test-utils';
import Header from './Header';

describe('Header', () => {
  it('renders the app name', () => {
    render(<Header />);
    expect(screen.getByText('Minky')).toBeInTheDocument();
  });

  it('renders the app description', () => {
    render(<Header />);
    expect(screen.getByText('Markdown Document Manager')).toBeInTheDocument();
  });

  it('renders navigation links', () => {
    render(<Header />);
    expect(screen.getByText('Documents')).toBeInTheDocument();
    expect(screen.getByText('Explore')).toBeInTheDocument();
    expect(screen.getByText('Config')).toBeInTheDocument();
  });

  it('logo links to home page', () => {
    render(<Header />);
    const logoLink = screen.getByRole('link', { name: /Minky/i });
    expect(logoLink).toHaveAttribute('href', '/');
  });

  it('documents link points to correct route', () => {
    render(<Header />);
    const link = screen.getByText('Documents');
    expect(link.closest('a')).toHaveAttribute('href', '/');
  });

  it('explore link points to analytics route', () => {
    render(<Header />);
    const link = screen.getByText('Explore');
    expect(link.closest('a')).toHaveAttribute('href', '/analytics');
  });

  it('config link points to settings route', () => {
    render(<Header />);
    const link = screen.getByText('Config');
    expect(link.closest('a')).toHaveAttribute('href', '/settings');
  });

  it('has correct CSS class', () => {
    const { container } = render(<Header />);
    expect(container.querySelector('.header')).toBeInTheDocument();
    expect(container.querySelector('.header-content')).toBeInTheDocument();
    expect(container.querySelector('.main-nav')).toBeInTheDocument();
  });
});
