from datetime import datetime
from app import db
from sqlalchemy import func


class Category(db.Model):
    __tablename__ = 'categories'
    
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(100), nullable=False)
    slug = db.Column(db.String(100), unique=True, nullable=False)
    description = db.Column(db.Text)
    parent_id = db.Column(db.Integer, db.ForeignKey('categories.id'), nullable=True)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    created_by = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=True)
    color = db.Column(db.String(7), default='#007bff')  # Hex color code
    sort_order = db.Column(db.Integer, default=0)
    is_active = db.Column(db.Boolean, default=True)
    
    # Relationships
    parent = db.relationship('Category', remote_side=[id], backref='children')
    creator = db.relationship('User', backref='created_categories')
    
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
    
    def get_document_count(self, include_descendants=True):
        """Get number of documents in this category"""
        count = self.documents.count()
        
        if include_descendants:
            for child in self.children:
                count += child.get_document_count(include_descendants=True)
        
        return count
    
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
        
        parent = Category.query.get(parent_id)
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
        """Get flat list of all categories with path information"""
        query = cls.query
        
        if not include_inactive:
            query = query.filter_by(is_active=True)
        
        categories = query.order_by(cls.name).all()
        
        result = []
        for category in categories:
            result.append({
                'id': category.id,
                'name': category.name,
                'path': category.get_path_string(),
                'level': category.get_level(),
                'document_count': category.get_document_count()
            })
        
        return result
    
    def to_dict(self, include_children=False, include_documents=False):
        """Convert category to dictionary"""
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
            'document_count': self.get_document_count(),
            'parent': self.parent.to_dict() if self.parent else None
        }
        
        if include_children:
            result['children'] = [child.to_dict() for child in self.children]
        
        if include_documents:
            result['documents'] = [doc.to_dict() for doc in self.documents]
        
        return result
    
    def __repr__(self):
        return f'<Category {self.name}>'