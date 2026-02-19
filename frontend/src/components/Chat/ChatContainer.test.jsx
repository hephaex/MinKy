import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import ChatContainer from './ChatContainer';

// Mock react-markdown and related ESM-only modules
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

// Mock useChat hook
const mockUseChat = {
  sessions: [],
  activeSessionId: null,
  messages: [],
  isLoading: false,
  error: null,
  sendMessage: jest.fn(),
  selectSession: jest.fn(),
  createSession: jest.fn(),
  deleteSession: jest.fn(),
};

jest.mock('../../hooks/useChat', () => ({
  useChat: () => mockUseChat,
}));

beforeEach(() => {
  jest.clearAllMocks();
  mockUseChat.sessions = [];
  mockUseChat.activeSessionId = null;
  mockUseChat.messages = [];
  mockUseChat.isLoading = false;
  mockUseChat.error = null;
  mockUseChat.sendMessage = jest.fn();
  mockUseChat.selectSession = jest.fn();
  mockUseChat.createSession = jest.fn();
  mockUseChat.deleteSession = jest.fn();

  // scrollIntoView is not available in jsdom
  window.HTMLElement.prototype.scrollIntoView = jest.fn();
});

describe('ChatContainer', () => {
  it('renders without crashing', () => {
    render(<ChatContainer />);
    expect(document.querySelector('.chat-container')).toBeInTheDocument();
  });

  it('shows empty state when no messages', () => {
    render(<ChatContainer />);
    expect(screen.getByText("Ask your team's knowledge")).toBeInTheDocument();
  });

  it('renders messages when provided', () => {
    mockUseChat.messages = [
      { id: 'm1', role: 'user', content: 'Hello from user', timestamp: '2026-02-19T10:00:00Z' },
      { id: 'm2', role: 'assistant', content: 'Hello from AI', timestamp: '2026-02-19T10:00:01Z', sources: [] },
    ];
    render(<ChatContainer />);
    expect(screen.getByText('Hello from user')).toBeInTheDocument();
    expect(screen.getByText('Hello from AI')).toBeInTheDocument();
  });

  it('shows typing indicator when loading', () => {
    mockUseChat.isLoading = true;
    render(<ChatContainer />);
    expect(screen.getByLabelText('AI is typing')).toBeInTheDocument();
  });

  it('does not show typing indicator when not loading', () => {
    mockUseChat.isLoading = false;
    render(<ChatContainer />);
    expect(screen.queryByLabelText('AI is typing')).not.toBeInTheDocument();
  });

  it('shows error message when error is set', () => {
    mockUseChat.error = 'Something went wrong';
    render(<ChatContainer />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('does not show error element when error is null', () => {
    mockUseChat.error = null;
    render(<ChatContainer />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('passes className prop to root element', () => {
    render(<ChatContainer className="custom-class" />);
    expect(document.querySelector('.chat-container')).toHaveClass('custom-class');
  });

  it('chat-main has role log for accessibility', () => {
    render(<ChatContainer />);
    expect(screen.getByRole('log')).toBeInTheDocument();
  });

  it('passes correct props to ChatHistory', () => {
    const sessions = [
      { id: 's1', title: 'Chat 1', updatedAt: '2026-02-19T09:00:00Z' },
    ];
    mockUseChat.sessions = sessions;
    mockUseChat.activeSessionId = 's1';
    render(<ChatContainer />);
    // ChatHistory should render the session title
    expect(screen.getByText('Chat 1')).toBeInTheDocument();
  });

  it('calls sendMessage when input is submitted', async () => {
    mockUseChat.sendMessage.mockResolvedValue(undefined);
    render(<ChatContainer />);
    const textarea = screen.getByRole('textbox');
    fireEvent.change(textarea, { target: { value: 'Test message' } });
    fireEvent.keyDown(textarea, { key: 'Enter', shiftKey: false });
    await waitFor(() => {
      expect(mockUseChat.sendMessage).toHaveBeenCalledWith('Test message');
    });
  });

  it('disables input when loading', () => {
    mockUseChat.isLoading = true;
    render(<ChatContainer />);
    const textarea = screen.getByRole('textbox');
    expect(textarea).toBeDisabled();
  });

  it('enables input when not loading', () => {
    mockUseChat.isLoading = false;
    render(<ChatContainer />);
    const textarea = screen.getByRole('textbox');
    expect(textarea).not.toBeDisabled();
  });

  it('does not show empty state when messages exist', () => {
    mockUseChat.messages = [
      { id: 'm1', role: 'user', content: 'A message', timestamp: '2026-02-19T10:00:00Z' },
    ];
    render(<ChatContainer />);
    expect(screen.queryByText("Ask your team's knowledge")).not.toBeInTheDocument();
  });

  it('calls createSession when New button in ChatHistory is clicked', async () => {
    mockUseChat.createSession.mockResolvedValue(undefined);
    render(<ChatContainer />);
    // ChatHistory renders a "+ New" or "New Chat" button
    const newBtn = screen.getByRole('button', { name: /new chat/i });
    await act(async () => {
      fireEvent.click(newBtn);
    });
    expect(mockUseChat.createSession).toHaveBeenCalled();
  });

  it('shows multiple messages in order', () => {
    mockUseChat.messages = [
      { id: 'm1', role: 'user', content: 'First message', timestamp: '2026-02-19T10:00:00Z' },
      { id: 'm2', role: 'assistant', content: 'Second message', timestamp: '2026-02-19T10:00:01Z', sources: [] },
      { id: 'm3', role: 'user', content: 'Third message', timestamp: '2026-02-19T10:00:02Z' },
    ];
    render(<ChatContainer />);
    const messages = document.querySelectorAll('.chat-message');
    expect(messages).toHaveLength(3);
  });

  it('shows empty suggestion list in empty state', () => {
    render(<ChatContainer />);
    const suggestions = document.querySelectorAll('.chat-empty__suggestions li');
    expect(suggestions.length).toBeGreaterThan(0);
  });
});
