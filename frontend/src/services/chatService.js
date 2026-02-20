import api from './api';

const generateId = () =>
  `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

export const chatService = {
  sendMessage: async (sessionId, content) => {
    const response = await api.post('/chat/message', { session_id: sessionId, content });
    return response.data;
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
