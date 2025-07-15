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

  uploadDocument: async (file) => {
    const formData = new FormData();
    formData.append('file', file);
    
    console.log('Uploading file:', file.name, 'size:', file.size);
    console.log('FormData contents:', formData.get('file'));
    
    // Try direct backend connection (bypassing nginx proxy)
    const directResponse = await axios.post('http://localhost:5001/api/documents/upload', formData, {
      timeout: 120000,
      validateStatus: function (status) {
        return status >= 200 && status < 300;
      },
    });
    return directResponse.data;
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
};

export const tagService = {
  getTags: async (page = 1, perPage = 20, search = '', popular = false) => {
    const params = { page, per_page: perPage };
    if (search) params.search = search;
    if (popular) params.popular = 'true';
    
    const response = await api.get('/tags', { params });
    return response.data;
  },

  getTag: async (slug, page = 1, perPage = 10) => {
    const params = { page, per_page: perPage };
    const response = await api.get(`/tags/${slug}`, { params });
    return response.data;
  },

  getTagStatistics: async () => {
    const response = await api.get('/tags/statistics');
    return response.data;
  },

  suggestTags: async (query, limit = 10) => {
    const params = { q: query, limit };
    const response = await api.get('/tags/suggest', { params });
    return response.data;
  },

  createTag: async (tagData) => {
    const response = await api.post('/tags', tagData);
    return response.data;
  },

  updateTag: async (slug, tagData) => {
    const response = await api.put(`/tags/${slug}`, tagData);
    return response.data;
  },

  deleteTag: async (slug) => {
    const response = await api.delete(`/tags/${slug}`);
    return response.data;
  },
};

export default api;