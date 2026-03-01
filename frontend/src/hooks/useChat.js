import { useState, useCallback, useEffect, useRef } from 'react';
import { chatService, generateId } from '../services/chatService';

const buildUserMessage = (content) => ({
  id: generateId(),
  role: 'user',
  content,
  timestamp: new Date().toISOString(),
});

const buildAssistantMessage = (id, content = '', sources = [], isStreaming = false) => ({
  id,
  role: 'assistant',
  content,
  sources,
  timestamp: new Date().toISOString(),
  isStreaming,
});

const buildErrorMessage = (text) => ({
  id: generateId(),
  role: 'assistant',
  content: text,
  timestamp: new Date().toISOString(),
  isError: true,
});

export const useChat = () => {
  const [sessions, setSessions] = useState([]);
  const [activeSessionId, setActiveSessionId] = useState(null);
  const [messages, setMessages] = useState([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState(null);
  const [streamingMessageId, setStreamingMessageId] = useState(null);
  const streamingContentRef = useRef('');

  const loadSessions = useCallback(async () => {
    try {
      const data = await chatService.getSessions();
      setSessions(data);
    } catch {
      // Sessions load is best-effort; start fresh if backend unavailable
      setSessions([]);
    }
  }, []);

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  const selectSession = useCallback(async (sessionId) => {
    setError(null);
    setActiveSessionId(sessionId);
    try {
      const session = await chatService.getSession(sessionId);
      setMessages(session.messages || []);
    } catch {
      setMessages([]);
      setError('Failed to load conversation.');
    }
  }, []);

  const createSession = useCallback(async () => {
    setError(null);
    try {
      const session = await chatService.createSession('New Chat');
      setSessions((prev) => [session, ...prev]);
      setActiveSessionId(session.id);
      setMessages([]);
    } catch {
      // Fallback: local-only session when backend unavailable
      const localId = generateId();
      const localSession = {
        id: localId,
        title: 'New Chat',
        updatedAt: new Date().toISOString(),
      };
      setSessions((prev) => [localSession, ...prev]);
      setActiveSessionId(localId);
      setMessages([]);
    }
  }, []);

  const deleteSession = useCallback(
    async (sessionId) => {
      try {
        await chatService.deleteSession(sessionId);
      } catch {
        // ignore deletion errors
      }
      setSessions((prev) => prev.filter((s) => s.id !== sessionId));
      if (activeSessionId === sessionId) {
        setActiveSessionId(null);
        setMessages([]);
      }
    },
    [activeSessionId]
  );

  const sendMessage = useCallback(
    async (content, options = {}) => {
      setError(null);
      const userMsg = buildUserMessage(content);
      setMessages((prev) => [...prev, userMsg]);
      setIsLoading(true);

      let sessionId = activeSessionId;

      try {
        if (!sessionId) {
          try {
            const session = await chatService.createSession(content.slice(0, 60));
            sessionId = session.id;
            setSessions((prev) => [session, ...prev]);
            setActiveSessionId(sessionId);
          } catch {
            sessionId = generateId();
            setActiveSessionId(sessionId);
          }
        }

        // Use streaming by default
        const useStreaming = options.streaming !== false;

        if (useStreaming) {
          const aiMsgId = generateId();
          streamingContentRef.current = '';
          setStreamingMessageId(aiMsgId);

          // Add initial empty message for streaming
          setMessages((prev) => [...prev, buildAssistantMessage(aiMsgId, '', [], true)]);

          let sources = [];

          await chatService.sendMessageStream(
            content,
            {
              topK: options.topK || 5,
              threshold: options.threshold || 0.7,
              includeSources: true,
            },
            {
              onSources: (receivedSources) => {
                sources = receivedSources;
                setMessages((prev) =>
                  prev.map((msg) =>
                    msg.id === aiMsgId ? { ...msg, sources: receivedSources } : msg
                  )
                );
              },
              onDelta: (text) => {
                streamingContentRef.current += text;
                setMessages((prev) =>
                  prev.map((msg) =>
                    msg.id === aiMsgId ? { ...msg, content: streamingContentRef.current } : msg
                  )
                );
              },
              onDone: ({ tokensUsed, model }) => {
                setMessages((prev) =>
                  prev.map((msg) =>
                    msg.id === aiMsgId
                      ? {
                          ...msg,
                          isStreaming: false,
                          tokensUsed,
                          model,
                        }
                      : msg
                  )
                );
                setStreamingMessageId(null);
                streamingContentRef.current = '';
              },
              onError: (err) => {
                setError(err.message);
                setMessages((prev) =>
                  prev.map((msg) =>
                    msg.id === aiMsgId
                      ? {
                          ...msg,
                          content: err.message,
                          isStreaming: false,
                          isError: true,
                        }
                      : msg
                  )
                );
                setStreamingMessageId(null);
                streamingContentRef.current = '';
              },
            }
          );
        } else {
          // Non-streaming fallback
          const response = await chatService.sendMessage(sessionId, content);
          const aiMsg = {
            id: generateId(),
            role: 'assistant',
            content: response.content || response.message || '',
            timestamp: new Date().toISOString(),
            sources: response.sources || [],
          };
          setMessages((prev) => [...prev, aiMsg]);
        }

        setSessions((prev) =>
          prev.map((s) =>
            s.id === sessionId
              ? {
                  ...s,
                  title: s.title === 'New Chat' ? content.slice(0, 60) : s.title,
                  updatedAt: new Date().toISOString(),
                }
              : s
          )
        );
      } catch (err) {
        const errorText =
          err?.response?.data?.error ||
          err?.message ||
          'Failed to get a response. Please try again.';
        setError(errorText);
        setMessages((prev) => [...prev, buildErrorMessage(errorText)]);
      } finally {
        setIsLoading(false);
      }
    },
    [activeSessionId]
  );

  return {
    sessions,
    activeSessionId,
    messages,
    isLoading,
    isStreaming: streamingMessageId !== null,
    streamingMessageId,
    error,
    sendMessage,
    selectSession,
    createSession,
    deleteSession,
  };
};
