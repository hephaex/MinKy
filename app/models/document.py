from app import db
from app.utils.datetime_utils import utc_now
import markdown
import bleach
from app.models.tag import document_tags


def escape_like_pattern(value: str) -> str:
    """Escape special characters for SQL LIKE/ILIKE queries."""
    if not value:
        return value
    return value.replace('\\', '\\\\').replace('%', '\\%').replace('_', '\\_')

# Allowed HTML tags for sanitized markdown output
ALLOWED_TAGS = [
    'p', 'br', 'strong', 'em', 'b', 'i', 'u', 's',
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
    'ul', 'ol', 'li', 'dl', 'dt', 'dd',
    'a', 'img', 'code', 'pre', 'blockquote',
    'table', 'thead', 'tbody', 'tr', 'th', 'td',
    'hr', 'div', 'span', 'sup', 'sub', 'mark'
]

ALLOWED_ATTRIBUTES = {
    'a': ['href', 'title', 'target', 'rel'],
    'img': ['src', 'alt', 'title', 'width', 'height'],
    'code': ['class'],
    'pre': ['class'],
    'span': ['class'],
    'div': ['class'],
    'td': ['colspan', 'rowspan'],
    'th': ['colspan', 'rowspan']
}


class Document(db.Model):
    __tablename__ = 'documents'
    __table_args__ = (
        db.Index('idx_documents_user_id', 'user_id'),
        db.Index('idx_documents_is_public', 'is_public'),
        db.Index('idx_documents_user_visibility', 'user_id', 'is_public'),
        db.Index('idx_documents_updated_at', 'updated_at'),
        db.Index('idx_documents_created_at', 'created_at'),
        db.Index('idx_documents_category_id', 'category_id'),
    )

    id = db.Column(db.Integer, primary_key=True)
    title = db.Column(db.String(255), nullable=False)
    author = db.Column(db.String(255))
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)
    markdown_content = db.Column(db.Text, nullable=False)
    html_content = db.Column(db.Text)
    search_vector = db.Column(db.Text)
    user_id = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=True)
    category_id = db.Column(db.Integer, db.ForeignKey('categories.id'), nullable=True)
    is_public = db.Column(db.Boolean, default=True)
    is_published = db.Column(db.Boolean, default=False)
    published_at = db.Column(db.DateTime, nullable=True)
    document_metadata = db.Column(db.JSON, nullable=True)  # 확장 가능한 메타데이터 저장
    
    # Relationships
    # Use 'selectin' for efficient batch loading when accessing tags
    tags = db.relationship('Tag', secondary=document_tags, backref=db.backref('documents', lazy='dynamic'), lazy='selectin')
    
    def __init__(self, title, markdown_content, author=None, user_id=None, is_public=True, document_metadata=None):
        self.title = title
        self.markdown_content = markdown_content
        self.author = author
        self.user_id = user_id
        self.is_public = is_public
        self.document_metadata = document_metadata
        self.html_content = self.convert_markdown_to_html()
    
    def convert_markdown_to_html(self):
        """Convert markdown to sanitized HTML to prevent XSS attacks"""
        raw_html = markdown.markdown(
            self.markdown_content,
            extensions=['tables', 'fenced_code', 'codehilite']
        )
        return bleach.clean(
            raw_html,
            tags=ALLOWED_TAGS,
            attributes=ALLOWED_ATTRIBUTES,
            strip=True
        )
    
    def update_content(self, title=None, markdown_content=None, author=None, create_version=True, change_summary=None, updated_by=None):
        # Check if content actually changed
        content_changed = False
        
        if title and title != self.title:
            content_changed = True
        if markdown_content and markdown_content != self.markdown_content:
            content_changed = True
        if author and author != self.author:
            content_changed = True
        
        # Create version before updating if content changed
        if create_version and content_changed:
            self.create_version(change_summary=change_summary, created_by=updated_by)
        
        # Update content
        if title:
            self.title = title
        if markdown_content:
            self.markdown_content = markdown_content
            self.html_content = self.convert_markdown_to_html()
        if author:
            self.author = author
        self.updated_at = utc_now()
    
    def create_version(self, change_summary=None, created_by=None):
        """Create a version of current document state"""
        from app.models.version import DocumentVersion, DocumentSnapshot
        
        version = DocumentVersion.create_version(self, change_summary, created_by)
        db.session.add(version)
        
        # Create snapshot if needed
        if DocumentSnapshot.should_create_snapshot(version.version_number):
            snapshot = DocumentSnapshot.create_snapshot(version)
            db.session.add(snapshot)
        
        return version
    
    def get_version_count(self):
        """Get total number of versions for this document"""
        from app.models.version import DocumentVersion
        return DocumentVersion.query.filter_by(document_id=self.id).count()
    
    def get_latest_version_number(self):
        """Get the latest version number"""
        from app.models.version import DocumentVersion
        latest = DocumentVersion.query.filter_by(document_id=self.id)\
            .order_by(DocumentVersion.version_number.desc()).first()
        return latest.version_number if latest else 0
    
    @classmethod
    def search_documents(cls, query_text, page=1, per_page=10, user_id=None, include_private=False, tags=None):
        base_query = cls.query
        
        # Filter by visibility
        if not include_private:
            base_query = base_query.filter(cls.is_public == True)
        elif user_id:
            # Include public documents and user's private documents
            base_query = base_query.filter(
                db.or_(cls.is_public == True, cls.user_id == user_id)
            )
        
        # Filter by tags if provided
        if tags:
            from app.models.tag import Tag
            if isinstance(tags, str):
                tags = [tags]
            tag_objects = Tag.query.filter(Tag.slug.in_(tags)).all()
            if tag_objects:
                for tag in tag_objects:
                    base_query = base_query.filter(cls.tags.contains(tag))
        
        if not query_text:
            return base_query.order_by(cls.updated_at.desc()).paginate(
                page=page, per_page=per_page, error_out=False
            )
        
        # Use simple ILIKE search instead of full-text search for better compatibility
        query_escaped = escape_like_pattern(query_text)
        search_query = base_query.filter(
            db.or_(
                cls.title.ilike(f'%{query_escaped}%'),
                cls.markdown_content.ilike(f'%{query_escaped}%')
            )
        ).order_by(cls.updated_at.desc())
        
        return search_query.paginate(page=page, per_page=per_page, error_out=False)
    
    def add_tags(self, tag_names):
        """Add tags to document by name"""
        from app.models.tag import Tag
        if isinstance(tag_names, str):
            tag_names = [tag_names]
        
        for tag_name in tag_names:
            tag_name = tag_name.strip()
            if tag_name:
                tag = Tag.get_or_create(tag_name, created_by=self.user_id)
                if tag not in self.tags:
                    self.tags.append(tag)
    
    def remove_tag(self, tag_name):
        """Remove tag from document by name"""
        from app.models.tag import Tag
        tag = Tag.query.filter_by(slug=Tag.create_slug(tag_name)).first()
        if tag and tag in self.tags:
            self.tags.remove(tag)
    
    def get_tag_names(self):
        """Get list of tag names for this document"""
        return [tag.name for tag in self.tags]
    
    def get_comment_count(self):
        """Get number of comments for this document"""
        from app.models.comment import Comment
        return Comment.query.filter_by(document_id=self.id, is_deleted=False).count()
    
    def get_rating_stats(self):
        """Get rating statistics for this document"""
        from app.models.comment import Rating
        return Rating.get_document_rating_stats(self.id)
    
    def can_edit(self, user_id):
        """Check if user can edit this document.

        Note: user_id=None means unauthenticated - they cannot edit any document.
        """
        if user_id is None:
            return False
        return self.user_id == user_id

    def can_view(self, user_id):
        """Check if user can view this document.

        Public documents can be viewed by anyone (including unauthenticated).
        Private documents can only be viewed by their owner.
        """
        if self.is_public:
            return True
        if user_id is None:
            return False
        return self.user_id == user_id
    
    def to_dict_lite(self):
        """Lightweight serialization for list views - avoids N+1 queries.

        Use this for paginated lists where full stats aren't needed.
        """
        return {
            'id': self.id,
            'title': self.title,
            'author': self.author,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'user_id': self.user_id,
            'category_id': self.category_id,
            'is_public': self.is_public,
            'is_published': self.is_published,
            'published_at': self.published_at.isoformat() if self.published_at else None,
            'tag_names': [tag.name for tag in self.tags],
        }

    def to_dict(self, include_stats=True, stats=None):
        """Full serialization with optional pre-computed stats.

        Args:
            include_stats: If False, skip expensive stat queries
            stats: Optional dict with pre-computed stats:
                   {'comment_count': N, 'version_count': N, 'latest_version': N, 'rating_stats': {...}}
        """
        result = {
            'id': self.id,
            'title': self.title,
            'author': self.author,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'markdown_content': self.markdown_content,
            'html_content': self.html_content,
            'user_id': self.user_id,
            'category_id': self.category_id,
            'category': {
                'id': self.category.id,
                'name': self.category.name,
                'slug': self.category.slug,
                'color': self.category.color
            } if self.category else None,
            'is_public': self.is_public,
            'is_published': self.is_published,
            'published_at': self.published_at.isoformat() if self.published_at else None,
            'metadata': self.document_metadata,
            'owner': self.owner.to_dict() if self.owner else None,
            'tags': [{'id': t.id, 'name': t.name, 'slug': t.slug, 'color': t.color} for t in self.tags],
            'tag_names': [tag.name for tag in self.tags],
        }

        if include_stats:
            if stats:
                # Use pre-computed stats
                result['comment_count'] = stats.get('comment_count', 0)
                result['rating_stats'] = stats.get('rating_stats', {'average': 0, 'count': 0})
                result['version_count'] = stats.get('version_count', 0)
                result['latest_version'] = stats.get('latest_version', 0)
            else:
                # Compute stats (expensive - avoid in list views)
                result['comment_count'] = self.get_comment_count()
                result['rating_stats'] = self.get_rating_stats()
                result['version_count'] = self.get_version_count()
                result['latest_version'] = self.get_latest_version_number()

        return result