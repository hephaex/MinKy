import { loadListState, saveListState } from './listStatePersistence';

describe('listStatePersistence', () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it('round-trips a state object through sessionStorage', () => {
    const state = { currentPage: 3, searchQuery: 'react', selectedTags: ['a', 'b'] };
    saveListState('documentListState', state);
    expect(loadListState('documentListState')).toEqual(state);
  });

  it('returns null for an unknown key', () => {
    expect(loadListState('missingKey')).toBeNull();
  });

  it('returns null (not throw) on corrupted JSON', () => {
    sessionStorage.setItem('documentListState', '{not valid json');
    expect(loadListState('documentListState')).toBeNull();
  });

  it('keeps separate state per key', () => {
    saveListState('tagDetailState:rust', { currentPage: 2 });
    saveListState('tagDetailState:python', { currentPage: 5 });
    expect(loadListState('tagDetailState:rust')).toEqual({ currentPage: 2 });
    expect(loadListState('tagDetailState:python')).toEqual({ currentPage: 5 });
  });

  it('does not throw when sessionStorage.setItem fails', () => {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = () => {
      throw new Error('QuotaExceededError');
    };
    expect(() => saveListState('documentListState', { currentPage: 1 })).not.toThrow();
    Storage.prototype.setItem = original;
  });
});
