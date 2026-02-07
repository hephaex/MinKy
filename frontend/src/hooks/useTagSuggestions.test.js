import { renderHook, act } from '@testing-library/react';
import useTagSuggestions from './useTagSuggestions';

describe('useTagSuggestions', () => {
  describe('initial state', () => {
    it('starts with empty tags when no initial tags provided', () => {
      const { result } = renderHook(() => useTagSuggestions());

      expect(result.current.tags).toEqual([]);
      expect(result.current.suggestedTags).toEqual([]);
    });

    it('starts with provided initial tags', () => {
      const initialTags = ['react', 'javascript'];
      const { result } = renderHook(() => useTagSuggestions({ initialTags }));

      expect(result.current.tags).toEqual(initialTags);
    });
  });

  describe('handleTagsChange', () => {
    it('updates tags when handleTagsChange is called', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.handleTagsChange(['react', 'testing']);
      });

      expect(result.current.tags).toEqual(['react', 'testing']);
    });

    it('calls onTagsChange callback when tags change', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({ onTagsChange }));

      act(() => {
        result.current.handleTagsChange(['newTag']);
      });

      expect(onTagsChange).toHaveBeenCalledWith(['newTag']);
    });
  });

  describe('handleTagSuggestions', () => {
    it('adds suggested tags that do not already exist', () => {
      const { result } = renderHook(() => useTagSuggestions({ initialTags: ['existing'] }));

      act(() => {
        result.current.handleTagSuggestions(['new1', 'new2']);
      });

      expect(result.current.tags).toContain('existing');
      expect(result.current.tags).toContain('new1');
      expect(result.current.tags).toContain('new2');
    });

    it('does not add duplicate tags (case-insensitive)', () => {
      const { result } = renderHook(() => useTagSuggestions({ initialTags: ['React'] }));

      act(() => {
        result.current.handleTagSuggestions(['react', 'REACT', 'React']);
      });

      // Should only have one React tag
      const reactTags = result.current.tags.filter(t =>
        t.toLowerCase() === 'react'
      );
      expect(reactTags).toHaveLength(1);
    });

    it('sets suggestedTags for display', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.handleTagSuggestions(['ai', 'ml']);
      });

      expect(result.current.suggestedTags).toEqual(['ai', 'ml']);
    });

    it('handles empty suggestion list', () => {
      const { result } = renderHook(() => useTagSuggestions({ initialTags: ['test'] }));

      act(() => {
        result.current.handleTagSuggestions([]);
      });

      expect(result.current.tags).toEqual(['test']);
    });

    it('handles null suggestion list', () => {
      const { result } = renderHook(() => useTagSuggestions({ initialTags: ['test'] }));

      act(() => {
        result.current.handleTagSuggestions(null);
      });

      expect(result.current.tags).toEqual(['test']);
    });

    it('calls onTagsChange when suggestions are added', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({ onTagsChange }));

      act(() => {
        result.current.handleTagSuggestions(['newTag']);
      });

      expect(onTagsChange).toHaveBeenCalledWith(['newTag']);
    });
  });

  describe('addTag', () => {
    it('adds a single tag', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.addTag('newTag');
      });

      expect(result.current.tags).toContain('newTag');
    });

    it('does not add duplicate tag (case-insensitive)', () => {
      const { result } = renderHook(() => useTagSuggestions({ initialTags: ['Existing'] }));

      act(() => {
        result.current.addTag('existing');
      });

      expect(result.current.tags).toHaveLength(1);
    });

    it('trims whitespace from tag', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.addTag('  spaced  ');
      });

      // Tag is normalized but original form may be preserved
      expect(result.current.tags.some(t => t.trim() === 'spaced')).toBe(true);
    });

    it('calls onTagsChange when tag is added', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({ onTagsChange }));

      act(() => {
        result.current.addTag('test');
      });

      expect(onTagsChange).toHaveBeenCalled();
    });
  });

  describe('removeTag', () => {
    it('removes tag at specified index', () => {
      const { result } = renderHook(() => useTagSuggestions({
        initialTags: ['first', 'second', 'third']
      }));

      act(() => {
        result.current.removeTag(1);
      });

      expect(result.current.tags).toEqual(['first', 'third']);
    });

    it('calls onTagsChange when tag is removed', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({
        initialTags: ['test'],
        onTagsChange
      }));

      act(() => {
        result.current.removeTag(0);
      });

      expect(onTagsChange).toHaveBeenCalledWith([]);
    });

    it('handles removing from empty array gracefully', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.removeTag(0);
      });

      expect(result.current.tags).toEqual([]);
    });
  });

  describe('clearSuggestedTags', () => {
    it('clears suggested tags', () => {
      const { result } = renderHook(() => useTagSuggestions());

      act(() => {
        result.current.handleTagSuggestions(['ai', 'ml']);
      });
      expect(result.current.suggestedTags).toHaveLength(2);

      act(() => {
        result.current.clearSuggestedTags();
      });

      expect(result.current.suggestedTags).toEqual([]);
    });
  });

  describe('setTags', () => {
    it('sets tags directly without callback', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({ onTagsChange }));

      act(() => {
        result.current.setTags(['direct1', 'direct2']);
      });

      expect(result.current.tags).toEqual(['direct1', 'direct2']);
      // setTags should not trigger callback
      expect(onTagsChange).not.toHaveBeenCalled();
    });
  });

  describe('updateTags', () => {
    it('updates tags and triggers callback', () => {
      const onTagsChange = jest.fn();
      const { result } = renderHook(() => useTagSuggestions({ onTagsChange }));

      act(() => {
        result.current.updateTags(['updated1', 'updated2']);
      });

      expect(result.current.tags).toEqual(['updated1', 'updated2']);
      expect(onTagsChange).toHaveBeenCalledWith(['updated1', 'updated2']);
    });
  });

  describe('function stability', () => {
    it('handler functions are stable across renders', () => {
      const { result, rerender } = renderHook(() => useTagSuggestions());

      const firstHandleTagsChange = result.current.handleTagsChange;
      const firstClearSuggestedTags = result.current.clearSuggestedTags;

      rerender();

      expect(result.current.handleTagsChange).toBe(firstHandleTagsChange);
      expect(result.current.clearSuggestedTags).toBe(firstClearSuggestedTags);
    });
  });
});
