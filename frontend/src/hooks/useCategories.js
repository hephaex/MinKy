import { useState, useEffect, useCallback } from 'react';
import api from '../services/api';

/**
 * Custom hook for fetching and managing categories
 * Eliminates duplicate fetchCategories code across components
 *
 * @param {Object} options - Hook options
 * @param {boolean} options.fetchOnMount - Whether to fetch categories on mount (default: true)
 * @param {boolean} options.includeInactive - Whether to include inactive categories (default: false)
 * @returns {Object} Categories state and actions
 *
 * @example
 * const { categories, loading, error, refetch } = useCategories();
 */
const useCategories = (options = {}) => {
  const { fetchOnMount = true, includeInactive = false } = options;

  const [categories, setCategories] = useState([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(null);

  const fetchCategories = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const params = new URLSearchParams({ format: 'flat' });
      if (includeInactive) {
        params.append('include_inactive', 'true');
      }
      const response = await api.get(`/categories/?${params.toString()}`);
      const fetchedCategories = response.data.data?.categories || [];
      setCategories(fetchedCategories);
      return fetchedCategories;
    } catch (err) {
      setError(err.message || 'Failed to fetch categories');
      setCategories([]);
      return [];
    } finally {
      setLoading(false);
    }
  }, [includeInactive]);

  useEffect(() => {
    if (fetchOnMount) {
      fetchCategories();
    }
  }, [fetchOnMount, fetchCategories]);

  return {
    categories,
    loading,
    error,
    refetch: fetchCategories,
    setCategories
  };
};

export default useCategories;
