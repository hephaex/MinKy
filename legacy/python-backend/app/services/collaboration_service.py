"""
Real-time Collaboration Service
Manages WebSocket connections and document collaboration
"""

from flask_socketio import SocketIO, emit, join_room, leave_room
from app.models.document import Document
from app.models.user import User
from app import db
import logging
import json
import threading
from collections import deque
from datetime import datetime, timezone
from typing import Dict, List, Optional, Deque
import time
import bleach

logger = logging.getLogger(__name__)

class CollaborationService:
    # SECURITY: Limit operation queue size to prevent unbounded memory growth
    MAX_OPERATION_QUEUE_SIZE = 1000

    # SECURITY: Rate limiting and connection limits
    MAX_CONNECTIONS_PER_USER = 5  # Maximum concurrent connections per user
    OPERATION_RATE_LIMIT = 60  # Max operations per minute per user
    CURSOR_UPDATE_THROTTLE_MS = 100  # Minimum ms between cursor updates
    MAX_ACTIVE_SESSIONS = 1000  # Maximum total active sessions

    # SECURITY: Cursor data validation limits
    MAX_CURSOR_POSITION = 10000000  # 10M character limit
    MAX_CURSOR_USERNAME_LENGTH = 100

    def __init__(self, socketio: SocketIO):
        self.socketio = socketio
        self.active_sessions: Dict[str, Dict] = {}  # document_id -> session info
        self.user_cursors: Dict[str, Dict] = {}  # document_id -> {user_id: cursor_info}
        # SECURITY: Use bounded deque instead of unbounded list
        self.operation_queue: Dict[str, Deque] = {}  # document_id -> deque of operations
        # SECURITY: Track user connection counts and rate limits
        self.user_connections: Dict[int, int] = {}  # user_id -> connection count
        self.user_operation_counts: Dict[int, List[float]] = {}  # user_id -> list of operation timestamps
        self.user_last_cursor_update: Dict[int, float] = {}  # user_id -> last cursor update timestamp
        # SECURITY: Threading lock for concurrent edit protection
        self._session_lock = threading.RLock()
        # SECURITY: Per-document locks for fine-grained concurrency control
        self._document_locks: Dict[str, threading.RLock] = {}

    def _get_document_lock(self, document_id: str) -> threading.RLock:
        """SECURITY: Get or create a lock for a specific document"""
        with self._session_lock:
            if document_id not in self._document_locks:
                self._document_locks[document_id] = threading.RLock()
            return self._document_locks[document_id]

    def _log_collaboration_operation(self, operation: str, user_id: int,
                                      document_id: str = None, details: dict = None) -> None:
        """SECURITY: Audit log collaboration operations for compliance"""
        log_entry = {
            'operation': operation,
            'user_id': user_id,
            'document_id': document_id,
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'details': details or {}
        }
        logger.info(f"AUDIT_COLLABORATION: {json.dumps(log_entry)}")

    def _check_rate_limit(self, user_id: int) -> bool:
        """SECURITY: Check if user has exceeded operation rate limit"""
        current_time = time.time()
        if user_id not in self.user_operation_counts:
            self.user_operation_counts[user_id] = []

        # Remove operations older than 1 minute
        self.user_operation_counts[user_id] = [
            t for t in self.user_operation_counts[user_id]
            if current_time - t < 60
        ]

        if len(self.user_operation_counts[user_id]) >= self.OPERATION_RATE_LIMIT:
            logger.warning(f"Rate limit exceeded for user {user_id}: {len(self.user_operation_counts[user_id])} ops/min")
            return False

        self.user_operation_counts[user_id].append(current_time)
        return True

    def _check_connection_limit(self, user_id: int) -> bool:
        """SECURITY: Check if user has exceeded connection limit"""
        current_count = self.user_connections.get(user_id, 0)
        if current_count >= self.MAX_CONNECTIONS_PER_USER:
            logger.warning(f"Connection limit exceeded for user {user_id}: {current_count} connections")
            return False
        return True

    def _increment_user_connections(self, user_id: int) -> None:
        """Track user connection count"""
        self.user_connections[user_id] = self.user_connections.get(user_id, 0) + 1

    def _decrement_user_connections(self, user_id: int) -> None:
        """Decrement user connection count"""
        if user_id in self.user_connections:
            self.user_connections[user_id] = max(0, self.user_connections[user_id] - 1)
            if self.user_connections[user_id] == 0:
                del self.user_connections[user_id]

    def _should_throttle_cursor(self, user_id: int) -> bool:
        """SECURITY: Check if cursor update should be throttled"""
        current_time = time.time() * 1000  # Convert to milliseconds
        last_update = self.user_last_cursor_update.get(user_id, 0)

        if current_time - last_update < self.CURSOR_UPDATE_THROTTLE_MS:
            return True

        self.user_last_cursor_update[user_id] = current_time
        return False

    def _validate_cursor_data(self, cursor_data: Dict) -> Optional[Dict]:
        """SECURITY: Validate and sanitize cursor data to prevent XSS"""
        if not isinstance(cursor_data, dict):
            return None

        validated = {}

        # Validate position
        position = cursor_data.get('position')
        if isinstance(position, int) and 0 <= position <= self.MAX_CURSOR_POSITION:
            validated['position'] = position
        else:
            validated['position'] = 0

        # Validate selection_start
        selection_start = cursor_data.get('selection_start')
        if isinstance(selection_start, int) and 0 <= selection_start <= self.MAX_CURSOR_POSITION:
            validated['selection_start'] = selection_start
        else:
            validated['selection_start'] = None

        # Validate selection_end
        selection_end = cursor_data.get('selection_end')
        if isinstance(selection_end, int) and 0 <= selection_end <= self.MAX_CURSOR_POSITION:
            validated['selection_end'] = selection_end
        else:
            validated['selection_end'] = None

        return validated
        
    def handle_connect(self, sid):
        """Handle new WebSocket connection"""
        logger.info(f"Client connected: {sid}")
        
    def handle_disconnect(self, sid):
        """Handle WebSocket disconnection"""
        logger.info(f"Client disconnected: {sid}")
        
        # Remove user from all document sessions
        for document_id in list(self.active_sessions.keys()):
            self.leave_document_session(document_id, sid)
    
    def join_document_session(self, document_id: str, user_id: int, sid: str, verified_user_id: int = None):
        """Join a collaborative editing session for a document

        Args:
            document_id: The document to join
            user_id: The user ID claimed by the client
            sid: The socket session ID
            verified_user_id: The authenticated user ID from JWT (must match user_id)
        """
        try:
            # SECURITY: Always require verified_user_id to prevent impersonation
            if verified_user_id is None:
                emit('error', {'message': 'Authentication required for collaboration'}, room=sid)  # type: ignore[call-arg]
                logger.warning(f"WebSocket connection attempted without authentication: sid={sid}")
                return

            # Verify user_id matches authenticated user to prevent impersonation
            if user_id != verified_user_id:
                emit('error', {'message': 'User ID mismatch - authentication required'}, room=sid)  # type: ignore[call-arg]
                logger.warning(f"WebSocket user ID mismatch: claimed={user_id}, verified={verified_user_id}")
                return

            # SECURITY: Check connection limit per user
            if not self._check_connection_limit(user_id):
                emit('error', {'message': 'Connection limit exceeded'}, room=sid)  # type: ignore[call-arg]
                return

            # SECURITY: Use session lock for thread-safe session management
            with self._session_lock:
                # SECURITY: Check total session limit
                if len(self.active_sessions) >= self.MAX_ACTIVE_SESSIONS:
                    emit('error', {'message': 'Server session limit reached'}, room=sid)  # type: ignore[call-arg]
                    logger.warning(f"Max active sessions limit reached: {len(self.active_sessions)}")
                    return

            # Verify document access
            document = Document.query.get_or_404(document_id)
            user = db.session.get(User, user_id) if user_id else None

            if not document.can_view(user_id):
                emit('error', {'message': 'Access denied'}, room=sid)  # type: ignore[call-arg]
                return

            # SECURITY: Use document-specific lock for initialization
            doc_lock = self._get_document_lock(document_id)
            with doc_lock:
                # Initialize session if not exists
                if document_id not in self.active_sessions:
                    self.active_sessions[document_id] = {
                        'users': {},
                        'last_save': datetime.now(timezone.utc),
                        'content': document.markdown_content,
                        'version': 1,
                        'last_editor_id': None  # SECURITY: Track actual editor
                    }
                    self.user_cursors[document_id] = {}
                    # SECURITY: Bounded deque for operation queue
                    self.operation_queue[document_id] = deque(maxlen=self.MAX_OPERATION_QUEUE_SIZE)

                # Add user to session
                room_name = f"document_{document_id}"
                join_room(room_name, sid=sid)

                # SECURITY: Sanitize username for display
                safe_username = bleach.clean(user.username if user else 'Anonymous')[:self.MAX_CURSOR_USERNAME_LENGTH]

                user_info = {
                    'user_id': user_id,
                    'username': safe_username,
                    'sid': sid,
                    'joined_at': datetime.now(timezone.utc),
                    'cursor_position': 0
                }

                self.active_sessions[document_id]['users'][sid] = user_info

                # SECURITY: Track user connection count
                self._increment_user_connections(user_id)

            # Send current document state to new user
            emit('document_joined', {
                'document_id': document_id,
                'content': self.active_sessions[document_id]['content'],
                'version': self.active_sessions[document_id]['version'],
                'active_users': [
                    {
                        'username': u['username'],
                        'user_id': u['user_id'],
                        'cursor_position': u.get('cursor_position', 0)
                    }
                    for u in self.active_sessions[document_id]['users'].values()
                ]
            }, room=sid)  # type: ignore[call-arg]  # type: ignore[call-arg]

            # Notify other users
            emit('user_joined', {
                'user_id': user_id,
                'username': safe_username
            }, room=room_name, include_self=False)  # type: ignore[call-arg]

            # SECURITY: Audit log session join
            self._log_collaboration_operation('join_session', user_id, document_id)

            logger.info(f"User {user_id} joined document {document_id}")

        except Exception as e:
            logger.error(f"Error joining document session: {e}")
            emit('error', {'message': 'Failed to join document session'}, room=sid)  # type: ignore[call-arg]
    
    def leave_document_session(self, document_id: str, sid: str):
        """Leave a collaborative editing session"""
        try:
            # SECURITY: Use document lock for thread-safe session management
            doc_lock = self._get_document_lock(document_id)
            with doc_lock:
                if document_id not in self.active_sessions:
                    return

                session = self.active_sessions[document_id]
                if sid not in session['users']:
                    return

                user_info = session['users'][sid]
                room_name = f"document_{document_id}"

                # SECURITY: Decrement user connection count
                self._decrement_user_connections(user_info['user_id'])

                # Remove user from session
                del session['users'][sid]
                if document_id in self.user_cursors and sid in self.user_cursors[document_id]:
                    del self.user_cursors[document_id][sid]

            leave_room(room_name, sid=sid)

            # Notify other users
            emit('user_left', {
                'user_id': user_info['user_id'],
                'username': user_info['username']
            }, room=room_name)  # type: ignore[call-arg]

            # SECURITY: Audit log session leave
            self._log_collaboration_operation('leave_session', user_info['user_id'], document_id)

            # Clean up empty sessions
            if not session['users']:
                self._cleanup_session(document_id)

            logger.info(f"User {user_info['user_id']} left document {document_id}")

        except Exception as e:
            logger.error(f"Error leaving document session: {e}")
    
    def handle_text_operation(self, document_id: str, operation: Dict, user_id: int, sid: str):
        """Handle text editing operation"""
        try:
            # SECURITY: Use document lock for thread-safe content modification
            doc_lock = self._get_document_lock(document_id)
            with doc_lock:
                if document_id not in self.active_sessions:
                    emit('error', {'message': 'Not in document session'}, room=sid)  # type: ignore[call-arg]
                    return

                session = self.active_sessions[document_id]
                if sid not in session['users']:
                    emit('error', {'message': 'Not in document session'}, room=sid)  # type: ignore[call-arg]
                    return

                # SECURITY: Check rate limit before processing operation
                if not self._check_rate_limit(user_id):
                    emit('error', {'message': 'Rate limit exceeded. Please slow down.'}, room=sid)  # type: ignore[call-arg]
                    return

                # Authorization check: verify user has edit permission
                document = Document.query.get(document_id)
                if not document or not document.can_edit(user_id):
                    emit('error', {'message': 'Edit permission denied'}, room=sid)  # type: ignore[call-arg]
                    logger.warning(f"Edit permission denied for user {user_id} on document {document_id}")
                    return

                # Validate operation
                if not self._validate_operation(operation):
                    emit('error', {'message': 'Invalid operation'}, room=sid)  # type: ignore[call-arg]
                    return

                # Apply operation to session content
                new_content = self._apply_operation(session['content'], operation)
                if new_content is None:
                    emit('error', {'message': 'Failed to apply operation'}, room=sid)  # type: ignore[call-arg]
                    return

                # Update session
                session['content'] = new_content
                session['version'] += 1
                # SECURITY: Track the actual editor for correct attribution
                session['last_editor_id'] = user_id

                # Add to operation queue
                operation_record = {
                    'operation': operation,
                    'user_id': user_id,
                    'timestamp': datetime.now(timezone.utc),
                    'version': session['version']
                }
                self.operation_queue[document_id].append(operation_record)

            # Broadcast to other users (outside lock to prevent deadlock)
            room_name = f"document_{document_id}"
            emit('text_operation', {
                'operation': operation,
                'user_id': user_id,
                'version': session['version']
            }, room=room_name, include_self=False)  # type: ignore[call-arg]

            # Auto-save periodically (uses last_editor_id for correct attribution)
            self._auto_save_if_needed(document_id)

        except Exception as e:
            logger.error(f"Error handling text operation: {e}")
            emit('error', {'message': 'Failed to process operation'}, room=sid)  # type: ignore[call-arg]
    
    def handle_cursor_update(self, document_id: str, cursor_data: Dict, user_id: int, sid: str):
        """Handle cursor position update"""
        try:
            if document_id not in self.active_sessions:
                return

            session = self.active_sessions[document_id]
            if sid not in session['users']:
                return

            # SECURITY: Throttle cursor updates to prevent DoS
            if self._should_throttle_cursor(user_id):
                return

            # SECURITY: Validate and sanitize cursor data to prevent XSS
            validated_cursor = self._validate_cursor_data(cursor_data)
            if validated_cursor is None:
                logger.warning(f"Invalid cursor data from user {user_id}")
                return

            # Update cursor position with validated data
            session['users'][sid]['cursor_position'] = validated_cursor.get('position', 0)

            if document_id not in self.user_cursors:
                self.user_cursors[document_id] = {}

            self.user_cursors[document_id][sid] = {
                'user_id': user_id,
                'username': session['users'][sid]['username'],
                'position': validated_cursor.get('position', 0),
                'selection_start': validated_cursor.get('selection_start'),
                'selection_end': validated_cursor.get('selection_end'),
                'timestamp': datetime.now(timezone.utc)
            }

            # Broadcast to other users with validated data only
            room_name = f"document_{document_id}"
            emit('cursor_update', {
                'user_id': user_id,
                'username': session['users'][sid]['username'],
                'cursor_data': validated_cursor  # SECURITY: Use validated data
            }, room=room_name, include_self=False)  # type: ignore[call-arg]

        except Exception as e:
            logger.error(f"Error handling cursor update: {e}")
    
    def save_document(self, document_id: str, user_id: int):
        """Manually save document"""
        try:
            # SECURITY: Use document lock for thread-safe save
            doc_lock = self._get_document_lock(document_id)
            with doc_lock:
                if document_id not in self.active_sessions:
                    return False

                session = self.active_sessions[document_id]
                document = db.session.get(Document, document_id)

                if not document or not document.can_edit(user_id):
                    return False

                # Update document content
                document.markdown_content = session['content']
                document.updated_at = datetime.now(timezone.utc)
                db.session.commit()

                session['last_save'] = datetime.now(timezone.utc)

            # Notify all users (outside lock)
            room_name = f"document_{document_id}"
            emit('document_saved', {
                'saved_by': user_id,
                'timestamp': datetime.now(timezone.utc).isoformat()
            }, room=room_name)  # type: ignore[call-arg]

            # SECURITY: Audit log document save
            self._log_collaboration_operation('save', user_id, document_id)

            logger.info(f"Document {document_id} saved by user {user_id}")
            return True

        except Exception as e:
            logger.error(f"Error saving document: {e}")
            return False
    
    def get_session_info(self, document_id: str, requesting_user_id: Optional[int] = None) -> Optional[Dict]:
        """Get information about active session

        Args:
            document_id: The document session to query
            requesting_user_id: The authenticated user making the request (required for authorization)
        """
        if document_id not in self.active_sessions:
            return None

        # SECURITY: Require authentication and authorization
        document = Document.query.get(document_id)
        if not document:
            return None

        # Check if user can view this document
        if requesting_user_id is None:
            # Unauthenticated - only allow for public documents
            if not document.is_public:
                logger.warning(f"Unauthorized session info request for document {document_id}")
                return None
        else:
            # Authenticated - check view permission
            if not document.can_view(requesting_user_id):
                logger.warning(f"User {requesting_user_id} denied session info for document {document_id}")
                return None

        session = self.active_sessions[document_id]
        return {
            'document_id': document_id,
            'active_users': [
                {
                    'user_id': u['user_id'],
                    'username': u['username'],
                    'joined_at': u['joined_at'].isoformat()
                }
                for u in session['users'].values()
            ],
            'version': session['version'],
            'last_save': session['last_save'].isoformat()
        }
    
    # SECURITY: Limits for operation validation
    MAX_TEXT_LENGTH = 100000  # 100KB limit for single operation
    MAX_POSITION = 10000000  # 10MB document position limit

    def _validate_operation(self, operation: Dict) -> bool:
        """Validate text operation with comprehensive type and range checks"""
        if not isinstance(operation, dict):
            return False

        required_fields = ['type', 'position']
        if not all(field in operation for field in required_fields):
            return False

        # SECURITY: Validate operation type against whitelist
        operation_type = operation.get('type')
        if operation_type not in ('insert', 'delete', 'replace'):
            return False

        # SECURITY: Validate position is non-negative integer within bounds
        position = operation.get('position')
        if not isinstance(position, int) or position < 0 or position > self.MAX_POSITION:
            return False

        if operation_type == 'insert':
            text = operation.get('text')
            # SECURITY: Validate text is string and within size limit
            if not isinstance(text, str) or len(text) > self.MAX_TEXT_LENGTH:
                return False
            return True

        elif operation_type == 'delete':
            length = operation.get('length')
            # SECURITY: Validate length is positive integer within bounds
            if not isinstance(length, int) or length <= 0 or length > self.MAX_TEXT_LENGTH:
                return False
            return True

        elif operation_type == 'replace':
            text = operation.get('text')
            length = operation.get('length')
            # SECURITY: Validate both text and length
            if not isinstance(text, str) or len(text) > self.MAX_TEXT_LENGTH:
                return False
            if not isinstance(length, int) or length <= 0 or length > self.MAX_TEXT_LENGTH:
                return False
            return True

        return False
    
    def _apply_operation(self, content: str, operation: Dict) -> Optional[str]:
        """Apply text operation to content"""
        try:
            operation_type = operation['type']
            position = operation['position']
            
            if position < 0 or position > len(content):
                return None
            
            if operation_type == 'insert':
                text = str(operation['text'])
                return content[:position] + text + content[position:]
            
            elif operation_type == 'delete':
                length = operation['length']
                if position + length > len(content):
                    return None
                return content[:position] + content[position + length:]
            
            elif operation_type == 'replace':
                length = operation['length']
                text = str(operation['text'])
                if position + length > len(content):
                    return None
                return content[:position] + text + content[position + length:]
            
            return None
            
        except Exception as e:
            logger.error(f"Error applying operation: {e}")
            return None
    
    def _auto_save_if_needed(self, document_id: str):
        """Auto-save document if needed"""
        session = self.active_sessions[document_id]

        # Save every 30 seconds of activity
        time_since_save = datetime.now(timezone.utc) - session['last_save']
        if time_since_save.total_seconds() > 30:
            # SECURITY: Use the actual last editor for correct attribution
            last_editor_id = session.get('last_editor_id')
            if last_editor_id:
                self.save_document(document_id, last_editor_id)
            else:
                # Fallback: find any user who can edit (shouldn't normally happen)
                for user_info in session['users'].values():
                    if self.save_document(document_id, user_info['user_id']):
                        break
    
    def _cleanup_session(self, document_id: str):
        """Clean up empty session"""
        with self._session_lock:
            if document_id in self.active_sessions:
                del self.active_sessions[document_id]
            if document_id in self.user_cursors:
                del self.user_cursors[document_id]
            if document_id in self.operation_queue:
                del self.operation_queue[document_id]
            # SECURITY: Clean up document lock
            if document_id in self._document_locks:
                del self._document_locks[document_id]

        logger.info(f"Cleaned up session for document {document_id}")

# Global collaboration service instance
collaboration_service = None

def init_collaboration_service(socketio: SocketIO):
    """Initialize collaboration service with SocketIO instance"""
    global collaboration_service
    collaboration_service = CollaborationService(socketio)