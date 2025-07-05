import axios from 'axios';

const API_BASE_URL = process.env.REACT_APP_API_URL || '/api';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

export const documentService = {
  getDocuments: async (page = 1, perPage = 10, search = '') => {
    const params = { page, per_page: perPage };
    if (search) {
      params.search = search;
    }
    const response = await api.get('/documents', { params });
    return response.data;
  },

  getDocument: async (id) => {
    const response = await api.get(`/documents/${id}`);
    return response.data;
  },

  createDocument: async (document) => {
    const response = await api.post('/documents', document);
    return response.data;
  },

  updateDocument: async (id, document) => {
    const response = await api.put(`/documents/${id}`, document);
    return response.data;
  },

  deleteDocument: async (id) => {
    const response = await api.delete(`/documents/${id}`);
    return response.data;
  },
};

export default api;