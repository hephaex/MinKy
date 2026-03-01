import api, { API_BASE_URL } from './api';

const generateId = () => `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

/**
 * Parse SSE stream events from the response body
 * @param {ReadableStream} body - Response body stream
 * @param {Object} callbacks - Event callbacks
 * @param {Function} callbacks.onSources - Called with sources array
 * @param {Function} callbacks.onDelta - Called with text delta
 * @param {Function} callbacks.onDone - Called when streaming complete
 * @param {Function} callbacks.onError - Called on error
 */
const parseSSEStream = async (body, callbacks) => {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      // Process complete SSE events
      const events = buffer.split('\n\n');
      buffer = events.pop() || ''; // Keep incomplete event in buffer

      for (const event of events) {
        if (!event.trim()) continue;

        const dataLine = event.split('\n').find((line) => line.startsWith('data: '));
        if (!dataLine) continue;

        const jsonStr = dataLine.slice(6);
        try {
          const data = JSON.parse(jsonStr);

          switch (data.type) {
            case 'sources':
              callbacks.onSources?.(data.sources);
              break;
            case 'delta':
              callbacks.onDelta?.(data.text);
              break;
            case 'done':
              callbacks.onDone?.({
                tokensUsed: data.tokens_used,
                model: data.model,
              });
              break;
            case 'error':
              callbacks.onError?.(new Error(data.message));
              break;
            default:
              break;
          }
        } catch {
          // Skip malformed JSON
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
};

export const chatService = {
  sendMessage: async (sessionId, content) => {
    const response = await api.post('/chat/message', { session_id: sessionId, content });
    return response.data;
  },

  /**
   * Send message with streaming response
   * @param {string} question - User question
   * @param {Object} options - Request options
   * @param {Object} callbacks - Streaming callbacks
   * @returns {Promise<void>}
   */
  sendMessageStream: async (question, options = {}, callbacks = {}) => {
    const response = await fetch(`${API_BASE_URL}/search/ask/stream`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include',
      body: JSON.stringify({
        question,
        top_k: options.topK || 5,
        threshold: options.threshold || 0.7,
        include_sources: options.includeSources !== false,
        user_id: options.userId,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(error || 'Failed to get streaming response');
    }

    await parseSSEStream(response.body, callbacks);
  },

  getSessions: async () => {
    const response = await api.get('/chat/sessions');
    return response.data.data || response.data || [];
  },

  getSession: async (sessionId) => {
    const response = await api.get(`/chat/sessions/${sessionId}`);
    return response.data.data || response.data;
  },

  createSession: async (title) => {
    const response = await api.post('/chat/sessions', { title });
    return response.data.data || response.data;
  },

  deleteSession: async (sessionId) => {
    const response = await api.delete(`/chat/sessions/${sessionId}`);
    return response.data;
  },
};

export { generateId };
