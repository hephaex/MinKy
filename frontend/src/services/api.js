import axios from 'axios';

const API_BASE_URL = process.env.REACT_APP_API_URL || 'http://localhost:8000/api';

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
  // Enable cookies for cross-origin requests (HttpOnly cookies)
  withCredentials: true,
});

// Request interceptor for FormData handling
api.interceptors.request.use((config) => {
  // For FormData uploads, remove default Content-Type to let axios set the boundary
  if (config.data instanceof FormData) {
    delete config.headers['Content-Type'];
  }

  return config;
});

// Response interceptor for handling auth errors
api.interceptors.response.use(
  (response) => response,
  async (error) => {
    const originalRequest = error.config;

    // If 401 and not already retrying, try to refresh token
    if (error.response?.status === 401 && !originalRequest._retry) {
      originalRequest._retry = true;

      try {
        // Try to refresh the token (cookie is automatically sent)
        await api.post('/auth/refresh', {});
        // Retry the original request
        return api(originalRequest);
      } catch (refreshError) {
        // Refresh failed, user needs to login again
        window.dispatchEvent(new CustomEvent('auth:logout'));
        return Promise.reject(error);
      }
    }

    return Promise.reject(error);
  }
);

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
  // Auth tokens are now stored in HttpOnly cookies (set by backend)
  // This prevents XSS attacks from accessing the tokens
  login: async (credentials) => {
    const response = await api.post('/auth/login', credentials);
    // Token is automatically set as HttpOnly cookie by the backend
    // Store user info in memory for quick access
    if (response.data.user) {
      sessionStorage.setItem('user', JSON.stringify(response.data.user));
    }
    return response.data;
  },

  logout: async () => {
    try {
      // Call backend to clear HttpOnly cookies
      await api.post('/auth/logout');
    } finally {
      // Clear user info from session storage
      sessionStorage.removeItem('user');
    }
  },

  getCurrentUser: async () => {
    const response = await api.get('/auth/me');
    return response.data;
  },

  // Check if user is authenticated by verifying session
  // Since we can't access HttpOnly cookies, we verify with the server
  isAuthenticated: async () => {
    try {
      await api.get('/auth/me');
      return true;
    } catch {
      return false;
    }
  },

  // Sync check using cached user info
  isAuthenticatedSync: () => {
    return !!sessionStorage.getItem('user');
  },

  // Get cached user info
  getCachedUser: () => {
    const user = sessionStorage.getItem('user');
    return user ? JSON.parse(user) : null;
  },

  // Refresh token (called automatically by interceptor)
  refreshToken: async () => {
    const response = await api.post('/auth/refresh', {});
    return response.data;
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