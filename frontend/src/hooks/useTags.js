import { useState, useEffect, useCallback } from 'react';
import api from '../services/api';

/**
 * Custom hook for fetching and managing tags
 * Provides a list of tags for filtering and selection
 *
 * @param {Object} options - Hook options
 * @param {boolean} options.fetchOnMount - Whether to fetch tags on mount (default: true)
 * @param {boolean} options.popular - Whether to fetch popular tags only (default: false)
 * @returns {Object} Tags state and actions
 *
 * @example
 * const { tags, loading, error, refetch } = useTags();
 */
const useTags = (options = {}) => {
  const { fetchOnMount = true, popular = false } = options;

  const [tags, setTags] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const fetchTags = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const params = new URLSearchParams();
      params.append('per_page', '100'); // Fetch more tags for filtering
      if (popular) {
        params.append('popular', 'true');
      }
      const response = await api.get(`/tags?${params.toString()}`);
      const fetchedTags = response.data?.tags || response.data?.data?.tags || [];
      setTags(fetchedTags);
      return fetchedTags;
    } catch (err) {
      setError(err.message || 'Failed to fetch tags');
      setTags([]);
      return [];
    } finally {
      setLoading(false);
    }
  }, [popular]);

  useEffect(() => {
    if (fetchOnMount) {
      fetchTags();
    }
  }, [fetchOnMount, fetchTags]);

  return {
    tags,
    loading,
    error,
    refetch: fetchTags,
    setTags,
  };
};

export default useTags;
