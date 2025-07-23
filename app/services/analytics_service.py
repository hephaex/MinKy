"""
Analytics Service for Minky Dashboard
Provides comprehensive analytics and reporting functionality
"""

from datetime import datetime, timedelta
from sqlalchemy import func, desc, and_
from app import db
from app.models.document import Document
from app.models.user import User
from app.models.tag import Tag, document_tags
from app.models.comment import Comment
from app.models.version import DocumentVersion
from app.models.attachment import Attachment
import logging

logger = logging.getLogger(__name__)

class AnalyticsService:
    """Service for generating analytics and insights"""
    
    @staticmethod
    def get_dashboard_stats():
        """Get comprehensive dashboard statistics"""
        try:
            # Basic counts
            total_documents = Document.query.count()
            total_users = User.query.count()
            total_tags = Tag.query.count()
            total_comments = Comment.query.count()
            
            # Recent activity (last 30 days)
            thirty_days_ago = datetime.utcnow() - timedelta(days=30)
            recent_documents = Document.query.filter(
                Document.created_at >= thirty_days_ago
            ).count()
            
            recent_comments = Comment.query.filter(
                Comment.created_at >= thirty_days_ago
            ).count()
            
            # Document statistics
            published_docs = Document.query.filter_by(is_public=True).count()
            private_docs = Document.query.filter_by(is_public=False).count()
            
            # Top tags
            top_tags = db.session.query(
                Tag.name,
                func.count(document_tags.c.document_id).label('usage_count')
            ).join(document_tags).group_by(Tag.id, Tag.name)\
             .order_by(desc('usage_count')).limit(10).all()
            
            # Most active users
            active_users = db.session.query(
                User.username,
                func.count(Document.id).label('document_count')
            ).join(Document, User.id == Document.user_id)\
             .group_by(User.id, User.username)\
             .order_by(desc('document_count')).limit(10).all()
            
            return {
                'overview': {
                    'total_documents': total_documents,
                    'total_users': total_users,
                    'total_tags': total_tags,
                    'total_comments': total_comments,
                    'published_documents': published_docs,
                    'private_documents': private_docs
                },
                'recent_activity': {
                    'documents_last_30_days': recent_documents,
                    'comments_last_30_days': recent_comments
                },
                'top_tags': [{'name': tag.name, 'count': tag.usage_count} for tag in top_tags],
                'active_users': [{'username': user.username, 'documents': user.document_count} for user in active_users]
            }
            
        except Exception as e:
            logger.error(f"Error generating dashboard stats: {e}")
            return None
    
    @staticmethod
    def get_document_activity_timeline(days=30):
        """Get document creation timeline for the last N days"""
        try:
            start_date = datetime.utcnow() - timedelta(days=days)
            
            # Get documents created each day
            daily_stats = db.session.query(
                func.date(Document.created_at).label('date'),
                func.count(Document.id).label('count')
            ).filter(
                Document.created_at >= start_date
            ).group_by(func.date(Document.created_at))\
             .order_by('date').all()
            
            return [{'date': stat.date.isoformat(), 'count': stat.count} for stat in daily_stats]
            
        except Exception as e:
            logger.error(f"Error generating activity timeline: {e}")
            return []
    
    @staticmethod
    def get_user_engagement_metrics():
        """Get user engagement metrics"""
        try:
            # Users with their document and comment counts
            engagement_data = db.session.query(
                User.username,
                func.count(Document.id).label('documents'),
                func.count(Comment.id).label('comments')
            ).outerjoin(Document, User.id == Document.user_id)\
             .outerjoin(Comment, User.id == Comment.user_id)\
             .group_by(User.id, User.username)\
             .having(func.count(Document.id) > 0)\
             .order_by(desc('documents')).all()
            
            return [{
                'username': user.username,
                'documents': user.documents,
                'comments': user.comments
            } for user in engagement_data]
            
        except Exception as e:
            logger.error(f"Error generating engagement metrics: {e}")
            return []
    
    @staticmethod
    def get_content_analytics():
        """Get content-related analytics"""
        try:
            # Average document length
            avg_length = db.session.query(
                func.avg(func.length(Document.markdown_content))
            ).scalar() or 0
            
            # Documents by tag distribution
            tag_distribution = db.session.query(
                Tag.name,
                func.count(document_tags.c.document_id).label('document_count')
            ).join(document_tags)\
             .group_by(Tag.id, Tag.name)\
             .order_by(desc('document_count')).all()
            
            # Version statistics
            total_versions = DocumentVersion.query.count()
            avg_versions_per_doc = db.session.query(
                func.avg(func.count(DocumentVersion.id))
            ).join(Document)\
             .group_by(Document.id).scalar() or 0
            
            # Attachment statistics
            total_attachments = Attachment.query.count()
            attachment_types = db.session.query(
                func.split_part(Attachment.filename, '.', -1).label('extension'),
                func.count(Attachment.id).label('count')
            ).group_by('extension')\
             .order_by(desc('count')).limit(10).all()
            
            return {
                'document_metrics': {
                    'average_length': round(avg_length, 2),
                    'total_versions': total_versions,
                    'avg_versions_per_document': round(avg_versions_per_doc, 2)
                },
                'tag_distribution': [{'name': tag.name, 'count': tag.document_count} for tag in tag_distribution],
                'attachment_stats': {
                    'total_attachments': total_attachments,
                    'file_types': [{'extension': att.extension, 'count': att.count} for att in attachment_types]
                }
            }
            
        except Exception as e:
            logger.error(f"Error generating content analytics: {e}")
            return {}
    
    @staticmethod
    def get_search_analytics():
        """Get search-related analytics (if search logging is implemented)"""
        # This would require implementing search query logging
        # For now, return placeholder data
        return {
            'popular_queries': [],
            'search_frequency': 0,
            'no_results_queries': []
        }
    
    @staticmethod
    def get_performance_metrics():
        """Get system performance metrics"""
        try:
            # Database size estimates
            doc_table_size = db.session.query(func.count(Document.id)).scalar()
            user_table_size = db.session.query(func.count(User.id)).scalar()
            
            # Growth metrics (last 7 days vs previous 7 days)
            seven_days_ago = datetime.utcnow() - timedelta(days=7)
            fourteen_days_ago = datetime.utcnow() - timedelta(days=14)
            
            recent_growth = Document.query.filter(
                Document.created_at >= seven_days_ago
            ).count()
            
            previous_growth = Document.query.filter(
                and_(
                    Document.created_at >= fourteen_days_ago,
                    Document.created_at < seven_days_ago
                )
            ).count()
            
            growth_rate = ((recent_growth - previous_growth) / max(previous_growth, 1)) * 100
            
            return {
                'database_stats': {
                    'documents': doc_table_size,
                    'users': user_table_size
                },
                'growth_metrics': {
                    'documents_last_7_days': recent_growth,
                    'documents_previous_7_days': previous_growth,
                    'growth_rate_percent': round(growth_rate, 2)
                }
            }
            
        except Exception as e:
            logger.error(f"Error generating performance metrics: {e}")
            return {}

# Convenience functions for common analytics
def get_comprehensive_analytics():
    """Get all analytics data in one call"""
    service = AnalyticsService()
    
    return {
        'dashboard_stats': service.get_dashboard_stats(),
        'activity_timeline': service.get_document_activity_timeline(),
        'user_engagement': service.get_user_engagement_metrics(),
        'content_analytics': service.get_content_analytics(),
        'search_analytics': service.get_search_analytics(),
        'performance_metrics': service.get_performance_metrics(),
        'generated_at': datetime.utcnow().isoformat()
    }