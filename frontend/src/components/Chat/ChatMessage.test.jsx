import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import ChatMessage from './ChatMessage';

// react-markdown and remark-gfm are ESM-only; mock them for Jest (CJS environment)
jest.mock('react-markdown', () => {
  const MockReactMarkdown = ({ children }) => <div>{children}</div>;
  return MockReactMarkdown;
});

jest.mock('remark-gfm', () => () => {});

jest.mock('react-syntax-highlighter', () => ({
  Prism: ({ children }) => <pre>{children}</pre>,
}));

jest.mock('react-syntax-highlighter/dist/esm/styles/prism', () => ({
  oneDark: {},
}));

const userMsg = {
  id: 'u1',
  role: 'user',
  content: 'Hello world',
  timestamp: '2026-02-19T10:00:00Z',
};

const aiMsg = {
  id: 'a1',
  role: 'assistant',
  content: 'Response text',
  timestamp: '2026-02-19T10:00:05Z',
  sources: [{ title: 'Doc A', url: 'https://example.com' }],
};

describe('ChatMessage', () => {
  it('renders user message text', () => {
    render(<ChatMessage message={userMsg} />);
    expect(screen.getByText('Hello world')).toBeInTheDocument();
  });

  it('applies user class for user role', () => {
    const { container } = render(<ChatMessage message={userMsg} />);
    expect(container.firstChild).toHaveClass('chat-message--user');
  });

  it('applies ai class for assistant role', () => {
    const { container } = render(<ChatMessage message={aiMsg} />);
    expect(container.firstChild).toHaveClass('chat-message--ai');
  });

  it('renders assistant message content', () => {
    render(<ChatMessage message={aiMsg} />);
    expect(screen.getByText('Response text')).toBeInTheDocument();
  });

  it('renders source links', () => {
    render(<ChatMessage message={aiMsg} />);
    const link = screen.getByRole('link', { name: 'Doc A' });
    expect(link).toHaveAttribute('href', 'https://example.com');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('does not render sources section when no sources', () => {
    const msg = { ...aiMsg, sources: [] };
    render(<ChatMessage message={msg} />);
    expect(screen.queryByText('Sources:')).not.toBeInTheDocument();
  });

  it('calls onCopy when copy button is clicked', () => {
    const onCopy = jest.fn();
    Object.assign(navigator, {
      clipboard: { writeText: jest.fn().mockResolvedValue(undefined) },
    });
    render(<ChatMessage message={userMsg} onCopy={onCopy} />);
    fireEvent.click(screen.getByRole('button', { name: /copy message/i }));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('Hello world');
  });

  it('displays formatted timestamp element', () => {
    render(<ChatMessage message={userMsg} />);
    const time = document.querySelector('.chat-message__time');
    expect(time).toBeTruthy();
  });

  it('shows user avatar label', () => {
    render(<ChatMessage message={userMsg} />);
    expect(document.querySelector('.chat-message__avatar')).toHaveTextContent('U');
  });

  it('shows AI avatar label', () => {
    render(<ChatMessage message={aiMsg} />);
    expect(document.querySelector('.chat-message__avatar')).toHaveTextContent('AI');
  });
});
