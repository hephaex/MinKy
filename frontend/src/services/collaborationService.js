import { io } from 'socket.io-client';
import { logError, logWarning } from '../utils/logger';

const SOCKET_URL = process.env.REACT_APP_SOCKET_URL || 'http://localhost:8000';

// Helper to get auth token
const getAuthToken = () => localStorage.getItem('token');

class CollaborationService {
  constructor() {
    this.socket = null;
    this.isConnected = false;
    this.currentDocument = null;
    this.eventHandlers = {};
    this.userCursors = new Map();
    this.operationQueue = [];
    this.isApplyingOperation = false;
  }

  connect() {
    if (this.socket && this.isConnected) {
      return;
    }

    // SECURITY: Include auth token in WebSocket connection
    const token = getAuthToken();
    this.socket = io(SOCKET_URL, {
      transports: ['websocket'],
      autoConnect: true,
      auth: {
        token: token
      }
    });

    this.socket.on('connect', () => {
      // connected
      this.isConnected = true;
      this.emit('connected');
    });

    this.socket.on('disconnect', () => {
      // disconnected
      this.isConnected = false;
      this.emit('disconnected');
    });

    this.socket.on('error', (error) => {
      logError('CollaborationService', error);
      this.emit('error', error);
    });

    // Document collaboration events
    this.socket.on('document_joined', (data) => {
      // joined document
      this.emit('document_joined', data);
    });

    this.socket.on('user_joined', (data) => {
      // user joined
      this.emit('user_joined', data);
    });

    this.socket.on('user_left', (data) => {
      // user left
      this.emit('user_left', data);
    });

    this.socket.on('text_operation', (data) => {
      this.handleRemoteOperation(data);
    });

    this.socket.on('cursor_update', (data) => {
      this.handleCursorUpdate(data);
    });

    this.socket.on('document_saved', (data) => {
      // document saved
      this.emit('document_saved', data);
    });

    this.socket.on('save_success', (data) => {
      this.emit('save_success', data);
    });

    this.socket.on('session_info', (data) => {
      this.emit('session_info', data);
    });
  }

  disconnect() {
    if (this.socket) {
      this.socket.disconnect();
      this.socket = null;
      this.isConnected = false;
      this.currentDocument = null;
    }
  }

  joinDocument(documentId) {
    if (!this.isConnected) {
      logWarning('CollaborationService', 'Not connected to collaboration server');
      return;
    }

    this.currentDocument = documentId;
    this.socket.emit('join_document', {
      document_id: documentId
    });
  }

  leaveDocument(documentId) {
    if (!this.isConnected || !documentId) {
      return;
    }

    this.socket.emit('leave_document', {
      document_id: documentId
    });

    if (this.currentDocument === documentId) {
      this.currentDocument = null;
    }
  }

  sendTextOperation(documentId, operation) {
    if (!this.isConnected || !documentId) {
      return;
    }

    this.socket.emit('text_operation', {
      document_id: documentId,
      operation: operation
    });
  }

  sendCursorUpdate(documentId, cursorData) {
    if (!this.isConnected || !documentId) {
      return;
    }

    this.socket.emit('cursor_update', {
      document_id: documentId,
      cursor_data: cursorData
    });
  }

  saveDocument(documentId) {
    if (!this.isConnected || !documentId) {
      return;
    }

    this.socket.emit('save_document', {
      document_id: documentId
    });
  }

  getSessionInfo(documentId) {
    if (!this.isConnected || !documentId) {
      return;
    }

    this.socket.emit('get_session_info', {
      document_id: documentId
    });
  }

  handleRemoteOperation(data) {
    if (this.isApplyingOperation) {
      // Queue operation if we're currently applying one
      this.operationQueue.push(data);
      return;
    }

    this.isApplyingOperation = true;
    this.emit('remote_operation', data);
    
    // Process queued operations
    setTimeout(() => {
      this.isApplyingOperation = false;
      if (this.operationQueue.length > 0) {
        const nextOperation = this.operationQueue.shift();
        this.handleRemoteOperation(nextOperation);
      }
    }, 0);
  }

