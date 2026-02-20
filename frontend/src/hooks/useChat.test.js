import { renderHook, act, waitFor } from '@testing-library/react';
import { useChat } from './useChat';
import { chatService } from '../services/chatService';

jest.mock('../services/chatService', () => ({
  chatService: {
    getSessions: jest.fn(),
    getSession: jest.fn(),
    createSession: jest.fn(),
    deleteSession: jest.fn(),
    sendMessage: jest.fn(),
  },
  generateId: () => 'mock-id',
}));

describe('useChat', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    chatService.getSessions.mockResolvedValue([]);
    chatService.createSession.mockResolvedValue({ id: 'new-session', title: 'New Chat', updatedAt: new Date().toISOString() });
  });

  it('initializes with empty state', async () => {
    const { result } = renderHook(() => useChat());
    await waitFor(() => {
      expect(result.current.sessions).toEqual([]);
    });
    expect(result.current.messages).toEqual([]);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.activeSessionId).toBeNull();
  });

  it('loads sessions on mount', async () => {
    const mockSessions = [{ id: 's1', title: 'Test', updatedAt: '' }];
    chatService.getSessions.mockResolvedValue(mockSessions);
    const { result } = renderHook(() => useChat());
    await waitFor(() => {
      expect(result.current.sessions).toEqual(mockSessions);
    });
  });

  it('createSession adds session and sets it active', async () => {
    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.createSession();
    });

    expect(result.current.activeSessionId).toBe('new-session');
    expect(result.current.sessions).toHaveLength(1);
    expect(result.current.messages).toEqual([]);
  });

  it('sendMessage appends user and AI messages', async () => {
    chatService.sendMessage.mockResolvedValue({
      content: 'AI response',
      sources: [],
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('Hello');
    });

    expect(result.current.messages).toHaveLength(2);
    expect(result.current.messages[0].role).toBe('user');
    expect(result.current.messages[0].content).toBe('Hello');
    expect(result.current.messages[1].role).toBe('assistant');
    expect(result.current.messages[1].content).toBe('AI response');
  });

  it('sets error message on sendMessage failure', async () => {
    chatService.sendMessage.mockRejectedValue({
      response: { data: { error: 'Server error' } },
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('fail');
    });

    expect(result.current.error).toBe('Server error');
  });

  it('deleteSession removes session from list', async () => {
    const mockSessions = [{ id: 's1', title: 'One', updatedAt: '' }];
    chatService.getSessions.mockResolvedValue(mockSessions);
    chatService.deleteSession.mockResolvedValue({});

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toHaveLength(1));

    await act(async () => {
      await result.current.deleteSession('s1');
    });

    expect(result.current.sessions).toHaveLength(0);
  });

  it('selectSession loads session messages', async () => {
    chatService.getSession.mockResolvedValue({
      messages: [{ id: 'm1', role: 'user', content: 'Hi', timestamp: '' }],
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.selectSession('s1');
    });

    expect(result.current.messages).toHaveLength(1);
    expect(result.current.activeSessionId).toBe('s1');
  });
});
