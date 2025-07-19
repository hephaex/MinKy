"""
Admin Panel API Routes
Provides comprehensive administrative functionality
"""

from flask import Blueprint, jsonify, request
from flask_jwt_extended import jwt_required, get_jwt_identity
from sqlalchemy import func, desc, and_
from datetime import datetime, timedelta
from app import db
from app.models.user import User
from app.models.document import Document
from app.models.tag import Tag
from app.models.comment import Comment
from app.models.attachment import Attachment
from app.models.workflow import DocumentWorkflow
import logging

logger = logging.getLogger(__name__)

admin_bp = Blueprint('admin', __name__)

def require_admin():
    """Check if current user is admin"""
    current_user_id = get_jwt_identity()
    if not current_user_id:
        return False
    
    user = User.query.get(current_user_id)
    return user and user.is_admin

@admin_bp.route('/admin/users', methods=['GET'])
@jwt_required()
def list_users():
    """List all users with pagination"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')
        
        query = User.query
        
        if search:
            query = query.filter(
                User.username.ilike(f'%{search}%') |
                User.email.ilike(f'%{search}%') |
                User.full_name.ilike(f'%{search}%')
            )
        
        pagination = query.order_by(desc(User.created_at)).paginate(
            page=page,
            per_page=per_page,
            error_out=False
        )
        
        users = []
        for user in pagination.items:
            user_data = user.to_dict(include_sensitive=True)
            # Add stats
            doc_count = Document.query.filter_by(user_id=user.id).count()
            comment_count = Comment.query.filter_by(user_id=user.id).count()
            user_data['document_count'] = doc_count
            user_data['comment_count'] = comment_count
            users.append(user_data)
        
        return jsonify({
            'success': True,
            'users': users,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            }
        })
        
    except Exception as e:
        logger.error(f"Error in list users: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/users/<int:user_id>', methods=['GET'])
@jwt_required()
def get_user_details(user_id):
    """Get detailed user information"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        user = User.query.get_or_404(user_id)
        user_data = user.to_dict(include_sensitive=True)
        
        # Add detailed stats
        documents = Document.query.filter_by(user_id=user.id).all()
        comments = Comment.query.filter_by(user_id=user.id).all()
        
        # Recent activity
        recent_docs = Document.query.filter(
            and_(
                Document.user_id == user.id,
                Document.created_at >= datetime.utcnow() - timedelta(days=30)
            )
        ).count()
        
        user_data.update({
            'document_count': len(documents),
            'comment_count': len(comments),
            'recent_documents': recent_docs,
            'recent_activity': [
                {
                    'type': 'document',
                    'title': doc.title,
                    'created_at': doc.created_at.isoformat() if doc.created_at else None
                }
                for doc in documents[-5:]  # Last 5 documents
            ]
        })
        
        return jsonify({
            'success': True,
            'user': user_data
        })
        
    except Exception as e:
        logger.error(f"Error getting user details: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/users/<int:user_id>', methods=['PUT'])
@jwt_required()
def update_user(user_id):
    """Update user information"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        user = User.query.get_or_404(user_id)
        data = request.get_json()
        
        # Update allowed fields
        if 'is_active' in data:
            user.is_active = data['is_active']
        if 'is_admin' in data:
            user.is_admin = data['is_admin']
        if 'full_name' in data:
            user.full_name = data['full_name']
        
        user.updated_at = datetime.utcnow()
        db.session.commit()
        
        return jsonify({
            'success': True,
            'message': 'User updated successfully',
            'user': user.to_dict(include_sensitive=True)
        })
        
    except Exception as e:
        logger.error(f"Error updating user: {e}")
        db.session.rollback()
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/documents', methods=['GET'])
@jwt_required()
def list_all_documents():
    """List all documents with admin privileges"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')
        
        query = Document.query
        
        if search:
            query = query.filter(
                Document.title.ilike(f'%{search}%') |
                Document.markdown_content.ilike(f'%{search}%')
            )
        
        pagination = query.order_by(desc(Document.created_at)).paginate(
            page=page,
            per_page=per_page,
            error_out=False
        )
        
        documents = []
        for doc in pagination.items:
            doc_data = doc.to_dict()
            # Add owner info
            if doc.owner:
                doc_data['owner'] = {
                    'id': doc.owner.id,
                    'username': doc.owner.username,
                    'full_name': doc.owner.full_name
                }
            documents.append(doc_data)
        
        return jsonify({
            'success': True,
            'documents': documents,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': pagination.total,
                'pages': pagination.pages,
                'has_next': pagination.has_next,
                'has_prev': pagination.has_prev
            }
        })
        
    except Exception as e:
        logger.error(f"Error in list all documents: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/system/stats', methods=['GET'])
@jwt_required()
def get_system_stats():
    """Get comprehensive system statistics"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        # Basic counts
        total_users = User.query.count()
        active_users = User.query.filter_by(is_active=True).count()
        admin_users = User.query.filter_by(is_admin=True).count()
        total_documents = Document.query.count()
        public_documents = Document.query.filter_by(is_public=True).count()
        total_tags = Tag.query.count()
        total_comments = Comment.query.count()
        total_attachments = Attachment.query.count()
        
        # Recent activity (last 7 days)
        week_ago = datetime.utcnow() - timedelta(days=7)
        new_users_week = User.query.filter(User.created_at >= week_ago).count()
        new_docs_week = Document.query.filter(Document.created_at >= week_ago).count()
        new_comments_week = Comment.query.filter(Comment.created_at >= week_ago).count()
        
        # Storage estimates
        avg_doc_size = db.session.query(
            func.avg(func.length(Document.markdown_content))
        ).scalar() or 0
        
        estimated_storage_kb = total_documents * (avg_doc_size / 1024)
        
        return jsonify({
            'success': True,
            'stats': {
                'users': {
                    'total': total_users,
                    'active': active_users,
                    'admins': admin_users,
                    'new_this_week': new_users_week
                },
                'content': {
                    'documents': total_documents,
                    'public_documents': public_documents,
                    'tags': total_tags,
                    'comments': total_comments,
                    'attachments': total_attachments,
                    'new_documents_week': new_docs_week,
                    'new_comments_week': new_comments_week
                },
                'storage': {
                    'estimated_kb': round(estimated_storage_kb, 2),
                    'avg_document_size': round(avg_doc_size, 2)
                }
            }
        })
        
    except Exception as e:
        logger.error(f"Error getting system stats: {e}")
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/system/cleanup', methods=['POST'])
@jwt_required()
def system_cleanup():
    """Perform system cleanup operations"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        data = request.get_json() or {}
        cleanup_type = data.get('type', 'all')
        
        results = {}
        
        if cleanup_type in ['all', 'orphaned_attachments']:
            # Find orphaned attachments
            orphaned_attachments = db.session.query(Attachment).filter(
                ~Attachment.document_id.in_(
                    db.session.query(Document.id)
                )
            ).all()
            
            count = len(orphaned_attachments)
            for attachment in orphaned_attachments:
                db.session.delete(attachment)
            
            results['orphaned_attachments_removed'] = count
        
        if cleanup_type in ['all', 'old_versions']:
            # Clean up old document versions (keep last 10 per document)
            # This would require implementing the cleanup logic
            results['old_versions_cleaned'] = 0
        
        db.session.commit()
        
        return jsonify({
            'success': True,
            'message': 'Cleanup completed',
            'results': results
        })
        
    except Exception as e:
        logger.error(f"Error in system cleanup: {e}")
        db.session.rollback()
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/tags/merge', methods=['POST'])
@jwt_required()
def merge_tags():
    """Merge duplicate tags"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        data = request.get_json()
        source_tag_id = data.get('source_tag_id')
        target_tag_id = data.get('target_tag_id')
        
        if not source_tag_id or not target_tag_id:
            return jsonify({'error': 'Both source and target tag IDs are required'}), 400
        
        source_tag = Tag.query.get_or_404(source_tag_id)
        target_tag = Tag.query.get_or_404(target_tag_id)
        
        # Move all documents from source tag to target tag
        for document in source_tag.documents:
            if target_tag not in document.tags:
                document.tags.append(target_tag)
        
        # Delete the source tag
        db.session.delete(source_tag)
        db.session.commit()
        
        return jsonify({
            'success': True,
            'message': f'Tag "{source_tag.name}" merged into "{target_tag.name}"'
        })
        
    except Exception as e:
        logger.error(f"Error merging tags: {e}")
        db.session.rollback()
        return jsonify({'error': 'Internal server error'}), 500

@admin_bp.route('/admin/reports/activity', methods=['GET'])
@jwt_required()
def activity_report():
    """Generate activity report"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403
        
        days = request.args.get('days', 30, type=int)
        start_date = datetime.utcnow() - timedelta(days=days)
        
        # Daily activity data
        daily_activity = db.session.query(
            func.date(Document.created_at).label('date'),
            func.count(Document.id).label('documents'),
            func.count(Comment.id).label('comments')
        ).outerjoin(Comment, func.date(Comment.created_at) == func.date(Document.created_at))\
         .filter(Document.created_at >= start_date)\
         .group_by(func.date(Document.created_at))\
         .order_by('date').all()
        
        # Top users by activity
        top_users = db.session.query(
            User.username,
            func.count(Document.id).label('document_count')
        ).join(Document)\
         .filter(Document.created_at >= start_date)\
         .group_by(User.id, User.username)\
         .order_by(desc('document_count'))\
         .limit(10).all()
        
        return jsonify({
            'success': True,
            'report': {
                'period_days': days,
                'daily_activity': [
                    {
                        'date': activity.date.isoformat(),
                        'documents': activity.documents,
                        'comments': activity.comments or 0
                    }
                    for activity in daily_activity
                ],
                'top_users': [
                    {
                        'username': user.username,
                        'documents': user.document_count
                    }
                    for user in top_users
                ]
            }
        })
        
    except Exception as e:
        logger.error(f"Error generating activity report: {e}")
        return jsonify({'error': 'Internal server error'}), 500