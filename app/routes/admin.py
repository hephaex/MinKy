"""
Admin Panel API Routes
Provides comprehensive administrative functionality
"""

from flask import Blueprint, jsonify, request
from flask_jwt_extended import jwt_required
from sqlalchemy import func, desc, and_
from datetime import datetime, timedelta, timezone
from app import db
from app.models.user import User
from app.models.document import Document
from app.models.tag import Tag
from app.models.comment import Comment
from app.models.attachment import Attachment
from app.utils.auth import get_current_user
from app.utils.responses import paginate_query
from app.utils.validation import escape_like
import logging

logger = logging.getLogger(__name__)

admin_bp = Blueprint('admin', __name__)

def require_admin():
    """Check if current user is admin"""
    user = get_current_user()
    return user and user.is_admin

@admin_bp.route('/admin/users', methods=['GET'])
@jwt_required()
def list_users():
    """List all users with pagination - optimized with single query aggregation"""
    try:
        if not require_admin():
            return jsonify({'error': 'Admin access required'}), 403

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 20, type=int)
        search = request.args.get('search', '')

        # Subqueries for counts (executed once, not N times)
        doc_count_subq = db.session.query(
            Document.user_id,
            func.count(Document.id).label('doc_count')
        ).group_by(Document.user_id).subquery()

        comment_count_subq = db.session.query(
            Comment.user_id,
            func.count(Comment.id).label('comment_count')
        ).group_by(Comment.user_id).subquery()

        # Main query with left joins to subqueries
        query = db.session.query(
            User,
            func.coalesce(doc_count_subq.c.doc_count, 0).label('document_count'),
            func.coalesce(comment_count_subq.c.comment_count, 0).label('comment_count')
        ).outerjoin(
            doc_count_subq, User.id == doc_count_subq.c.user_id
        ).outerjoin(
            comment_count_subq, User.id == comment_count_subq.c.user_id
        )

        if search:
            search_escaped = escape_like(search)
            query = query.filter(
                User.username.ilike(f'%{search_escaped}%') |
                User.email.ilike(f'%{search_escaped}%') |
                User.full_name.ilike(f'%{search_escaped}%')
            )

        query = query.order_by(desc(User.created_at))

        # Manual pagination for tuple results
        total = query.count()
        items = query.offset((page - 1) * per_page).limit(per_page).all()

        users = []
        for user, doc_count, comment_count in items:
            user_data = user.to_dict(include_sensitive=True)
            user_data['document_count'] = doc_count
            user_data['comment_count'] = comment_count
            users.append(user_data)

        return jsonify({
            'success': True,
            'users': users,
            'pagination': {
                'page': page,
                'per_page': per_page,
                'total': total,
                'pages': (total + per_page - 1) // per_page
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
        
        # Add detailed stats - use count() instead of loading all records
        document_count = Document.query.filter_by(user_id=user.id).count()
        comment_count = Comment.query.filter_by(user_id=user.id).count()

        # Recent activity count
        recent_docs = Document.query.filter(
            and_(
                Document.user_id == user.id,
                Document.created_at >= datetime.now(timezone.utc) - timedelta(days=30)
            )
        ).count()

        # Get only the last 5 documents for recent activity
        recent_activity_docs = Document.query.filter_by(user_id=user.id)\
            .order_by(Document.created_at.desc())\
            .limit(5)\
            .all()

        user_data.update({
            'document_count': document_count,
            'comment_count': comment_count,
            'recent_documents': recent_docs,
            'recent_activity': [
                {
                    'type': 'document',
                    'title': doc.title,
                    'created_at': doc.created_at.isoformat() if doc.created_at else None
                }
                for doc in recent_activity_docs
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
        
        user.updated_at = datetime.now(timezone.utc)
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
        
        def serialize_doc_with_owner(doc):
            doc_data = doc.to_dict()
            if doc.owner:
                doc_data['owner'] = {
                    'id': doc.owner.id,
                    'username': doc.owner.username,
                    'full_name': doc.owner.full_name
                }
            return doc_data

        query = query.order_by(desc(Document.created_at))
        return paginate_query(
            query, page, per_page,
            serializer_func=serialize_doc_with_owner,
            items_key='documents',
            extra_fields={'success': True}
        )
        
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
        week_ago = datetime.now(timezone.utc) - timedelta(days=7)
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
        start_date = datetime.now(timezone.utc) - timedelta(days=days)
        
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