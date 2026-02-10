from flask import Blueprint, request
from flask_jwt_extended import jwt_required, get_jwt_identity
from app import db, limiter
from app.models.comment import Comment, Rating
from app.models.document import Document
from app.utils.auth import get_current_user_id
from app.utils.responses import paginate_query, success_response, error_response
import bleach
import logging

logger = logging.getLogger(__name__)

comments_bp = Blueprint('comments', __name__)

@comments_bp.route('/documents/<int:document_id>/comments', methods=['GET'])
def get_comments(document_id):
    """Get comments for a document"""
    try:
        document = Document.query.get_or_404(document_id)

        # Check if user can view document
        current_user_id = get_current_user_id()
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        page = request.args.get('page', 1, type=int)
        per_page = request.args.get('per_page', 10, type=int)

        query = Comment.query.filter_by(
            document_id=document_id,
            parent_id=None,
            is_deleted=False
        ).order_by(Comment.created_at.desc())

        return paginate_query(
            query, page, per_page,
            serializer_func=lambda c: c.to_dict(),
            items_key='comments'
        )

    except Exception as e:
        logger.error("Error getting comments for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/documents/<int:document_id>/comments', methods=['POST'])
@limiter.limit("30 per hour")
@jwt_required()
def create_comment(document_id):
    """Create a new comment on a document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_jwt_identity()

        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data or 'content' not in data:
            return error_response('Comment content is required', 400)

        content = bleach.clean(data['content'].strip())
        parent_id = data.get('parent_id')

        if not content:
            return error_response('Comment content cannot be empty', 400)

        # Validate parent comment if provided
        if parent_id:
            parent_comment = Comment.query.filter_by(
                id=parent_id,
                document_id=document_id,
                is_deleted=False
            ).first()
            if not parent_comment:
                return error_response('Parent comment not found', 404)

        comment = Comment(
            content=content,
            document_id=document_id,
            user_id=current_user_id,
            parent_id=parent_id
        )

        db.session.add(comment)
        db.session.commit()

        return success_response(comment.to_dict(include_replies=False), status_code=201)

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating comment for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/comments/<int:comment_id>', methods=['PUT'])
@limiter.limit("60 per hour")
@jwt_required()
def update_comment(comment_id):
    """Update a comment"""
    try:
        comment = Comment.query.get_or_404(comment_id)
        current_user_id = get_jwt_identity()

        if not comment.can_edit(current_user_id):
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data or 'content' not in data:
            return error_response('Comment content is required', 400)

        content = bleach.clean(data['content'].strip())
        if not content:
            return error_response('Comment content cannot be empty', 400)

        comment.content = content
        comment.updated_at = db.func.now()

        db.session.commit()

        return success_response(comment.to_dict(include_replies=False))

    except Exception as e:
        db.session.rollback()
        logger.error("Error updating comment %s: %s", comment_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/comments/<int:comment_id>', methods=['DELETE'])
@limiter.limit("30 per hour")
@jwt_required()
def delete_comment(comment_id):
    """Delete a comment (soft delete)"""
    try:
        comment = Comment.query.get_or_404(comment_id)
        current_user_id = get_jwt_identity()

        if not comment.can_delete(current_user_id):
            return error_response('Access denied', 403)

        comment.soft_delete()
        db.session.commit()

        return success_response(message='Comment deleted successfully')

    except Exception as e:
        db.session.rollback()
        logger.error("Error deleting comment %s: %s", comment_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/documents/<int:document_id>/rating', methods=['POST'])
@jwt_required()
def rate_document(document_id):
    """Rate a document"""
    try:
        document = Document.query.get_or_404(document_id)
        current_user_id = get_jwt_identity()

        # Check if user can view document
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        data = request.get_json()
        if not data or 'rating' not in data:
            return error_response('Rating is required', 400)

        rating_value = data['rating']
        if not isinstance(rating_value, int) or rating_value < 1 or rating_value > 5:
            return error_response('Rating must be between 1 and 5', 400)

        # Check if user already rated this document
        existing_rating = Rating.query.filter_by(
            document_id=document_id,
            user_id=current_user_id
        ).first()

        if existing_rating:
            # Update existing rating
            existing_rating.rating = rating_value
            existing_rating.updated_at = db.func.now()
            rating = existing_rating
        else:
            # Create new rating
            rating = Rating(
                document_id=document_id,
                user_id=current_user_id,
                rating=rating_value
            )
            db.session.add(rating)

        db.session.commit()

        # Return updated rating stats
        rating_stats = Rating.get_document_rating_stats(document_id)

        return success_response({
            'rating': rating.to_dict(),
            'document_rating_stats': rating_stats
        })

    except Exception as e:
        db.session.rollback()
        logger.error("Error rating document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/documents/<int:document_id>/rating', methods=['GET'])
def get_document_rating(document_id):
    """Get rating statistics for a document"""
    try:
        document = Document.query.get_or_404(document_id)

        # Check if user can view document
        current_user_id = get_current_user_id()
        if not document.can_view(current_user_id):
            return error_response('Access denied', 403)

        rating_stats = Rating.get_document_rating_stats(document_id)

        # Get current user's rating if authenticated
        user_rating = None
        if current_user_id:
            user_rating_obj = Rating.query.filter_by(
                document_id=document_id,
                user_id=current_user_id
            ).first()
            if user_rating_obj:
                user_rating = user_rating_obj.rating

        return success_response({
            'rating_stats': rating_stats,
            'user_rating': user_rating
        })

    except Exception as e:
        logger.error("Error getting rating for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)

@comments_bp.route('/documents/<int:document_id>/rating', methods=['DELETE'])
@jwt_required()
def remove_rating(document_id):
    """Remove user's rating from a document"""
    try:
        current_user_id = get_jwt_identity()

        rating = Rating.query.filter_by(
            document_id=document_id,
            user_id=current_user_id
        ).first()

        if not rating:
            return error_response('No rating found', 404)

        db.session.delete(rating)
        db.session.commit()

        # Return updated rating stats
        rating_stats = Rating.get_document_rating_stats(document_id)

        return success_response({
            'message': 'Rating removed successfully',
            'document_rating_stats': rating_stats
        })

    except Exception as e:
        db.session.rollback()
        logger.error("Error removing rating for document %s: %s", document_id, e)
        return error_response('Internal server error', 500)