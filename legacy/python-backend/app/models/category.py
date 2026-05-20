from app import db
from app.utils.datetime_utils import utc_now


class Category(db.Model):
    __tablename__ = 'categories'
    
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(100), nullable=False)
    slug = db.Column(db.String(100), unique=True, nullable=False)
    description = db.Column(db.Text)
    parent_id = db.Column(db.Integer, db.ForeignKey('categories.id'), nullable=True)
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)
    created_by = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=True)
    color = db.Column(db.String(7), default='#007bff')  # Hex color code
    sort_order = db.Column(db.Integer, default=0)
    is_active = db.Column(db.Boolean, default=True)
    
    # Relationships
    parent = db.relationship('Category', remote_side=[id], backref='children')
    creator = db.relationship('User', backref='created_categories')
    documents = db.relationship('Document', backref='category', lazy='dynamic')
    
    def __init__(self, name, description=None, parent_id=None, created_by=None, color='#007bff'):
        self.name = name
        self.slug = self.create_slug(name)
        self.description = description
        self.parent_id = parent_id
        self.created_by = created_by
        self.color = color
        
    @staticmethod
    def create_slug(name):
        """Create a URL-friendly slug from category name"""
        import re
        slug = re.sub(r'[^\w\s-]', '', name.lower())
        slug = re.sub(r'[-\s]+', '-', slug)
        return slug.strip('-')
    
    @classmethod
    def get_or_create(cls, name, parent_id=None, created_by=None, description=None, color='#007bff'):
        """Get existing category or create new one"""
        slug = cls.create_slug(name)
        category = cls.query.filter_by(slug=slug).first()
        
        if not category:
            category = cls(name=name, description=description, parent_id=parent_id, 
                         created_by=created_by, color=color)
            db.session.add(category)
            db.session.flush()  # Get the ID
        
        return category
    
    def get_full_path(self):
        """Get full hierarchical path as list of categories"""
        path = []
        current = self
        while current:
            path.insert(0, current)
            current = current.parent
        return path
    
    def get_path_string(self, separator=' > '):
        """Get full hierarchical path as string"""
        path = self.get_full_path()
        return separator.join([cat.name for cat in path])
    
    def get_all_descendants(self):
        """Get all descendant categories recursively"""
        descendants = []
        for child in self.children:
            descendants.append(child)
            descendants.extend(child.get_all_descendants())
        return descendants
    
    def get_document_count(self, include_descendants=True, _cache=None):
        """Get number of documents in this category (optimized)"""
        # Use cache if provided (for batch operations)
        if _cache is not None and self.id in _cache:
            return _cache[self.id]

        from app.models.document import Document

        if not include_descendants:
            return Document.query.filter_by(category_id=self.id).count()

        # Get all descendant IDs in a single query
        descendant_ids = [self.id] + [d.id for d in self.get_all_descendants()]
        return Document.query.filter(Document.category_id.in_(descendant_ids)).count()

    @classmethod
    def get_document_counts_bulk(cls):
        """Get document counts for all categories in a single query"""
        from app.models.document import Document
        from sqlalchemy import func

        # Single query to get counts per category
        counts = db.session.query(
            Document.category_id,
            func.count(Document.id).label('count')
        ).filter(
            Document.category_id.isnot(None)
        ).group_by(Document.category_id).all()

        return {cat_id: count for cat_id, count in counts}
    
    def get_level(self):
        """Get the depth level in the hierarchy (0 for root)"""
        level = 0
        current = self.parent
        while current:
            level += 1
            current = current.parent
        return level
    
    def is_ancestor_of(self, other_category):
        """Check if this category is an ancestor of another"""
        current = other_category.parent
        while current:
            if current.id == self.id:
                return True
            current = current.parent
        return False
    
    def can_have_parent(self, parent_id):
        """Check if this category can have the given parent (avoid circular references)"""
        if not parent_id:
            return True
        
        if parent_id == self.id:
            return False

        parent = db.session.get(Category, parent_id)
        if not parent:
            return False
        
        return not self.is_ancestor_of(parent)
    
    @classmethod
    def get_tree(cls, parent_id=None, include_inactive=False):
        """Get hierarchical tree structure"""
        query = cls.query
        
        if not include_inactive:
            query = query.filter_by(is_active=True)
        
        categories = query.filter_by(parent_id=parent_id).order_by(cls.sort_order, cls.name).all()
        
        tree = []
        for category in categories:
            node = {
                'category': category,
                'children': cls.get_tree(category.id, include_inactive)
            }
            tree.append(node)
        
        return tree
    
    @classmethod
    def get_flat_list(cls, include_inactive=False):
        """Get flat list of all categories with path information (optimized)"""
        query = cls.query

        if not include_inactive:
            query = query.filter_by(is_active=True)

        categories = query.order_by(cls.name).all()

        # Get all document counts in a single query
        doc_counts = cls.get_document_counts_bulk()

        # Build parent lookup for efficient path calculation
        cat_lookup = {cat.id: cat for cat in categories}

        result = []
        for category in categories:
            # Calculate level and path without additional queries
            level = 0
            path_parts = []
            current = category
            while current:
                path_parts.insert(0, current.name)
                level += 1 if current.parent_id else 0
                current = cat_lookup.get(current.parent_id)

            result.append({
                'id': category.id,
                'name': category.name,
                'path': ' > '.join(path_parts),
                'level': level,
                'document_count': doc_counts.get(category.id, 0)
            })

        return result
    
    def to_dict(self, include_children=False, include_documents=False, _doc_counts=None):
        """Convert category to dictionary (with optional pre-computed counts)"""
        result = {
            'id': self.id,
            'name': self.name,
            'slug': self.slug,
            'description': self.description,
            'parent_id': self.parent_id,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'created_by': self.created_by,
            'color': self.color,
            'sort_order': self.sort_order,
            'is_active': self.is_active,
            'level': self.get_level(),
            'path': self.get_path_string(),
            'document_count': _doc_counts.get(self.id, 0) if _doc_counts else self.get_document_count(include_descendants=False),
            'parent': {'id': self.parent.id, 'name': self.parent.name, 'slug': self.parent.slug} if self.parent else None
        }

        if include_children:
            result['children'] = [child.to_dict(_doc_counts=_doc_counts) for child in self.children]

        if include_documents:
            result['documents'] = [doc.to_dict() for doc in self.documents.limit(100)]

        return result
    
    def __repr__(self):
        return f'<Category {self.name}>'