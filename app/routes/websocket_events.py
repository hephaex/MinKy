"""
WebSocket Event Handlers for Real-time Collaboration
"""

from flask import request
from flask_socketio import emit
from flask_jwt_extended import verify_jwt_in_request
from app.services.collaboration_service import collaboration_service
from app.utils.auth import get_current_user_id
import logging

logger = logging.getLogger(__name__)

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
            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return
            
            # Get user ID from JWT token (optional for anonymous users)
            user_id = None
            try:
                verify_jwt_in_request()
                user_id = get_current_user_id()
            except:
                pass  # Allow anonymous users
            
            collaboration_service.join_document_session(
                document_id=str(document_id),
                user_id=user_id,
                sid=request.sid
            )
            
        except Exception as e:
            logger.error(f"Error joining document: {e}")
            emit('error', {'message': 'Failed to join document'})
    
    @socketio.on('leave_document')
    def handle_leave_document(data):
        """Handle leaving a document"""
        try:
            document_id = data.get('document_id')
            if not document_id:
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
            document_id = data.get('document_id')
            operation = data.get('operation')
            
            if not document_id or not operation:
                emit('error', {'message': 'Document ID and operation required'})
                return
            
            # Get user ID from JWT token
            user_id = None
            try:
                verify_jwt_in_request()
                user_id = get_current_user_id()
            except:
                pass  # Allow anonymous users
            
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
            document_id = data.get('document_id')
            cursor_data = data.get('cursor_data')
            
            if not document_id or not cursor_data:
                return
            
            # Get user ID from JWT token
            user_id = None
            try:
                verify_jwt_in_request()
                user_id = get_current_user_id()
            except:
                pass  # Allow anonymous users
            
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
            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return
            
            # Get user ID from JWT token
            user_id = None
            try:
                verify_jwt_in_request()
                user_id = get_current_user_id()
            except:
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
            document_id = data.get('document_id')
            if not document_id:
                emit('error', {'message': 'Document ID required'})
                return
            
            session_info = collaboration_service.get_session_info(str(document_id))
            
            emit('session_info', {
                'session': session_info
            })
            
        except Exception as e:
            logger.error(f"Error getting session info: {e}")
            emit('error', {'message': 'Failed to get session info'})