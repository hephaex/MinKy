import axios from 'axios';
import { documentService, authService, tagService } from './api';

// Mock axios
jest.mock('axios', () => ({
  create: jest.fn(() => ({
    get: jest.fn(),
    post: jest.fn(),
    put: jest.fn(),
    delete: jest.fn(),
    interceptors: {
      request: { use: jest.fn() },
      response: { use: jest.fn() },
    },
    defaults: {
      headers: { common: {} },
    },
  })),
  post: jest.fn(),
}));

describe('authService', () => {
  let mockGetItem;
  let mockSetItem;
  let mockRemoveItem;

  beforeEach(() => {
    // Setup localStorage mock for each test
    mockGetItem = jest.fn();
    mockSetItem = jest.fn();
    mockRemoveItem = jest.fn();

    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: mockGetItem,
        setItem: mockSetItem,
        removeItem: mockRemoveItem,
        clear: jest.fn(),
      },
      writable: true,
    });
  });

  describe('isAuthenticated', () => {
    it('returns true when token exists', () => {
      mockGetItem.mockReturnValue('test-token');
      expect(authService.isAuthenticated()).toBe(true);
    });

    it('returns false when token does not exist', () => {
      mockGetItem.mockReturnValue(null);
      expect(authService.isAuthenticated()).toBe(false);
    });
  });

  describe('getToken', () => {
    it('returns token from localStorage', () => {
      mockGetItem.mockReturnValue('my-token');
      expect(authService.getToken()).toBe('my-token');
    });

    it('returns null when no token', () => {
      mockGetItem.mockReturnValue(null);
      expect(authService.getToken()).toBeNull();
    });
  });

  describe('logout', () => {
    it('removes token from localStorage', async () => {
      await authService.logout();
      expect(mockRemoveItem).toHaveBeenCalledWith('token');
    });
  });
});

describe('documentService', () => {
  it('has required methods', () => {
    expect(documentService.getDocuments).toBeDefined();
    expect(documentService.getDocument).toBeDefined();
    expect(documentService.createDocument).toBeDefined();
    expect(documentService.updateDocument).toBeDefined();
    expect(documentService.deleteDocument).toBeDefined();
    expect(documentService.uploadDocument).toBeDefined();
    expect(documentService.getDocumentTree).toBeDefined();
  });
});

describe('tagService', () => {
  it('has required methods', () => {
    expect(tagService.getTags).toBeDefined();
    expect(tagService.getTag).toBeDefined();
    expect(tagService.getTagStatistics).toBeDefined();
    expect(tagService.suggestTags).toBeDefined();
    expect(tagService.createTag).toBeDefined();
    expect(tagService.updateTag).toBeDefined();
    expect(tagService.deleteTag).toBeDefined();
  });
});
