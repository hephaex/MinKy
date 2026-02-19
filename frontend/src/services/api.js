import axios from 'axios';

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8000/api';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add token to requests if available
api.interceptors.request.use((config) => {
  const token = localStorage.getItem('token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  
  // For FormData uploads, remove default Content-Type to let axios set the boundary
  if (config.data instanceof FormData) {
    delete config.headers['Content-Type'];
  }
  
  return config;
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

  uploadDocument: async (file) => {
    const formData = new FormData();
    formData.append('file', file);

    const response = await api.post('/documents/upload', formData, {
      timeout: 120000,
    });
    return response.data;
  },

  previewBackupSync: async () => {
    const response = await api.get('/documents/sync/preview');
    return response.data;
  },

  syncBackupFiles: async (dryRun = false) => {
    const response = await api.post('/documents/sync', { dry_run: dryRun });
    return response.data;
  },

  exportAllDocuments: async (shortFilename = false) => {
    const response = await api.post('/documents/export', { short_filename: shortFilename });
    return response.data;
  },

  getDocumentTree: async (mode = 'by-tag') => {
    const response = await api.get('/documents/tree', { params: { mode } });
    return response.data;
  },
};

export const authService = {
  // SECURITY TODO: localStorage is vulnerable to XSS attacks.
  // Consider migrating to HttpOnly cookies for token storage.
  // Backend should set: Set-Cookie: token=<jwt>; HttpOnly; Secure; SameSite=Strict
  login: async (credentials) => {
    const response = await api.post('/auth/login', credentials);
    if (response.data.access_token) {
      localStorage.setItem('token', response.data.access_token);
      api.defaults.headers.common['Authorization'] = `Bearer ${response.data.access_token}`;
    }
    return response.data;
  },

  logout: async () => {
    localStorage.removeItem('token');
    delete api.defaults.headers.common['Authorization'];
  },

  getCurrentUser: async () => {
    const response = await api.get('/auth/me');
    return response.data;
  },

  isAuthenticated: () => {
    return !!localStorage.getItem('token');
  },

  getToken: () => {
    return localStorage.getItem('token');
  }
};

export const tagService = {
  getTags: async (page = 1, perPage = 20, search = '', popular = false) => {
    const params = { page, per_page: perPage };
    if (search) params.search = search;
    if (popular) params.popular = 'true';

    const response = await api.get('/tags', { params });
    return response.data.data || response.data;
  },

  getTag: async (slug, page = 1, perPage = 10) => {
    const params = { page, per_page: perPage };
    const response = await api.get(`/tags/${slug}`, { params });
    return response.data.data || response.data;
  },

  getTagStatistics: async () => {
    const response = await api.get('/tags/statistics');
    return response.data.data || response.data;
  },

  suggestTags: async (query, limit = 10) => {
    const params = { q: query, limit };
    const response = await api.get('/tags/suggest', { params });
    return response.data.data || response.data;
  },

  createTag: async (tagData) => {
    const response = await api.post('/tags', tagData);
    return response.data.data || response.data;
  },

  updateTag: async (slug, tagData) => {
    const response = await api.put(`/tags/${slug}`, tagData);
    return response.data.data || response.data;
  },

  deleteTag: async (slug) => {
    const response = await api.delete(`/tags/${slug}`);
    return response.data.data || response.data;
  },
};

export const searchService = {
  ask: async (question, options = {}) => {
    const response = await api.post('/search/ask', {
      question,
      max_results: options.maxResults || 5,
      ...options,
    });
    return response.data;
  },

  semantic: async (query, options = {}) => {
    const response = await api.post('/search/semantic', {
      query,
      limit: options.limit || 10,
      threshold: options.threshold || 0.7,
      ...options,
    });
    return response.data;
  },

  getSimilar: async (documentId, limit = 5) => {
    const response = await api.get(`/embeddings/similar/${documentId}`, {
      params: { limit },
    });
    return response.data;
  },
};

export default api;