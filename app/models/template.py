from datetime import datetime
from app import db
import markdown

class DocumentTemplate(db.Model):
    __tablename__ = 'document_templates'
    
    id = db.Column(db.Integer, primary_key=True)
    name = db.Column(db.String(255), nullable=False)
    description = db.Column(db.Text)
    category = db.Column(db.String(100), default='General')
    title_template = db.Column(db.String(255), nullable=False)
    content_template = db.Column(db.Text, nullable=False)
    is_public = db.Column(db.Boolean, default=True)
    is_featured = db.Column(db.Boolean, default=False)
    created_by = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    usage_count = db.Column(db.Integer, default=0)
    
    # Relationships
    creator = db.relationship('User', backref='created_templates')
    
    def __init__(self, name, title_template, content_template, created_by, 
                 description=None, category='General', is_public=True):
        self.name = name
        self.title_template = title_template
        self.content_template = content_template
        self.created_by = created_by
        self.description = description
        self.category = category
        self.is_public = is_public
    
    def render_title(self, variables=None):
        """Render title template with variables"""
        if not variables:
            return self.title_template
        
        try:
            return self.title_template.format(**variables)
        except KeyError:
            return self.title_template
    
    def render_content(self, variables=None):
        """Render content template with variables"""
        if not variables:
            return self.content_template
        
        try:
            return self.content_template.format(**variables)
        except KeyError:
            return self.content_template
    
    def get_template_variables(self):
        """Extract template variables from title and content"""
        import re
        
        # Find all {variable} patterns
        title_vars = re.findall(r'\{([^}]+)\}', self.title_template)
        content_vars = re.findall(r'\{([^}]+)\}', self.content_template)
        
        # Remove duplicates and sort
        all_vars = sorted(list(set(title_vars + content_vars)))
        
        return all_vars
    
    def create_document(self, variables=None, author=None, user_id=None, tags=None):
        """Create a new document from this template"""
        from app.models.document import Document
        
        rendered_title = self.render_title(variables)
        rendered_content = self.render_content(variables)
        
        document = Document(
            title=rendered_title,
            markdown_content=rendered_content,
            author=author,
            user_id=user_id,
            is_public=True  # Default to public, can be changed later
        )
        
        # Add tags if provided
        if tags:
            document.add_tags(tags)
        
        # Increment usage count
        self.usage_count += 1
        
        return document
    
    @staticmethod
    def get_popular_templates(limit=10):
        """Get most popular templates by usage"""
        return DocumentTemplate.query.filter_by(is_public=True)\
            .order_by(DocumentTemplate.usage_count.desc())\
            .limit(limit).all()
    
    @staticmethod
    def get_featured_templates():
        """Get featured templates"""
        return DocumentTemplate.query.filter_by(is_public=True, is_featured=True)\
            .order_by(DocumentTemplate.updated_at.desc()).all()
    
    @staticmethod
    def get_by_category(category):
        """Get templates by category"""
        return DocumentTemplate.query.filter_by(category=category, is_public=True)\
            .order_by(DocumentTemplate.name).all()
    
    @staticmethod
    def get_categories():
        """Get all available categories"""
        categories = db.session.query(DocumentTemplate.category.distinct())\
            .filter_by(is_public=True).all()
        return [cat[0] for cat in categories]
    
    def can_edit(self, user_id):
        """Check if user can edit this template"""
        return self.created_by == user_id
    
    def can_delete(self, user_id):
        """Check if user can delete this template"""
        return self.created_by == user_id
    
    def to_dict(self, include_content=True):
        data = {
            'id': self.id,
            'name': self.name,
            'description': self.description,
            'category': self.category,
            'is_public': self.is_public,
            'is_featured': self.is_featured,
            'created_by': self.created_by,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None,
            'usage_count': self.usage_count,
            'creator': self.creator.to_dict() if self.creator else None,
            'template_variables': self.get_template_variables()
        }
        
        if include_content:
            data.update({
                'title_template': self.title_template,
                'content_template': self.content_template
            })
        
        return data
    
    def __repr__(self):
        return f'<DocumentTemplate {self.name}>'