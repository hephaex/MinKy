"""
WebSocket Event Handlers for Real-time Collaboration
"""

from flask import request
from flask_socketio import emit
from flask_jwt_extended import verify_jwt_in_request
from flask_jwt_extended.exceptions import NoAuthorizationError
from jwt.exceptions import PyJWTError
from app.services.collaboration_service import collaboration_service
from app.utils.auth import get_current_user_id
import logging

logger = logging.getLogger(__name__)


def get_websocket_user_id():
    """Get user_id from JWT in websocket context, None for anonymous users."""
    try:
        verify_jwt_in_request()
        return get_current_user_id()
    except (NoAuthorizationError, PyJWTError):
        return None  # Allow anonymous users
    except Exception as e:
        logger.debug("Unexpected error getting websocket user_id: %s", e)
        return None

def register_websocket_events(socketio):
    """Register WebSocket event handlers"""
    
    @socketio.on('connect')
    def handle_connect():
        """Handle client connection"""
        logger.info("Client connected")
        collaboration_service.handle_connect(request.sid)
    
    @socketio.on('disconnect')
    def handle_disconnect():
        """Handle client disconnection"""
        logger.info("Client disconnected")
        collaboration_service.handle_disconnect(request.sid)
    
    @socketio.on('join_document')
    def handle_join_document(data):
        """Handle joining a document for collaboration"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                emit('error', {'message': 'Invalid request format'})
                return

            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    emit('error', {'message': 'Invalid document ID'})
                    return
            except (ValueError, TypeError):
                emit('error', {'message': 'Invalid document ID format'})
                return
            
            # Get user ID from JWT token (optional for anonymous users)
            user_id = get_websocket_user_id()
            
            # SECURITY: Pass verified_user_id to prevent impersonation
            collaboration_service.join_document_session(
                document_id=str(document_id),
                user_id=user_id,
                sid=request.sid,
                verified_user_id=user_id
            )
            
        except Exception as e:
            logger.error(f"Error joining document: {e}")
            emit('error', {'message': 'Failed to join document'})
    
    @socketio.on('leave_document')
    def handle_leave_document(data):
        """Handle leaving a document"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                return

            document_id = data.get('document_id')
            if not document_id:
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    return
            except (ValueError, TypeError):
                return

            collaboration_service.leave_document_session(
                document_id=str(document_id),
                sid=request.sid
            )

        except Exception as e:
            logger.error(f"Error leaving document: {e}")
    
    @socketio.on('text_operation')
    def handle_text_operation(data):
        """Handle text editing operation"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                emit('error', {'message': 'Invalid request format'})
                return

            document_id = data.get('document_id')
            operation = data.get('operation')

            if not document_id or not operation:
                emit('error', {'message': 'Document ID and operation required'})
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    emit('error', {'message': 'Invalid document ID'})
                    return
            except (ValueError, TypeError):
                emit('error', {'message': 'Invalid document ID format'})
                return

            # SECURITY: Require authentication for text operations
            user_id = get_websocket_user_id()
            if not user_id:
                emit('error', {'message': 'Authentication required for editing'})
                return

            collaboration_service.handle_text_operation(
                document_id=str(document_id),
                operation=operation,
                user_id=user_id,
                sid=request.sid
            )
            
        except Exception as e:
            logger.error(f"Error handling text operation: {e}")
            emit('error', {'message': 'Failed to process text operation'})
    
    @socketio.on('cursor_update')
    def handle_cursor_update(data):
        """Handle cursor position update"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                return

            document_id = data.get('document_id')
            cursor_data = data.get('cursor_data')

            if not document_id or not cursor_data:
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    return
            except (ValueError, TypeError):
                return

            # SECURITY: Require authentication for cursor updates
            user_id = get_websocket_user_id()
            if not user_id:
                return  # Silently ignore anonymous cursor updates

            collaboration_service.handle_cursor_update(
                document_id=str(document_id),
                cursor_data=cursor_data,
                user_id=user_id,
                sid=request.sid
            )

        except Exception as e:
            logger.error(f"Error handling cursor update: {e}")
    
    @socketio.on('save_document')
    def handle_save_document(data):
        """Handle manual document save"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                emit('error', {'message': 'Invalid request format'})
                return

            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    emit('error', {'message': 'Invalid document ID'})
                    return
            except (ValueError, TypeError):
                emit('error', {'message': 'Invalid document ID format'})
                return

            # Get user ID from JWT token (required for saving)
            user_id = get_websocket_user_id()
            if not user_id:
                emit('error', {'message': 'Authentication required to save'})
                return

            success = collaboration_service.save_document(
                document_id=str(document_id),
                user_id=user_id
            )

            if success:
                emit('save_success', {'message': 'Document saved'})
            else:
                emit('error', {'message': 'Failed to save document'})

        except Exception as e:
            logger.error(f"Error saving document: {e}")
            emit('error', {'message': 'Failed to save document'})
    
    @socketio.on('get_session_info')
    def handle_get_session_info(data):
        """Get information about active collaboration session"""
        try:
            # SECURITY: Validate input data type
            if not isinstance(data, dict):
                emit('error', {'message': 'Invalid request format'})
                return

            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return

            # SECURITY: Validate document_id type
            try:
                doc_id = int(document_id)
                if doc_id < 1:
                    emit('error', {'message': 'Invalid document ID'})
                    return
            except (ValueError, TypeError):
                emit('error', {'message': 'Invalid document ID format'})
                return

            # SECURITY: Require authentication to prevent information disclosure
            user_id = get_websocket_user_id()
            if not user_id:
                emit('error', {'message': 'Authentication required'})
                return

            session_info = collaboration_service.get_session_info(
                str(document_id),
                requesting_user_id=user_id
            )

            if session_info is None:
                emit('error', {'message': 'Session not found or access denied'})
                return

            emit('session_info', {
                'session': session_info
            })

        except Exception as e:
            logger.error(f"Error getting session info: {e}")
            emit('error', {'message': 'Failed to get session info'})