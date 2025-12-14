from datetime import datetime, timezone
from app import db


def utc_now():
    """Return current UTC time as timezone-aware datetime."""
    return datetime.now(timezone.utc)


class Comment(db.Model):
    __tablename__ = 'comments'
    
    id = db.Column(db.Integer, primary_key=True)
    content = db.Column(db.Text, nullable=False)
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=False)
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    parent_id = db.Column(db.Integer, db.ForeignKey('comments.id'), nullable=True)  # For nested comments
    is_deleted = db.Column(db.Boolean, default=False)
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)
    
    # Relationships
    author = db.relationship('User', backref='comments')
    document = db.relationship('Document', backref='comments')
    parent = db.relationship('Comment', remote_side=[id], backref='replies')
    
    def __init__(self, content, document_id, user_id, parent_id=None):
        self.content = content
        self.document_id = document_id
        self.user_id = user_id
        self.parent_id = parent_id
    
    def can_edit(self, user_id):
        return self.user_id == user_id and not self.is_deleted
    
    def can_delete(self, user_id):
        return self.user_id == user_id and not self.is_deleted
    
    def soft_delete(self):
        self.is_deleted = True
        self.content = "[This comment has been deleted]"
        self.updated_at = datetime.now(timezone.utc)
    
    def get_replies(self):
        return Comment.query.filter_by(parent_id=self.id, is_deleted=False)\
            .order_by(Comment.created_at.asc()).all()
    
    def to_dict(self, include_replies=True):
        data = {
            'id': self.id,
            'content': self.content,
            'document_id': self.document_id,
            'user_id': self.user_id,
            'parent_id': self.parent_id,
            'is_deleted': self.is_deleted,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'author': self.author.to_dict() if self.author else None
        }
        
        if include_replies and not self.parent_id:  # Only include replies for top-level comments
            data['replies'] = [reply.to_dict(include_replies=False) for reply in self.get_replies()]
        
        return data
    
    def __repr__(self):
        return f'<Comment {self.id} by {self.author.username if self.author else "Unknown"}>'

class Rating(db.Model):
    __tablename__ = 'ratings'
    
    id = db.Column(db.Integer, primary_key=True)
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=False)
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    rating = db.Column(db.Integer, nullable=False)  # 1-5 stars
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)
    
    # Relationships
    user = db.relationship('User', backref='ratings')
    document = db.relationship('Document', backref='ratings')
    
    # Unique constraint to prevent multiple ratings from same user
    __table_args__ = (db.UniqueConstraint('document_id', 'user_id', name='unique_user_document_rating'),)
    
    def __init__(self, document_id, user_id, rating):
        self.document_id = document_id
        self.user_id = user_id
        self.rating = max(1, min(5, rating))  # Ensure rating is between 1-5
    
    def to_dict(self):
        return {
            'id': self.id,
            'document_id': self.document_id,
            'user_id': self.user_id,
            'rating': self.rating,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'user': self.user.to_dict() if self.user else None
        }
    
    @staticmethod
    def get_document_rating_stats(document_id):
        """Get rating statistics for a document"""
        ratings = db.session.query(Rating.rating).filter_by(document_id=document_id).all()
        
        if not ratings:
            return {
                'average_rating': 0,
                'total_ratings': 0,
                'rating_distribution': {1: 0, 2: 0, 3: 0, 4: 0, 5: 0}
            }
        
        rating_values = [r.rating for r in ratings]
        average_rating = sum(rating_values) / len(rating_values)
        
        distribution = {1: 0, 2: 0, 3: 0, 4: 0, 5: 0}
        for rating in rating_values:
            distribution[rating] += 1
        
        return {
            'average_rating': round(average_rating, 2),
            'total_ratings': len(rating_values),
            'rating_distribution': distribution
        }
    
    def __repr__(self):
        return f'<Rating {self.rating}/5 by {self.user.username if self.user else "Unknown"}>'