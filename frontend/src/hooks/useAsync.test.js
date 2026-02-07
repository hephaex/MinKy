import { renderHook, act, waitFor } from '@testing-library/react';
import useAsync from './useAsync';

describe('useAsync', () => {
  describe('initial state', () => {
    it('starts with loading false when immediate is false', () => {
      const asyncFn = jest.fn();
      const { result } = renderHook(() => useAsync(asyncFn, false));

      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.data).toBeNull();
    });

    it('starts with loading true when immediate is true', () => {
      const asyncFn = jest.fn();
      const { result } = renderHook(() => useAsync(asyncFn, true));

      expect(result.current.loading).toBe(true);
    });
  });

  describe('execute', () => {
    it('sets loading to true when execute is called', async () => {
      const asyncFn = jest.fn().mockImplementation(() => new Promise(() => {}));
      const { result } = renderHook(() => useAsync(asyncFn));

      act(() => {
        result.current.execute();
      });

      expect(result.current.loading).toBe(true);
    });

    it('sets data on successful execution', async () => {
      const mockData = { id: 1, name: 'Test' };
      const asyncFn = jest.fn().mockResolvedValue(mockData);
      const { result } = renderHook(() => useAsync(asyncFn));

      await act(async () => {
        await result.current.execute();
      });

      expect(result.current.data).toEqual(mockData);
      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it('passes arguments to async function', async () => {
      const asyncFn = jest.fn().mockResolvedValue('result');
      const { result } = renderHook(() => useAsync(asyncFn));

      await act(async () => {
        await result.current.execute('arg1', 'arg2', 123);
      });

      expect(asyncFn).toHaveBeenCalledWith('arg1', 'arg2', 123);
    });

    it('sets error on failed execution', async () => {
      const mockError = new Error('Test error');
      const asyncFn = jest.fn().mockRejectedValue(mockError);
      const { result } = renderHook(() => useAsync(asyncFn));

      await act(async () => {
        try {
          await result.current.execute();
        } catch (e) {
          // Expected to throw
        }
      });

      expect(result.current.error).toEqual(mockError);
      expect(result.current.loading).toBe(false);
      expect(result.current.data).toBeNull();
    });

    it('clears previous error on new execution', async () => {
      const asyncFn = jest.fn()
        .mockRejectedValueOnce(new Error('First error'))
        .mockResolvedValueOnce('success');
      const { result } = renderHook(() => useAsync(asyncFn));

      // First call - should fail
      await act(async () => {
        try {
          await result.current.execute();
        } catch (e) {}
      });
      expect(result.current.error).not.toBeNull();

      // Second call - should succeed and clear error
      await act(async () => {
        await result.current.execute();
      });
      expect(result.current.error).toBeNull();
      expect(result.current.data).toBe('success');
    });

    it('returns the result from async function', async () => {
      const mockData = { success: true };
      const asyncFn = jest.fn().mockResolvedValue(mockData);
      const { result } = renderHook(() => useAsync(asyncFn));

      let returnValue;
      await act(async () => {
        returnValue = await result.current.execute();
      });

      expect(returnValue).toEqual(mockData);
    });
  });

  describe('reset', () => {
    it('resets all state to initial values', async () => {
      const asyncFn = jest.fn().mockResolvedValue({ data: 'test' });
      const { result } = renderHook(() => useAsync(asyncFn));

      // Execute to populate state
      await act(async () => {
        await result.current.execute();
      });
      expect(result.current.data).not.toBeNull();

      // Reset
      act(() => {
        result.current.reset();
      });

      expect(result.current.loading).toBe(false);
      expect(result.current.error).toBeNull();
      expect(result.current.data).toBeNull();
    });
  });

  describe('setData', () => {
    it('allows manual data setting', () => {
      const asyncFn = jest.fn();
      const { result } = renderHook(() => useAsync(asyncFn));

      act(() => {
        result.current.setData({ custom: 'data' });
      });

      expect(result.current.data).toEqual({ custom: 'data' });
    });
  });

  describe('setError', () => {
    it('allows manual error setting', () => {
      const asyncFn = jest.fn();
      const { result } = renderHook(() => useAsync(asyncFn));

      const customError = new Error('Manual error');
      act(() => {
        result.current.setError(customError);
      });

      expect(result.current.error).toEqual(customError);
    });
  });

  describe('function stability', () => {
    it('execute function is stable across renders', () => {
      const asyncFn = jest.fn().mockResolvedValue('test');
      const { result, rerender } = renderHook(() => useAsync(asyncFn));

      const firstExecute = result.current.execute;
      rerender();
      const secondExecute = result.current.execute;

      expect(firstExecute).toBe(secondExecute);
    });

    it('reset function is stable across renders', () => {
      const asyncFn = jest.fn();
      const { result, rerender } = renderHook(() => useAsync(asyncFn));

      const firstReset = result.current.reset;
      rerender();
      const secondReset = result.current.reset;

      expect(firstReset).toBe(secondReset);
    });
  });
});
