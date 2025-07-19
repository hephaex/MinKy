"""
Real-time Collaboration Service
Manages WebSocket connections and document collaboration
"""

from flask_socketio import SocketIO, emit, join_room, leave_room, rooms
from flask_jwt_extended import verify_jwt_in_request, get_jwt_identity
from app.models.document import Document
from app.models.user import User
from app import db
import json
import logging
from datetime import datetime
from typing import Dict, List, Optional
import difflib
import time

logger = logging.getLogger(__name__)

class CollaborationService:
    def __init__(self, socketio: SocketIO):
        self.socketio = socketio
        self.active_sessions: Dict[str, Dict] = {}  # document_id -> session info
        self.user_cursors: Dict[str, Dict] = {}  # document_id -> {user_id: cursor_info}
        self.operation_queue: Dict[str, List] = {}  # document_id -> list of operations
        
    def handle_connect(self, sid):
        """Handle new WebSocket connection"""
        logger.info(f"Client connected: {sid}")
        
    def handle_disconnect(self, sid):
        """Handle WebSocket disconnection"""
        logger.info(f"Client disconnected: {sid}")
        
        # Remove user from all document sessions
        for document_id in list(self.active_sessions.keys()):
            self.leave_document_session(document_id, sid)
    
    def join_document_session(self, document_id: str, user_id: int, sid: str):
        """Join a collaborative editing session for a document"""
        try:
            # Verify document access
            document = Document.query.get_or_404(document_id)
            user = User.query.get(user_id) if user_id else None
            
            if not document.can_view(user_id):
                emit('error', {'message': 'Access denied'}, room=sid)
                return
            
            # Initialize session if not exists
            if document_id not in self.active_sessions:
                self.active_sessions[document_id] = {
                    'users': {},
                    'last_save': datetime.utcnow(),
                    'content': document.markdown_content,
                    'version': 1
                }
                self.user_cursors[document_id] = {}
                self.operation_queue[document_id] = []
            
            # Add user to session
            room_name = f"document_{document_id}"
            join_room(room_name, sid=sid)
            
            user_info = {
                'user_id': user_id,
                'username': user.username if user else 'Anonymous',
                'sid': sid,
                'joined_at': datetime.utcnow(),
                'cursor_position': 0
            }
            
            self.active_sessions[document_id]['users'][sid] = user_info
            
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
            }, room=sid)
            
            # Notify other users
            emit('user_joined', {
                'user_id': user_id,
                'username': user.username if user else 'Anonymous'
            }, room=room_name, include_self=False)
            
            logger.info(f"User {user_id} joined document {document_id}")
            
        except Exception as e:
            logger.error(f"Error joining document session: {e}")
            emit('error', {'message': 'Failed to join document session'}, room=sid)
    
    def leave_document_session(self, document_id: str, sid: str):
        """Leave a collaborative editing session"""
        try:
            if document_id not in self.active_sessions:
                return
            
            session = self.active_sessions[document_id]
            if sid not in session['users']:
                return
            
            user_info = session['users'][sid]
            room_name = f"document_{document_id}"
            
            # Remove user from session
            del session['users'][sid]
            if document_id in self.user_cursors and sid in self.user_cursors[document_id]:
                del self.user_cursors[document_id][sid]
            
            leave_room(room_name, sid=sid)
            
            # Notify other users
            emit('user_left', {
                'user_id': user_info['user_id'],
                'username': user_info['username']
            }, room=room_name)
            
            # Clean up empty sessions
            if not session['users']:
                self._cleanup_session(document_id)
            
            logger.info(f"User {user_info['user_id']} left document {document_id}")
            
        except Exception as e:
            logger.error(f"Error leaving document session: {e}")
    
    def handle_text_operation(self, document_id: str, operation: Dict, user_id: int, sid: str):
        """Handle text editing operation"""
        try:
            if document_id not in self.active_sessions:
                emit('error', {'message': 'Not in document session'}, room=sid)
                return
            
            session = self.active_sessions[document_id]
            if sid not in session['users']:
                emit('error', {'message': 'Not in document session'}, room=sid)
                return
            
            # Validate operation
            if not self._validate_operation(operation):
                emit('error', {'message': 'Invalid operation'}, room=sid)
                return
            
            # Apply operation to session content
            new_content = self._apply_operation(session['content'], operation)
            if new_content is None:
                emit('error', {'message': 'Failed to apply operation'}, room=sid)
                return
            
            # Update session
            session['content'] = new_content
            session['version'] += 1
            
            # Add to operation queue
            operation_record = {
                'operation': operation,
                'user_id': user_id,
                'timestamp': datetime.utcnow(),
                'version': session['version']
            }
            self.operation_queue[document_id].append(operation_record)
            
            # Broadcast to other users
            room_name = f"document_{document_id}"
            emit('text_operation', {
                'operation': operation,
                'user_id': user_id,
                'version': session['version']
            }, room=room_name, include_self=False)
            
            # Auto-save periodically
            self._auto_save_if_needed(document_id)
            
        except Exception as e:
            logger.error(f"Error handling text operation: {e}")
            emit('error', {'message': 'Failed to process operation'}, room=sid)
    
    def handle_cursor_update(self, document_id: str, cursor_data: Dict, user_id: int, sid: str):
        """Handle cursor position update"""
        try:
            if document_id not in self.active_sessions:
                return
            
            session = self.active_sessions[document_id]
            if sid not in session['users']:
                return
            
            # Update cursor position
            session['users'][sid]['cursor_position'] = cursor_data.get('position', 0)
            
            if document_id not in self.user_cursors:
                self.user_cursors[document_id] = {}
            
            self.user_cursors[document_id][sid] = {
                'user_id': user_id,
                'username': session['users'][sid]['username'],
                'position': cursor_data.get('position', 0),
                'selection_start': cursor_data.get('selection_start'),
                'selection_end': cursor_data.get('selection_end'),
                'timestamp': datetime.utcnow()
            }
            
            # Broadcast to other users
            room_name = f"document_{document_id}"
            emit('cursor_update', {
                'user_id': user_id,
                'username': session['users'][sid]['username'],
                'cursor_data': cursor_data
            }, room=room_name, include_self=False)
            
        except Exception as e:
            logger.error(f"Error handling cursor update: {e}")
    
    def save_document(self, document_id: str, user_id: int):
        """Manually save document"""
        try:
            if document_id not in self.active_sessions:
                return False
            
            session = self.active_sessions[document_id]
            document = Document.query.get(document_id)
            
            if not document or not document.can_edit(user_id):
                return False
            
            # Update document content
            document.markdown_content = session['content']
            document.updated_at = datetime.utcnow()
            db.session.commit()
            
            session['last_save'] = datetime.utcnow()
            
            # Notify all users
            room_name = f"document_{document_id}"
            emit('document_saved', {
                'saved_by': user_id,
                'timestamp': datetime.utcnow().isoformat()
            }, room=room_name)
            
            logger.info(f"Document {document_id} saved by user {user_id}")
            return True
            
        except Exception as e:
            logger.error(f"Error saving document: {e}")
            return False
    
    def get_session_info(self, document_id: str) -> Optional[Dict]:
        """Get information about active session"""
        if document_id not in self.active_sessions:
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
    
    def _validate_operation(self, operation: Dict) -> bool:
        """Validate text operation"""
        required_fields = ['type', 'position']
        if not all(field in operation for field in required_fields):
            return False
        
        operation_type = operation['type']
        if operation_type == 'insert':
            return 'text' in operation
        elif operation_type == 'delete':
            return 'length' in operation
        elif operation_type == 'replace':
            return 'text' in operation and 'length' in operation
        
        return False
    
    def _apply_operation(self, content: str, operation: Dict) -> Optional[str]:
        """Apply text operation to content"""
        try:
            operation_type = operation['type']
            position = operation['position']
            
            if position < 0 or position > len(content):
                return None
            
            if operation_type == 'insert':
                text = operation['text']
                return content[:position] + text + content[position:]
            
            elif operation_type == 'delete':
                length = operation['length']
                if position + length > len(content):
                    return None
                return content[:position] + content[position + length:]
            
            elif operation_type == 'replace':
                length = operation['length']
                text = operation['text']
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
        time_since_save = datetime.utcnow() - session['last_save']
        if time_since_save.total_seconds() > 30:
            # Find any user who can edit
            for user_info in session['users'].values():
                if self.save_document(document_id, user_info['user_id']):
                    break
    
    def _cleanup_session(self, document_id: str):
        """Clean up empty session"""
        if document_id in self.active_sessions:
            del self.active_sessions[document_id]
        if document_id in self.user_cursors:
            del self.user_cursors[document_id]
        if document_id in self.operation_queue:
            del self.operation_queue[document_id]
        
        logger.info(f"Cleaned up session for document {document_id}")

# Global collaboration service instance
collaboration_service = None

def init_collaboration_service(socketio: SocketIO):
    """Initialize collaboration service with SocketIO instance"""
    global collaboration_service
    collaboration_service = CollaborationService(socketio)