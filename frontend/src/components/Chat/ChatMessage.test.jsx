import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import ChatMessage from './ChatMessage';

const renderWithRouter = (ui) => {
  return render(<BrowserRouter>{ui}</BrowserRouter>);
};

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
  sources: [{ document_title: 'Doc A', document_id: 'doc-1', chunk_text: 'Preview text for the document...', similarity: 0.95 }],
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
    const { container } = renderWithRouter(<ChatMessage message={aiMsg} />);
    expect(container.querySelector('.chat-message')).toHaveClass('chat-message--ai');
  });

  it('renders assistant message content', () => {
    renderWithRouter(<ChatMessage message={aiMsg} />);
    expect(screen.getByText('Response text')).toBeInTheDocument();
  });

  it('renders source cards', () => {
    renderWithRouter(<ChatMessage message={aiMsg} />);
    // Check source card title is rendered
    expect(screen.getByText('Doc A')).toBeInTheDocument();
    // Check similarity percentage is displayed
    expect(screen.getByText('95%')).toBeInTheDocument();
    // Check preview text is displayed
    expect(screen.getByText(/Preview text for the document/)).toBeInTheDocument();
    // Check source number badge
    expect(screen.getByText('[1]')).toBeInTheDocument();
  });

  it('does not render sources section when no sources', () => {
    const msg = { ...aiMsg, sources: [] };
    const { container } = renderWithRouter(<ChatMessage message={msg} />);
    expect(container.querySelector('.chat-message__sources')).not.toBeInTheDocument();
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
    renderWithRouter(<ChatMessage message={aiMsg} />);
    expect(document.querySelector('.chat-message__avatar')).toHaveTextContent('AI');
  });
});
