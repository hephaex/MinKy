import { useState, useEffect, useRef, useCallback } from 'react';
import { documentService } from '../services/api';
import { logError } from '../utils/logger';

const POLL_INTERVAL = 5000;

const useDocumentStatus = (documentId) => {
  const [status, setStatus] = useState(null);
  const [queuePosition, setQueuePosition] = useState(null);
  const [errorMessage, setErrorMessage] = useState(null);
  const [isPolling, setIsPolling] = useState(false);
  const intervalRef = useRef(null);
  const mountedRef = useRef(true);

  const stopPolling = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
    setIsPolling(false);
  }, []);

  const fetchStatus = useCallback(async () => {
    if (!documentId || !mountedRef.current) return;
    try {
      const data = await documentService.getDocumentStatus(documentId);
      if (!mountedRef.current) return;

      setStatus(data.processing_status);
      setQueuePosition(data.queue_position ?? null);
      setErrorMessage(data.error_message ?? null);

      if (data.processing_status !== 'pending') {
        stopPolling();
      }
    } catch (err) {
      logError('useDocumentStatus.fetchStatus', err);
      if (mountedRef.current) {
        stopPolling();
      }
    }
  }, [documentId, stopPolling]);

  const startPolling = useCallback(() => {
    if (intervalRef.current) return;
    setIsPolling(true);
    fetchStatus();
    intervalRef.current = setInterval(fetchStatus, POLL_INTERVAL);
  }, [fetchStatus]);

  useEffect(() => {
    stopPolling();
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      stopPolling();
    };
  }, [documentId, stopPolling]);

  return { status, queuePosition, errorMessage, isPolling, startPolling, stopPolling };
};

export default useDocumentStatus;
