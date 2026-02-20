import React from 'react';
import { render, screen } from '@testing-library/react';
import AdminOverview from './AdminOverview';

describe('AdminOverview', () => {
  const mockSystemStats = {
    users: {
      total: 50,
      active: 42,
      admins: 3,
      new_this_week: 5
    },
    content: {
      documents: 250,
      public_documents: 100,
      tags: 45,
      comments: 320,
      new_documents_week: 12,
      new_comments_week: 35
    },
    storage: {
      estimated_kb: 512000,
      avg_document_size: 2048
    }
  };

  it('returns null when systemStats is not provided', () => {
    const { container } = render(<AdminOverview systemStats={null} />);
    expect(container.firstChild).toBeNull();
  });

  it('returns null when systemStats is undefined', () => {
    const { container } = render(<AdminOverview />);
    expect(container.firstChild).toBeNull();
  });

  it('renders system overview title', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('System Overview')).toBeInTheDocument();
  });

  it('renders user statistics', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Users')).toBeInTheDocument();
    expect(screen.getByText('50')).toBeInTheDocument();
  });

  it('displays active users and admin count', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('42 active, 3 admins')).toBeInTheDocument();
  });

  it('renders document statistics', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Documents')).toBeInTheDocument();
    expect(screen.getByText('100 public')).toBeInTheDocument();
  });

  it('renders tag statistics', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Tags')).toBeInTheDocument();
    expect(screen.getByText('Unique tags')).toBeInTheDocument();
  });

  it('renders comment statistics', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Comments')).toBeInTheDocument();
    expect(screen.getByText('Total comments')).toBeInTheDocument();
  });

  it('displays recent activity section', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Recent Activity (This Week)')).toBeInTheDocument();
  });

  it('shows new users this week', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('New Users')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
  });

  it('shows new documents this week', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('New Documents')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
  });

  it('shows new comments this week', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('New Comments')).toBeInTheDocument();
    expect(screen.getByText('35')).toBeInTheDocument();
  });

  it('displays storage information section', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Storage Information')).toBeInTheDocument();
  });

  it('calculates and displays storage in MB', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Estimated Storage')).toBeInTheDocument();
    // 512000 KB / 1024 = 500 MB
    expect(screen.getByText('500.00 MB')).toBeInTheDocument();
  });

  it('calculates and displays average document size in KB', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('Average Document Size')).toBeInTheDocument();
    // 2048 bytes / 1024 = 2 KB
    expect(screen.getByText('2.00 KB')).toBeInTheDocument();
  });

  it('renders stat cards with correct classes', () => {
    const { container } = render(<AdminOverview systemStats={mockSystemStats} />);
    expect(container.querySelector('.users-stat')).toBeInTheDocument();
    expect(container.querySelector('.documents-stat')).toBeInTheDocument();
    expect(container.querySelector('.tags-stat')).toBeInTheDocument();
    expect(container.querySelector('.comments-stat')).toBeInTheDocument();
  });

  it('renders overview tab container', () => {
    const { container } = render(<AdminOverview systemStats={mockSystemStats} />);
    expect(container.querySelector('.overview-tab')).toBeInTheDocument();
  });

  it('renders stats grid', () => {
    const { container } = render(<AdminOverview systemStats={mockSystemStats} />);
    expect(container.querySelector('.stats-grid')).toBeInTheDocument();
  });

  it('formats large numbers with locale string', () => {
    const largeStats = {
      ...mockSystemStats,
      users: { ...mockSystemStats.users, total: 1000000 }
    };
    render(<AdminOverview systemStats={largeStats} />);
    expect(screen.getByText(/1,000,000/)).toBeInTheDocument();
  });

  it('handles zero values', () => {
    const zeroStats = {
      users: { total: 0, active: 0, admins: 0, new_this_week: 0 },
      content: { documents: 0, public_documents: 0, tags: 0, comments: 0, new_documents_week: 0, new_comments_week: 0 },
      storage: { estimated_kb: 0, avg_document_size: 0 }
    };
    render(<AdminOverview systemStats={zeroStats} />);
    expect(screen.getByText('0 active, 0 admins')).toBeInTheDocument();
  });

  it('displays stat values correctly', () => {
    render(<AdminOverview systemStats={mockSystemStats} />);
    expect(screen.getByText('250')).toBeInTheDocument();
    expect(screen.getByText('45')).toBeInTheDocument();
    expect(screen.getByText('320')).toBeInTheDocument();
  });

  it('renders activity stats section', () => {
    const { container } = render(<AdminOverview systemStats={mockSystemStats} />);
    expect(container.querySelector('.activity-stats')).toBeInTheDocument();
  });

  it('renders storage stats section', () => {
    const { container } = render(<AdminOverview systemStats={mockSystemStats} />);
    expect(container.querySelector('.storage-stats')).toBeInTheDocument();
  });
});
