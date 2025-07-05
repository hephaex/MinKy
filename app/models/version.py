from datetime import datetime
from app import db
import hashlib
import difflib

class DocumentVersion(db.Model):
    __tablename__ = 'document_versions'
    
    id = db.Column(db.Integer, primary_key=True)
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=False)
    version_number = db.Column(db.Integer, nullable=False)
    title = db.Column(db.String(255), nullable=False)
    markdown_content = db.Column(db.Text, nullable=False)
    html_content = db.Column(db.Text)
    author = db.Column(db.String(255))
    content_hash = db.Column(db.String(64), nullable=False)  # SHA-256 hash
    change_summary = db.Column(db.Text)  # Optional summary of changes
    created_by = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    
    # Relationships
    document = db.relationship('Document', backref='versions')
    creator = db.relationship('User', backref='created_versions')
    
    def __init__(self, document_id, version_number, title, markdown_content, html_content, 
                 author=None, change_summary=None, created_by=None):
        self.document_id = document_id
        self.version_number = version_number
        self.title = title
        self.markdown_content = markdown_content
        self.html_content = html_content
        self.author = author
        self.change_summary = change_summary
        self.created_by = created_by
        self.content_hash = self.generate_content_hash()
    
    def generate_content_hash(self):
        """Generate SHA-256 hash of the content"""
        content = f"{self.title}|{self.markdown_content}|{self.author or ''}"
        return hashlib.sha256(content.encode('utf-8')).hexdigest()
    
    @staticmethod
    def create_version(document, change_summary=None, created_by=None):
        """Create a new version from a document"""
        # Get the next version number
        last_version = DocumentVersion.query.filter_by(document_id=document.id)\
            .order_by(DocumentVersion.version_number.desc()).first()
        
        next_version = (last_version.version_number + 1) if last_version else 1
        
        version = DocumentVersion(
            document_id=document.id,
            version_number=next_version,
            title=document.title,
            markdown_content=document.markdown_content,
            html_content=document.html_content,
            author=document.author,
            change_summary=change_summary,
            created_by=created_by or document.user_id
        )
        
        return version
    
    def get_diff_from_previous(self):
        """Get diff from previous version"""
        previous_version = DocumentVersion.query.filter_by(document_id=self.document_id)\
            .filter(DocumentVersion.version_number < self.version_number)\
            .order_by(DocumentVersion.version_number.desc()).first()
        
        if not previous_version:
            return None
        
        # Generate diff for title
        title_diff = list(difflib.unified_diff(
            previous_version.title.splitlines(keepends=True),
            self.title.splitlines(keepends=True),
            fromfile=f"v{previous_version.version_number}/title",
            tofile=f"v{self.version_number}/title",
            lineterm=''
        ))
        
        # Generate diff for content
        content_diff = list(difflib.unified_diff(
            previous_version.markdown_content.splitlines(keepends=True),
            self.markdown_content.splitlines(keepends=True),
            fromfile=f"v{previous_version.version_number}/content",
            tofile=f"v{self.version_number}/content",
            lineterm=''
        ))
        
        return {
            'previous_version': previous_version.version_number,
            'current_version': self.version_number,
            'title_diff': title_diff,
            'content_diff': content_diff,
            'has_changes': bool(title_diff or content_diff)
        }
    
    def restore_to_document(self):
        """Restore this version to the document"""
        self.document.title = self.title
        self.document.markdown_content = self.markdown_content
        self.document.html_content = self.html_content
        self.document.author = self.author
        self.document.updated_at = datetime.utcnow()
    
    def to_dict(self, include_content=True):
        data = {
            'id': self.id,
            'document_id': self.document_id,
            'version_number': self.version_number,
            'title': self.title,
            'author': self.author,
            'content_hash': self.content_hash,
            'change_summary': self.change_summary,
            'created_by': self.created_by,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'creator': self.creator.to_dict() if self.creator else None
        }
        
        if include_content:
            data.update({
                'markdown_content': self.markdown_content,
                'html_content': self.html_content
            })
        
        return data
    
    def __repr__(self):
        return f'<DocumentVersion {self.version_number} of Document {self.document_id}>'

class DocumentSnapshot(db.Model):
    """Store periodic snapshots for efficient version control"""
    __tablename__ = 'document_snapshots'
    
    id = db.Column(db.Integer, primary_key=True)
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=False)
    version_number = db.Column(db.Integer, nullable=False)  # Snapshot at this version
    title = db.Column(db.String(255), nullable=False)
    markdown_content = db.Column(db.Text, nullable=False)
    html_content = db.Column(db.Text)
    author = db.Column(db.String(255))
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    
    # Relationships
    document = db.relationship('Document', backref='snapshots')
    
    def __init__(self, document_id, version_number, title, markdown_content, html_content, author=None):
        self.document_id = document_id
        self.version_number = version_number
        self.title = title
        self.markdown_content = markdown_content
        self.html_content = html_content
        self.author = author
    
    @staticmethod
    def should_create_snapshot(version_number):
        """Determine if a snapshot should be created (every 10 versions)"""
        return version_number % 10 == 0
    
    @staticmethod
    def create_snapshot(document_version):
        """Create a snapshot from a document version"""
        return DocumentSnapshot(
            document_id=document_version.document_id,
            version_number=document_version.version_number,
            title=document_version.title,
            markdown_content=document_version.markdown_content,
            html_content=document_version.html_content,
            author=document_version.author
        )
    
    def to_dict(self):
        return {
            'id': self.id,
            'document_id': self.document_id,
            'version_number': self.version_number,
            'title': self.title,
            'markdown_content': self.markdown_content,
            'html_content': self.html_content,
            'author': self.author,
            'created_at': self.created_at.isoformat() if self.created_at else None
        }
    
    def __repr__(self):
        return f'<DocumentSnapshot v{self.version_number} of Document {self.document_id}>'