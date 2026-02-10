import React, { useState, useEffect, useRef, useCallback } from 'react';
import MDEditor from '@uiw/react-md-editor';
import { collaborationService } from '../services/collaborationService';
import AISuggestions from './AISuggestions';
import { logError } from '../utils/logger';
import './CollaborativeEditor.css';

const CollaborativeEditor = ({
  documentId,
  initialValue = '',
  onChange,
  onTitleSuggestion,
  onTagSuggestions,
  placeholder = "Start writing your markdown...",
  showAISuggestions = true
}) => {
  const [value, setValue] = useState(initialValue);
  const [isCollaborating, setIsCollaborating] = useState(false);
  const [activeUsers, setActiveUsers] = useState([]);
  const [userCursors, setUserCursors] = useState([]);
  const [connectionStatus, setConnectionStatus] = useState('disconnected');
  const [previewMode, setPreviewMode] = useState('edit');
  const [cursorPosition, setCursorPosition] = useState(0);
  
  const editorRef = useRef(null);
  const lastValueRef = useRef(initialValue);
  const pendingOperationsRef = useRef([]);
  const isApplyingRemoteOperation = useRef(false);

  // Initialize collaboration
  useEffect(() => {
    if (!documentId) return;

    // Connect to collaboration service
    collaborationService.connect();

    // Set up event handlers
    const handleConnected = () => {
      setConnectionStatus('connected');
      collaborationService.joinDocument(documentId);
    };

    const handleDisconnected = () => {
      setConnectionStatus('disconnected');
      setIsCollaborating(false);
      setActiveUsers([]);
    };

    const handleDocumentJoined = (data) => {
      setIsCollaborating(true);
      setActiveUsers(data.active_users || []);
      
      // Update content if different from server
      if (data.content !== value) {
        setValue(data.content);
        lastValueRef.current = data.content;
        if (onChange) {
          onChange(data.content);
        }
      }
    };

    const handleUserJoined = (data) => {
      setActiveUsers(prev => {
        const exists = prev.find(u => u.user_id === data.user_id);
        if (!exists) {
          return [...prev, data];
        }
        return prev;
      });
    };

    const handleUserLeft = (data) => {
      setActiveUsers(prev => prev.filter(u => u.user_id !== data.user_id));
      setUserCursors(prev => prev.filter(c => c.userId !== data.user_id));
    };

    const handleRemoteOperation = (data) => {
      if (isApplyingRemoteOperation.current) {
        pendingOperationsRef.current.push(data);
        return;
      }

      applyRemoteOperation(data);
    };

    const handleCursorUpdate = (data) => {
      setUserCursors(prev => {
        const filtered = prev.filter(c => c.userId !== data.user_id);
        return [...filtered, {
          userId: data.user_id,
          username: data.username,
          position: data.cursor_data.position,
          selectionStart: data.cursor_data.selection_start,
          selectionEnd: data.cursor_data.selection_end
        }];
      });
    };

    const handleDocumentSaved = (data) => {
      // document auto-saved
    };

    const handleError = (error) => {
      logError('CollaborativeEditor', error);
      setConnectionStatus('error');
    };

    // Register event handlers
    collaborationService.on('connected', handleConnected);
    collaborationService.on('disconnected', handleDisconnected);
    collaborationService.on('document_joined', handleDocumentJoined);
    collaborationService.on('user_joined', handleUserJoined);
    collaborationService.on('user_left', handleUserLeft);
    collaborationService.on('remote_operation', handleRemoteOperation);
    collaborationService.on('cursor_update', handleCursorUpdate);
    collaborationService.on('document_saved', handleDocumentSaved);
    collaborationService.on('error', handleError);

    // Cleanup on unmount
    return () => {
      collaborationService.off('connected', handleConnected);
      collaborationService.off('disconnected', handleDisconnected);
      collaborationService.off('document_joined', handleDocumentJoined);
      collaborationService.off('user_joined', handleUserJoined);
      collaborationService.off('user_left', handleUserLeft);
      collaborationService.off('remote_operation', handleRemoteOperation);
      collaborationService.off('cursor_update', handleCursorUpdate);
      collaborationService.off('document_saved', handleDocumentSaved);
      collaborationService.off('error', handleError);
      
      collaborationService.leaveDocument(documentId);
    };
  }, [documentId]);

  // Handle local content changes
  const handleLocalChange = useCallback((newValue) => {
    if (isApplyingRemoteOperation.current) {
      return;
    }

    const oldValue = lastValueRef.current || '';
    
    if (newValue === oldValue) {
      return;
    }

    // Calculate operation
    const operation = calculateOperation(oldValue, newValue);
    
    if (operation && isCollaborating) {
      // Send operation to server
      collaborationService.sendTextOperation(documentId, operation);
    }

    // Update local state
    setValue(newValue);
    lastValueRef.current = newValue;
    
    if (onChange) {
      onChange(newValue);
    }
  }, [documentId, isCollaborating, onChange]);

  // Apply remote operation
  const applyRemoteOperation = useCallback((data) => {
    isApplyingRemoteOperation.current = true;
    
    try {
      const currentValue = lastValueRef.current || '';
      const newValue = collaborationService.applyOperation(currentValue, data.operation);
      
      setValue(newValue);
      lastValueRef.current = newValue;
      
      if (onChange) {
        onChange(newValue);
      }
    } catch (error) {
      logError('CollaborativeEditor.applyRemoteOperation', error);
    } finally {
      isApplyingRemoteOperation.current = false;
      
      // Process pending operations
      if (pendingOperationsRef.current.length > 0) {
        const nextOperation = pendingOperationsRef.current.shift();
        setTimeout(() => applyRemoteOperation(nextOperation), 0);
      }
    }
  }, [onChange]);

  // Calculate operation between old and new content
  const calculateOperation = (oldContent, newContent) => {
    if (oldContent === newContent) {
      return null;
    }

    // Simple diff algorithm - find first difference
    let i = 0;
    while (i < Math.min(oldContent.length, newContent.length) && 
           oldContent[i] === newContent[i]) {
      i++;
    }

    // Find last difference
    let j = 0;
    while (j < Math.min(oldContent.length - i, newContent.length - i) &&
           oldContent[oldContent.length - 1 - j] === newContent[newContent.length - 1 - j]) {
      j++;
    }

    const deleteLength = oldContent.length - i - j;
    const insertText = newContent.substring(i, newContent.length - j);

    if (deleteLength > 0 && insertText.length > 0) {
      // Replace operation
      return collaborationService.createReplaceOperation(i, deleteLength, insertText);
    } else if (deleteLength > 0) {
      // Delete operation
      return collaborationService.createDeleteOperation(i, deleteLength);
    } else if (insertText.length > 0) {
      // Insert operation
      return collaborationService.createInsertOperation(i, insertText);
    }

    return null;
  };

  // Handle cursor position updates
  const handleCursorChange = useCallback((event) => {
    if (!isCollaborating || !event.target) {
      return;
    }

    const position = event.target.selectionStart;
    const selectionStart = event.target.selectionStart;
    const selectionEnd = event.target.selectionEnd;

    setCursorPosition(position);

    // Throttle cursor updates
    const now = Date.now();
    if (!handleCursorChange.lastSent || now - handleCursorChange.lastSent > 500) {
      collaborationService.sendCursorUpdate(documentId, {
        position,
        selection_start: selectionStart,
        selection_end: selectionEnd
      });
      handleCursorChange.lastSent = now;
    }
  }, [documentId, isCollaborating]);

  // Handle save
  const handleSave = () => {
    if (isCollaborating) {
      collaborationService.saveDocument(documentId);
    }
  };

  const getConnectionStatusIcon = () => {
    switch (connectionStatus) {
      case 'connected':
        return isCollaborating ? '🟢' : '🟡';
      case 'disconnected':
        return '🔴';
      case 'error':
        return '⚠️';
      default:
        return '⚪';
    }
  };

  const getConnectionStatusText = () => {
    switch (connectionStatus) {
      case 'connected':
        return isCollaborating ? 'Collaborating' : 'Connected';
      case 'disconnected':
        return 'Offline';
      case 'error':
        return 'Error';
      default:
        return 'Connecting...';
    }
  };

  return (
    <div className="collaborative-editor">
      <div className="collaboration-header">
        <div className="collaboration-status">
          <span className="status-icon">{getConnectionStatusIcon()}</span>
          <span className="status-text">{getConnectionStatusText()}</span>
          {activeUsers.length > 0 && (
            <span className="active-users">
              {activeUsers.length} user{activeUsers.length !== 1 ? 's' : ''} online
            </span>
          )}
        </div>
        
        <div className="collaboration-actions">
          {isCollaborating && (
            <button className="save-btn" onClick={handleSave}>
              💾 Save
            </button>
          )}
        </div>
      </div>

      {activeUsers.length > 0 && (
        <div className="active-users-list">
          {activeUsers.map(user => (
            <div key={user.user_id} className="user-badge">
              {user.username || `User ${user.user_id}`}
            </div>
          ))}
        </div>
      )}

      <div className="editor-toolbar">
        <div className="editor-mode-tabs">
          <button
            className={`mode-tab ${previewMode === 'edit' ? 'active' : ''}`}
            onClick={() => setPreviewMode('edit')}
          >
            Edit
          </button>
          <button
            className={`mode-tab ${previewMode === 'preview' ? 'active' : ''}`}
            onClick={() => setPreviewMode('preview')}
          >
            Preview
          </button>
          <button
            className={`mode-tab ${previewMode === 'live' ? 'active' : ''}`}
            onClick={() => setPreviewMode('live')}
          >
            Split
          </button>
        </div>
      </div>

      <div className="editor-content">
        <div className="editor-with-ai">
          <MDEditor
            ref={editorRef}
            value={value}
            onChange={handleLocalChange}
            preview={previewMode}
            hideToolbar={false}
            visibleDragBar={false}
            data-color-mode="light"
            height={400}
            textareaProps={{
              placeholder,
              style: {
                fontSize: 14,
                lineHeight: 1.5,
                fontFamily: "'Monaco', 'Menlo', 'Ubuntu Mono', monospace"
              },
              onSelect: handleCursorChange,
              onKeyUp: handleCursorChange,
              onClick: handleCursorChange
            }}
          />

          {showAISuggestions && (
            <AISuggestions
              content={value}
              cursorPosition={cursorPosition}
              onSuggestionSelect={(text) => {
                const newValue = value.substring(0, cursorPosition) + 
                                text + 
                                value.substring(cursorPosition);
                handleLocalChange(newValue);
              }}
              onTitleSuggestion={onTitleSuggestion}
              onTagSuggestions={onTagSuggestions}
              isVisible={value && value.length > 10}
            />
          )}
        </div>
      </div>

      {/* User cursor indicators */}
      <div className="user-cursors">
        {userCursors.map(cursor => (
          <div
            key={cursor.userId}
            className="user-cursor"
            style={{
              // This would need more sophisticated positioning
              // For now, just show as indicators
            }}
            title={`${cursor.username} is editing`}
          >
            {cursor.username}
          </div>
        ))}
      </div>
    </div>
  );
};

export default CollaborativeEditor;