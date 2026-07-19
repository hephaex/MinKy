import { renderHook, act, waitFor } from '@testing-library/react';
import { useChat } from './useChat';
import { chatService } from '../services/chatService';

let mockIdCounter = 0;
jest.mock('../services/chatService', () => ({
  chatService: {
    getSessions: jest.fn(),
    getSession: jest.fn(),
    createSession: jest.fn(),
    deleteSession: jest.fn(),
    sendMessage: jest.fn(),
    sendMessageStream: jest.fn(),
    appendMessage: jest.fn(),
  },
  generateId: () => `mock-id-${++mockIdCounter}`,
}));

describe('useChat', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockIdCounter = 0;
    chatService.getSessions.mockResolvedValue([]);
    chatService.createSession.mockResolvedValue({
      id: 'new-session',
      title: 'New Chat',
      updatedAt: new Date().toISOString(),
    });
    chatService.appendMessage.mockResolvedValue({});
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

  it('sendMessage appends user and AI messages (non-streaming)', async () => {
    // Test non-streaming path for simpler mocking
    chatService.sendMessage.mockResolvedValue({
      content: 'AI response',
      sources: [],
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('Hello', { streaming: false });
    });

    expect(result.current.messages).toHaveLength(2);
    expect(result.current.messages[0].role).toBe('user');
    expect(result.current.messages[0].content).toBe('Hello');
    expect(result.current.messages[1].role).toBe('assistant');
    expect(result.current.messages[1].content).toBe('AI response');
  });

  it('sendMessage with streaming calls sendMessageStream', async () => {
    // Just verify streaming method is called
    chatService.sendMessageStream.mockImplementation(async (question, options, callbacks) => {
      callbacks.onDone({ tokensUsed: 10, model: 'test-model' });
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('Hello');
    });

    expect(chatService.sendMessageStream).toHaveBeenCalledWith(
      'Hello',
      expect.objectContaining({ topK: 5, threshold: 0.7 }),
      expect.objectContaining({
        onSources: expect.any(Function),
        onDelta: expect.any(Function),
        onDone: expect.any(Function),
        onError: expect.any(Function),
      })
    );
  });

  it('persists the user and completed assistant message after streaming finishes', async () => {
    // Reload-survival is the entire point of session persistence: once the
    // stream finishes, both turns of this exchange must be written back so
    // GET /chat/sessions/:id can reconstruct them later.
    chatService.sendMessageStream.mockImplementation(async (question, options, callbacks) => {
      callbacks.onDelta('Hello ');
      callbacks.onDelta('there!');
      callbacks.onDone({ tokensUsed: 10, model: 'solar-open2' });
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('Hi');
    });

    expect(chatService.appendMessage).toHaveBeenCalledWith(
      'new-session',
      expect.objectContaining({ role: 'user', content: 'Hi' })
    );
    expect(chatService.appendMessage).toHaveBeenCalledWith(
      'new-session',
      expect.objectContaining({
        role: 'assistant',
        content: 'Hello there!',
        tokensUsed: 10,
        model: 'solar-open2',
      })
    );
  });

  it('does not persist an assistant message when streaming errors', async () => {
    chatService.sendMessageStream.mockImplementation(async (question, options, callbacks) => {
      callbacks.onError(new Error('Server error'));
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('fail');
    });

    expect(chatService.appendMessage).toHaveBeenCalledWith(
      'new-session',
      expect.objectContaining({ role: 'user' })
    );
    expect(chatService.appendMessage).not.toHaveBeenCalledWith(
      'new-session',
      expect.objectContaining({ role: 'assistant' })
    );
  });

  it('does not block the send flow when persistence fails', async () => {
    chatService.appendMessage.mockRejectedValue(new Error('network down'));
    chatService.sendMessageStream.mockImplementation(async (question, options, callbacks) => {
      callbacks.onDelta('ok');
      callbacks.onDone({});
    });

    const { result } = renderHook(() => useChat());
    await waitFor(() => expect(result.current.sessions).toBeDefined());

    await act(async () => {
      await result.current.sendMessage('Hi');
    });

    expect(result.current.messages).toHaveLength(2);
    expect(result.current.error).toBeNull();
  });

  it('sets error message on sendMessage failure', async () => {
    // Mock streaming to call error callback
    chatService.sendMessageStream.mockImplementation(async (question, options, callbacks) => {
      callbacks.onError(new Error('Server error'));
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