  handleCursorUpdate(data) {
    this.userCursors.set(data.user_id, {
      username: data.username,
      cursor_data: data.cursor_data,
      timestamp: Date.now()
    });

    this.emit('cursor_update', data);
  }

  // Operational Transform utilities
  createInsertOperation(position, text) {
    return {
      type: 'insert',
      position: position,
      text: text
    };
  }

  createDeleteOperation(position, length) {
    return {
      type: 'delete',
      position: position,
      length: length
    };
  }

  createReplaceOperation(position, length, text) {
    return {
      type: 'replace',
      position: position,
      length: length,
      text: text
    };
  }

  applyOperation(content, operation) {
    try {
      const { type, position } = operation;

      if (position < 0 || position > content.length) {
        logWarning('CollaborationService', `Invalid operation position: ${position}`);
        return content;
      }

      switch (type) {
        case 'insert':
          return content.substring(0, position) + 
                 operation.text + 
                 content.substring(position);

        case 'delete':
          const endPos = Math.min(position + operation.length, content.length);
          return content.substring(0, position) + 
                 content.substring(endPos);

        case 'replace':
          const replaceEndPos = Math.min(position + operation.length, content.length);
          return content.substring(0, position) + 
                 operation.text + 
                 content.substring(replaceEndPos);

        default:
          logWarning('CollaborationService', `Unknown operation type: ${type}`);
          return content;
      }
    } catch (error) {
      logError('CollaborationService.applyOperation', error);
      return content;
    }
  }

  // Transform operation against another operation (basic OT)
  transformOperation(op1, op2, isOwnOperation = false) {
    if (op1.type === 'insert' && op2.type === 'insert') {
      if (op1.position <= op2.position) {
        return {
          ...op2,
          position: op2.position + op1.text.length
        };
      }
      return op2;
    }

    if (op1.type === 'delete' && op2.type === 'insert') {
      if (op1.position < op2.position) {
        return {
          ...op2,
          position: Math.max(op1.position, op2.position - op1.length)
        };
      }
      return op2;
    }

    if (op1.type === 'insert' && op2.type === 'delete') {
      if (op1.position <= op2.position) {
        return {
          ...op2,
          position: op2.position + op1.text.length
        };
      }
      return op2;
    }

    if (op1.type === 'delete' && op2.type === 'delete') {
      if (op1.position < op2.position) {
        return {
          ...op2,
          position: Math.max(op1.position, op2.position - op1.length),
          length: op2.length
        };
      } else if (op1.position >= op2.position + op2.length) {
        return op2;
      } else {
        // Overlapping deletes - more complex case
        return null; // Skip conflicting operation
      }
    }

    return op2;
  }

  // Event handling
  on(event, handler) {
    if (!this.eventHandlers[event]) {
      this.eventHandlers[event] = [];
    }
    this.eventHandlers[event].push(handler);
  }

  off(event, handler) {
    if (!this.eventHandlers[event]) {
      return;
    }
    const index = this.eventHandlers[event].indexOf(handler);
    if (index > -1) {
      this.eventHandlers[event].splice(index, 1);
    }
  }

  emit(event, data) {
    if (!this.eventHandlers[event]) {
      return;
    }
    this.eventHandlers[event].forEach(handler => {
      try {
        handler(data);
      } catch (error) {
        logError('CollaborationService.emit', error);
      }
    });
  }

  getUserCursors() {
    return Array.from(this.userCursors.entries()).map(([userId, data]) => ({
      userId,
      ...data
    }));
  }

  cleanupOldCursors() {
    const now = Date.now();
    const CURSOR_TIMEOUT = 30000; // 30 seconds

    for (const [userId, data] of this.userCursors.entries()) {
      if (now - data.timestamp > CURSOR_TIMEOUT) {
        this.userCursors.delete(userId);
      }
    }
  }
}

// Export singleton instance
export const collaborationService = new CollaborationService();