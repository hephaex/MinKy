import { useState, useCallback } from 'react';

/**
 * Custom hook for handling AI tag suggestions.
 * Eliminates duplicated tag handling logic in DocumentCreate and DocumentEdit.
 *
 * @param {Object} options - Hook options
 * @param {Array} options.initialTags - Initial tags array
 * @param {Function} options.onTagsChange - Callback when tags change (tags) => void
 * @returns {Object} Hook state and handlers
 */
const useTagSuggestions = ({ initialTags = [], onTagsChange } = {}) => {
  const [suggestedTags, setSuggestedTags] = useState([]);
  const [tags, setTags] = useState(initialTags);

  /**
   * Update tags and notify parent component
   */
  const updateTags = useCallback((newTags) => {
    setTags(newTags);
    onTagsChange?.(newTags);
  }, [onTagsChange]);

  /**
   * Handle AI-suggested tags by merging with existing tags.
   * Auto-applies suggested tags that don't already exist.
   */
  const handleTagSuggestions = useCallback((suggestedTagsList) => {
    console.log('Suggested tags:', suggestedTagsList);

    if (!suggestedTagsList || suggestedTagsList.length === 0) {
      return;
    }

    setTags(currentTags => {
      const newTags = [...currentTags];

      // Add suggested tags that aren't already present
      suggestedTagsList.forEach(suggestedTag => {
        const normalizedSuggested = suggestedTag.toLowerCase().trim();
        const exists = newTags.some(existingTag =>
          existingTag.toLowerCase().trim() === normalizedSuggested
        );

        if (!exists) {
          newTags.push(suggestedTag);
        }
      });

      // Notify parent of change
      onTagsChange?.(newTags);
      return newTags;
    });

    // Set suggested tags for display (user can still see what was added)
    setSuggestedTags(suggestedTagsList);
  }, [onTagsChange]);

  /**
   * Handle direct tag changes (from TagInput component)
   */
  const handleTagsChange = useCallback((newTags) => {
    setTags(newTags);
    onTagsChange?.(newTags);
  }, [onTagsChange]);

  /**
   * Clear suggested tags display
   */
  const clearSuggestedTags = useCallback(() => {
    setSuggestedTags([]);
  }, []);

  /**
   * Add a single tag
   */
  const addTag = useCallback((tag) => {
    const normalizedTag = tag.toLowerCase().trim();

    setTags(currentTags => {
      const exists = currentTags.some(existingTag =>
        existingTag.toLowerCase().trim() === normalizedTag
      );

      if (exists) {
        return currentTags;
      }

      const newTags = [...currentTags, tag];
      onTagsChange?.(newTags);
      return newTags;
    });
  }, [onTagsChange]);

  /**
   * Remove a tag by index
   */
  const removeTag = useCallback((index) => {
    setTags(currentTags => {
      const newTags = currentTags.filter((_, i) => i !== index);
      onTagsChange?.(newTags);
      return newTags;
    });
  }, [onTagsChange]);

  /**
   * Set tags directly (useful for initialization)
   */
  const setTagsDirectly = useCallback((newTags) => {
    setTags(newTags);
  }, []);

  return {
    // State
    tags,
    suggestedTags,

    // Handlers
    handleTagSuggestions,
    handleTagsChange,
    clearSuggestedTags,
    addTag,
    removeTag,
    setTags: setTagsDirectly,
    updateTags,
  };
};

export default useTagSuggestions;
