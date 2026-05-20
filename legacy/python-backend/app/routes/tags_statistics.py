"""Tag statistics endpoints."""
from flask import Blueprint, Response
from sqlalchemy import func
from app import db, cache, limiter
from app.models.tag import Tag
from app.models.document import Document
from app.utils.responses import success_response, error_response
import logging

logger = logging.getLogger(__name__)

tags_statistics_bp = Blueprint('tags_statistics', __name__)


@tags_statistics_bp.route('/tags/statistics', methods=['GET'])
@limiter.limit("60 per minute")  # SECURITY: Rate limiting
@cache.cached(timeout=60)
def get_tags_statistics() -> Response | tuple[Response, int]:
    """Get comprehensive tag statistics."""
    try:
        # Total tags
        total_tags = Tag.query.count()

        # Most popular tags
        popular_tags_data = Tag.get_popular_tags(limit=10)
        popular_list = [
            {'name': tag.name, 'usage_count': count, 'color': tag.color}
            for tag, count in popular_tags_data
        ]

        # Recently created tags
        recent_tags = Tag.query.order_by(Tag.created_at.desc()).limit(5).all()
        recent_list = [
            {'name': tag.name, 'created_at': tag.created_at.isoformat(), 'color': tag.color}
            for tag in recent_tags
        ]

        # Tag usage distribution - single query with LEFT JOIN instead of N+1
        tag_counts = db.session.query(
            Tag.id,
            func.count(Document.id).label('doc_count')
        ).outerjoin(
            Tag.documents
        ).group_by(Tag.id).all()

        usage_distribution = {'unused': 0, 'low': 0, 'medium': 0, 'high': 0}
        for _, doc_count in tag_counts:
            if doc_count == 0:
                usage_distribution['unused'] += 1
            elif doc_count <= 5:
                usage_distribution['low'] += 1
            elif doc_count <= 20:
                usage_distribution['medium'] += 1
            else:
                usage_distribution['high'] += 1

        # Auto-generated tags (tags without description)
        auto_generated_count = Tag.query.filter(Tag.description.is_(None)).count()

        return success_response({
            'total_tags': total_tags,
            'auto_generated_tags': auto_generated_count,
            'manual_tags': total_tags - auto_generated_count,
            'popular_tags': popular_list,
            'recent_tags': recent_list,
            'usage_distribution': usage_distribution
        })

    except Exception as e:
        logger.error("Error getting tag statistics: %s", e)
        return error_response('Internal server error', 500)
