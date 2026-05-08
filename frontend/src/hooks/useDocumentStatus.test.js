import { renderHook, act } from '@testing-library/react';
import useDocumentStatus from './useDocumentStatus';
import { documentService } from '../services/api';
import { logError } from '../utils/logger';

jest.mock('../services/api', () => ({
  documentService: {
    getDocumentStatus: jest.fn(),
  },
}));

jest.mock('../utils/logger', () => ({
  logError: jest.fn(),
}));

const POLL_INTERVAL = 5000;

const makePendingResponse = (overrides = {}) => ({
  processing_status: 'pending',
  queue_position: 3,
  error_message: null,
  ...overrides,
});

describe('useDocumentStatus', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  describe('initial state', () => {
    it('returns null/false for all fields before polling starts', () => {
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      expect(result.current.status).toBeNull();
      expect(result.current.queuePosition).toBeNull();
      expect(result.current.errorMessage).toBeNull();
      expect(result.current.isPolling).toBe(false);
    });

    it('exposes startPolling and stopPolling as functions', () => {
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      expect(typeof result.current.startPolling).toBe('function');
      expect(typeof result.current.stopPolling).toBe('function');
    });
  });

  describe('startPolling', () => {
    it('sets isPolling to true immediately', () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      act(() => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(true);
    });

    it('calls fetchStatus immediately on start', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(1);
      expect(documentService.getDocumentStatus).toHaveBeenCalledWith('doc-1');
    });

    it('calls fetchStatus again after each interval tick', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL);
      });

      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(2);

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL);
      });

      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(3);
    });

    it('does not create duplicate intervals when called again while already polling', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      await act(async () => {
        result.current.startPolling();
      });

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL);
      });

      // Only 2 calls: initial + one interval tick (not 3 from a duplicate interval)
      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(2);
    });
  });

  describe('stopPolling', () => {
    it('sets isPolling to false', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      act(() => {
        result.current.stopPolling();
      });

      expect(result.current.isPolling).toBe(false);
    });

    it('stops the interval so no further fetches occur', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      act(() => {
        result.current.stopPolling();
      });

      const callsAfterStop = documentService.getDocumentStatus.mock.calls.length;

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL * 3);
      });

      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(callsAfterStop);
    });
  });

  describe('auto-stop on non-pending status', () => {
    it('stops polling when API returns completed status', async () => {
      documentService.getDocumentStatus.mockResolvedValue({
        processing_status: 'completed',
        queue_position: null,
        error_message: null,
      });
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(false);
      expect(result.current.status).toBe('completed');
    });

    it('stops polling when API returns failed status', async () => {
      documentService.getDocumentStatus.mockResolvedValue({
        processing_status: 'failed',
        queue_position: null,
        error_message: 'Processing failed',
      });
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(false);
      expect(result.current.status).toBe('failed');
      expect(result.current.errorMessage).toBe('Processing failed');
    });

    it('stops polling when status is processing', async () => {
      documentService.getDocumentStatus.mockResolvedValueOnce({
        processing_status: 'processing',
        queue_position: null,
        error_message: null,
      });
      const { result } = renderHook(() => useDocumentStatus(123));
      await act(async () => {
        result.current.startPolling();
      });
      expect(result.current.status).toBe('processing');
      expect(result.current.isPolling).toBe(false);
    });

    it('continues polling while status remains pending', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(true);

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL);
      });

      expect(result.current.isPolling).toBe(true);
      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(2);
    });
  });

  describe('state updates from API response', () => {
    it('updates status, queuePosition and errorMessage from API data', async () => {
      documentService.getDocumentStatus.mockResolvedValue(
        makePendingResponse({ queue_position: 7, error_message: null })
      );
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.status).toBe('pending');
      expect(result.current.queuePosition).toBe(7);
      expect(result.current.errorMessage).toBeNull();
    });

    it('maps missing queue_position to null', async () => {
      documentService.getDocumentStatus.mockResolvedValue({
        processing_status: 'completed',
      });
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.queuePosition).toBeNull();
      expect(result.current.errorMessage).toBeNull();
    });
  });

  describe('error handling', () => {
    it('stops polling gracefully when API call throws', async () => {
      documentService.getDocumentStatus.mockRejectedValue(new Error('Network error'));
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(false);
    });

    it('calls logError with context and error on API failure', async () => {
      const err = new Error('Network error');
      documentService.getDocumentStatus.mockRejectedValue(err);
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      expect(logError).toHaveBeenCalledWith('useDocumentStatus.fetchStatus', err);
    });

    it('does not throw to callers on API failure', async () => {
      documentService.getDocumentStatus.mockRejectedValue(new Error('boom'));
      const { result } = renderHook(() => useDocumentStatus('doc-1'));

      await expect(
        act(async () => {
          result.current.startPolling();
        })
      ).resolves.not.toThrow();
    });
  });

  describe('no documentId guard', () => {
    it('does not call the API when documentId is null', async () => {
      const { result } = renderHook(() => useDocumentStatus(null));

      await act(async () => {
        result.current.startPolling();
      });

      expect(documentService.getDocumentStatus).not.toHaveBeenCalled();
    });

    it('does not call the API when documentId is undefined', async () => {
      const { result } = renderHook(() => useDocumentStatus(undefined));

      await act(async () => {
        result.current.startPolling();
      });

      expect(documentService.getDocumentStatus).not.toHaveBeenCalled();
    });

    it('does not call the API when documentId is empty string', async () => {
      const { result } = renderHook(() => useDocumentStatus(''));
      await act(async () => {
        result.current.startPolling();
      });
      expect(documentService.getDocumentStatus).not.toHaveBeenCalled();
    });
  });

  describe('unmount cleanup', () => {
    it('clears interval on unmount so no further fetches occur', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result, unmount } = renderHook(() => useDocumentStatus('doc-1'));

      await act(async () => {
        result.current.startPolling();
      });

      const callsBeforeUnmount = documentService.getDocumentStatus.mock.calls.length;
      unmount();

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL * 3);
      });

      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(callsBeforeUnmount);
    });

    it('does not update state after unmount (mountedRef guard)', async () => {
      let resolveStatus;
      documentService.getDocumentStatus.mockImplementation(
        () => new Promise((resolve) => { resolveStatus = resolve; })
      );

      const { result, unmount } = renderHook(() => useDocumentStatus('doc-1'));

      act(() => {
        result.current.startPolling();
      });

      // Unmount before the in-flight promise resolves
      unmount();

      // Now resolve — should not trigger any state setter
      await act(async () => {
        resolveStatus(makePendingResponse({ processing_status: 'completed' }));
      });

      // No assertion on result.current after unmount (would be stale);
      // the test passes if no "Can't perform a React state update on an unmounted component" error appears.
    });
  });

  describe('documentId change', () => {
    it('stops any active polling when documentId changes', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result, rerender } = renderHook(
        ({ id }) => useDocumentStatus(id),
        { initialProps: { id: 'doc-1' } }
      );

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(true);

      await act(async () => {
        rerender({ id: 'doc-2' });
      });

      expect(result.current.isPolling).toBe(false);
    });

    it('does not carry over stale interval after documentId changes', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result, rerender } = renderHook(
        ({ id }) => useDocumentStatus(id),
        { initialProps: { id: 'doc-1' } }
      );

      await act(async () => {
        result.current.startPolling();
      });

      await act(async () => {
        rerender({ id: 'doc-2' });
      });

      const callsAfterChange = documentService.getDocumentStatus.mock.calls.length;

      await act(async () => {
        jest.advanceTimersByTime(POLL_INTERVAL * 3);
      });

      // No extra calls from the old interval after the id changed
      expect(documentService.getDocumentStatus).toHaveBeenCalledTimes(callsAfterChange);
    });

    it('allows fresh polling to start after documentId changes', async () => {
      documentService.getDocumentStatus.mockResolvedValue(makePendingResponse());
      const { result, rerender } = renderHook(
        ({ id }) => useDocumentStatus(id),
        { initialProps: { id: 'doc-1' } }
      );

      await act(async () => {
        result.current.startPolling();
      });

      await act(async () => {
        rerender({ id: 'doc-2' });
      });

      await act(async () => {
        result.current.startPolling();
      });

      expect(result.current.isPolling).toBe(true);
      expect(documentService.getDocumentStatus).toHaveBeenLastCalledWith('doc-2');
    });
  });
});
