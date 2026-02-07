import { useState, useCallback } from 'react';

/**
 * Custom hook for handling async operations with loading and error states
 *
 * @example
 * const { execute, loading, error, data } = useAsync(fetchDocuments);
 *
 * useEffect(() => {
 *   execute();
 * }, [execute]);
 */
const useAsync = (asyncFunction, immediate = false) => {
  const [loading, setLoading] = useState(immediate);
  const [error, setError] = useState(null);
  const [data, setData] = useState(null);

  const execute = useCallback(async (...args) => {
    setLoading(true);
    setError(null);

    try {
      const result = await asyncFunction(...args);
      setData(result);
      return result;
    } catch (err) {
      setError(err);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [asyncFunction]);

  const reset = useCallback(() => {
    setLoading(false);
    setError(null);
    setData(null);
  }, []);

  return {
    execute,
    loading,
    error,
    data,
    reset,
    setData,
    setError
  };
};

export default useAsync;
