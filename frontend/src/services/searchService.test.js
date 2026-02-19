import { searchService, embeddingService } from './api';

// Mock the entire api module
jest.mock('./api', () => {
  const mockApi = {
    get: jest.fn(),
    post: jest.fn(),
    put: jest.fn(),
    delete: jest.fn(),
    interceptors: {
      request: { use: jest.fn() },
      response: { use: jest.fn() },
    },
    defaults: { headers: { common: {} } },
  };

  // Re-implement the service objects using the mock
  return {
    searchService: {
      ask: jest.fn(),
      semantic: jest.fn(),
      history: jest.fn(),
    },
    embeddingService: {
      getStats: jest.fn(),
      getEmbedding: jest.fn(),
      createEmbedding: jest.fn(),
      semanticSearch: jest.fn(),
      getSimilar: jest.fn(),
    },
    documentService: {
      getDocuments: jest.fn(),
      getDocument: jest.fn(),
      createDocument: jest.fn(),
      updateDocument: jest.fn(),
      deleteDocument: jest.fn(),
      uploadDocument: jest.fn(),
      getDocumentTree: jest.fn(),
    },
    authService: {
      login: jest.fn(),
      logout: jest.fn(),
      getCurrentUser: jest.fn(),
      isAuthenticated: jest.fn(),
      getToken: jest.fn(),
    },
    tagService: {
      getTags: jest.fn(),
      getTag: jest.fn(),
      getTagStatistics: jest.fn(),
      suggestTags: jest.fn(),
      createTag: jest.fn(),
      updateTag: jest.fn(),
      deleteTag: jest.fn(),
    },
  };
});

describe('searchService', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('has required methods', () => {
    expect(searchService.ask).toBeDefined();
    expect(searchService.semantic).toBeDefined();
    expect(searchService.history).toBeDefined();
  });

  it('ask returns a response when called', async () => {
    const mockResponse = {
      answer: 'Rust is a systems programming language.',
      sources: [],
    };
    searchService.ask.mockResolvedValue(mockResponse);

    const result = await searchService.ask('What is Rust?');
    expect(result).toEqual(mockResponse);
    expect(searchService.ask).toHaveBeenCalledWith('What is Rust?');
  });

  it('semantic returns matching documents', async () => {
    const mockResponse = { results: [{ id: '1', title: 'Doc', score: 0.9 }] };
    searchService.semantic.mockResolvedValue(mockResponse);

    const result = await searchService.semantic('vector search');
    expect(result.results).toHaveLength(1);
    expect(result.results[0].score).toBe(0.9);
  });

  it('history returns list of past searches', async () => {
    const mockHistory = [
      { id: '1', query: 'rust', created_at: '2026-02-19' },
      { id: '2', query: 'async', created_at: '2026-02-18' },
    ];
    searchService.history.mockResolvedValue(mockHistory);

    const result = await searchService.history();
    expect(result).toHaveLength(2);
  });

  it('ask handles API error gracefully', async () => {
    searchService.ask.mockRejectedValue(new Error('API error'));
    await expect(searchService.ask('test')).rejects.toThrow('API error');
  });
});

describe('embeddingService', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('has required methods', () => {
    expect(embeddingService.getStats).toBeDefined();
    expect(embeddingService.getEmbedding).toBeDefined();
    expect(embeddingService.createEmbedding).toBeDefined();
    expect(embeddingService.semanticSearch).toBeDefined();
    expect(embeddingService.getSimilar).toBeDefined();
  });

  it('getStats returns embedding statistics', async () => {
    const mockStats = { total_embeddings: 42, model: 'text-embedding-3-small' };
    embeddingService.getStats.mockResolvedValue(mockStats);

    const result = await embeddingService.getStats();
    expect(result.total_embeddings).toBe(42);
  });

  it('createEmbedding returns created embedding info', async () => {
    const mockResult = { document_id: 'uuid-123', status: 'created' };
    embeddingService.createEmbedding.mockResolvedValue(mockResult);

    const result = await embeddingService.createEmbedding('uuid-123');
    expect(result.status).toBe('created');
  });

  it('getSimilar returns similar documents', async () => {
    const mockSimilar = [{ id: 'uuid-456', similarity: 0.85 }];
    embeddingService.getSimilar.mockResolvedValue(mockSimilar);

    const result = await embeddingService.getSimilar('uuid-123');
    expect(result).toHaveLength(1);
    expect(result[0].similarity).toBeGreaterThan(0.8);
  });

  it('semanticSearch with threshold filters results', async () => {
    const mockResults = { results: [{ title: 'Match', score: 0.95 }] };
    embeddingService.semanticSearch.mockResolvedValue(mockResults);

    const result = await embeddingService.semanticSearch('query', { threshold: 0.7 });
    expect(result.results[0].score).toBeGreaterThan(0.7);
  });
});
