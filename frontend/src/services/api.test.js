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
  let mockSessionGetItem;
  let mockSessionSetItem;
  let mockSessionRemoveItem;

  beforeEach(() => {
    // Setup sessionStorage mock for each test (used for cached user info)
    mockSessionGetItem = jest.fn();
    mockSessionSetItem = jest.fn();
    mockSessionRemoveItem = jest.fn();

    Object.defineProperty(window, 'sessionStorage', {
      value: {
        getItem: mockSessionGetItem,
        setItem: mockSessionSetItem,
        removeItem: mockSessionRemoveItem,
        clear: jest.fn(),
      },
      writable: true,
    });
  });

  describe('isAuthenticatedSync', () => {
    it('returns true when user is cached in sessionStorage', () => {
      mockSessionGetItem.mockReturnValue('{"id":1,"email":"test@test.com"}');
      expect(authService.isAuthenticatedSync()).toBe(true);
    });

    it('returns false when no user is cached', () => {
      mockSessionGetItem.mockReturnValue(null);
      expect(authService.isAuthenticatedSync()).toBe(false);
    });
  });

  describe('getCachedUser', () => {
    it('returns parsed user from sessionStorage', () => {
      const user = { id: 1, email: 'test@test.com', username: 'testuser' };
      mockSessionGetItem.mockReturnValue(JSON.stringify(user));
      expect(authService.getCachedUser()).toEqual(user);
    });

    it('returns null when no user cached', () => {
      mockSessionGetItem.mockReturnValue(null);
      expect(authService.getCachedUser()).toBeNull();
    });
  });

  describe('logout', () => {
    it('removes user from sessionStorage', async () => {
      // Mock the API post call (logout endpoint)
      const mockApi = require('./api').default;
      mockApi.post = jest.fn().mockResolvedValue({ data: { success: true } });

      await authService.logout();
      expect(mockSessionRemoveItem).toHaveBeenCalledWith('user');
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
