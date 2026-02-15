from app import db
from app.utils.datetime_utils import utc_now


# Association table for many-to-many relationship between documents and tags
document_tags = db.Table('document_tags',
    db.Column('document_id', db.Integer, db.ForeignKey('documents.id'), primary_key=True),
    db.Column('tag_id', db.Integer, db.ForeignKey('tags.id'), primary_key=True),
    db.Column('created_at', db.DateTime, default=utc_now)
)


class Tag(db.Model):
    __tablename__ = 'tags'
    __table_args__ = (
        db.Index('idx_tags_created_at', 'created_at'),
        db.Index('idx_tags_created_by', 'created_by'),
    )

    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(50), unique=True, nullable=False)
    slug = db.Column(db.String(50), unique=True, nullable=False)
    description = db.Column(db.Text)
    color = db.Column(db.String(7), default='#007bff')  # Hex color code
    created_at = db.Column(db.DateTime, default=utc_now)
    created_by = db.Column(db.Integer, db.ForeignKey('users.id'))
    
    # Relationships
    creator = db.relationship('User', backref='created_tags')
    
    def __init__(self, name, description=None, color='#007bff', created_by=None):
        self.name = name.strip()
        self.slug = self.create_slug(name)
        self.description = description
        self.color = color
        self.created_by = created_by
    
    @staticmethod
    def create_slug(name):
        """Create a URL-friendly slug from tag name that supports Unicode"""
        import re
        
        slug = name.strip()
        
        # Remove leading # symbol for URL compatibility (but keep in name)
        slug = slug.lstrip('#')
        
        # Replace spaces and some special characters with hyphens
        slug = re.sub(r'[\s_/\\]+', '-', slug)
        
        # Remove characters that are problematic for URLs but preserve Unicode letters/numbers
        slug = re.sub(r'[^\w\-가-힣]', '-', slug, flags=re.UNICODE)
        
        # Clean up multiple hyphens
        slug = re.sub(r'-+', '-', slug)
        slug = slug.strip('-')
        
        # Convert to lowercase
        slug = slug.lower()
        
        return slug[:50] if slug else 'unnamed'  # Limit length with fallback
    
    @staticmethod
    def get_or_create(name, created_by=None):
        """Get existing tag or create new one"""
        slug = Tag.create_slug(name)
        tag = Tag.query.filter_by(slug=slug).first()
        if not tag:
            tag = Tag(name=name, created_by=created_by)
            db.session.add(tag)
        return tag
    
    @staticmethod
    def get_popular_tags(limit=20):
        """Get most popular tags by document count"""
        return db.session.query(Tag, db.func.count(document_tags.c.document_id).label('doc_count'))\
            .join(document_tags)\
            .group_by(Tag.id)\
            .order_by(db.desc('doc_count'))\
            .limit(limit)\
            .all()
    
    def get_document_count(self):
        """Get number of documents with this tag"""
        return db.session.query(db.func.count(document_tags.c.document_id))\
            .filter(document_tags.c.tag_id == self.id)\
            .scalar()
    
    def to_dict_lite(self):
        """Lightweight serialization without document count - for list views."""
        return {
            'id': self.id,
            'name': self.name,
            'slug': self.slug,
            'color': self.color,
        }

    def to_dict(self, document_count=None):
        """Full serialization with optional pre-computed document count.

        Args:
            document_count: Optional pre-computed count to avoid N+1 queries
        """
        return {
            'id': self.id,
            'name': self.name,
            'slug': self.slug,
            'description': self.description,
            'color': self.color,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'document_count': document_count if document_count is not None else self.get_document_count(),
            'creator': self.creator.to_dict() if self.creator else None
        }

    @staticmethod
    def get_tags_with_counts(tag_ids=None):
        """Get tags with document counts in a single query.

        Args:
            tag_ids: Optional list of tag IDs to filter

        Returns:
            Dict mapping tag_id to document_count
        """
        query = db.session.query(
            Tag.id,
            db.func.count(document_tags.c.document_id).label('doc_count')
        ).outerjoin(
            document_tags, Tag.id == document_tags.c.tag_id
        ).group_by(Tag.id)

        if tag_ids:
            query = query.filter(Tag.id.in_(tag_ids))

        results = query.all()
        return {tag_id: count for tag_id, count in results}

    def __repr__(self):
        return f'<Tag {self.name}>'