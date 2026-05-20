from app import db
from app.utils.datetime_utils import utc_now
import os
import hashlib
from werkzeug.utils import secure_filename


class Attachment(db.Model):
    __tablename__ = 'attachments'
    
    id = db.Column(db.Integer, primary_key=True)
    filename = db.Column(db.String(255), nullable=False)
    original_filename = db.Column(db.String(255), nullable=False)
    file_path = db.Column(db.String(500), nullable=False)
    file_size = db.Column(db.Integer, nullable=False)  # Size in bytes
    mime_type = db.Column(db.String(100), nullable=False)
    file_hash = db.Column(db.String(64), nullable=False)  # SHA-256 hash
    document_id = db.Column(db.Integer, db.ForeignKey('documents.id'), nullable=True)  # Can be null for orphaned files
    uploaded_by = db.Column(db.Integer, db.ForeignKey('users.id'), nullable=False)
    is_public = db.Column(db.Boolean, default=False)  # Private by default for security
    created_at = db.Column(db.DateTime, default=utc_now)
    
    # Relationships
    document = db.relationship('Document', backref='attachments')
    uploader = db.relationship('User', backref='uploaded_files')
    
    def __init__(self, filename, original_filename, file_path, file_size, mime_type,
                 file_hash, uploaded_by, document_id=None, is_public=False):
        self.filename = filename
        self.original_filename = original_filename
        self.file_path = file_path
        self.file_size = file_size
        self.mime_type = mime_type
        self.file_hash = file_hash
        self.uploaded_by = uploaded_by
        self.document_id = document_id
        self.is_public = is_public
    
    @staticmethod
    def generate_filename(original_filename):
        """Generate a secure unique filename"""
        name, ext = os.path.splitext(original_filename)
        timestamp = datetime.now(timezone.utc).strftime('%Y%m%d_%H%M%S')
        random_suffix = hashlib.md5(f"{original_filename}{timestamp}".encode()).hexdigest()[:8]
        secure_name = secure_filename(name)[:50]  # Limit length
        return f"{secure_name}_{timestamp}_{random_suffix}{ext}"
    
    @staticmethod
    def calculate_file_hash(file_path):
        """Calculate SHA-256 hash of file"""
        sha256_hash = hashlib.sha256()
        try:
            with open(file_path, "rb") as f:
                for chunk in iter(lambda: f.read(4096), b""):
                    sha256_hash.update(chunk)
            return sha256_hash.hexdigest()
        except FileNotFoundError:
            return None
    
    def get_file_extension(self):
        """Get file extension"""
        return os.path.splitext(self.original_filename)[1].lower()
    
    def is_image(self):
        """Check if file is an image.

        SECURITY: SVG excluded due to potential XSS via embedded JavaScript.
        """
        # SECURITY: SVG removed to prevent XSS attacks
        image_extensions = ['.jpg', '.jpeg', '.png', '.gif', '.bmp', '.webp']
        return self.get_file_extension() in image_extensions
    
    def is_document(self):
        """Check if file is a document"""
        doc_extensions = ['.pdf', '.doc', '.docx', '.txt', '.rtf', '.odt']
        return self.get_file_extension() in doc_extensions
    
    def is_video(self):
        """Check if file is a video"""
        video_extensions = ['.mp4', '.avi', '.mov', '.wmv', '.flv', '.webm']
        return self.get_file_extension() in video_extensions
    
    def is_audio(self):
        """Check if file is audio"""
        audio_extensions = ['.mp3', '.wav', '.ogg', '.m4a', '.flac']
        return self.get_file_extension() in audio_extensions
    
    def get_file_type(self):
        """Get file type category"""
        if self.is_image():
            return 'image'
        elif self.is_document():
            return 'document'
        elif self.is_video():
            return 'video'
        elif self.is_audio():
            return 'audio'
        else:
            return 'other'
    
    def get_human_readable_size(self):
        """Get human readable file size"""
        size = self.file_size  # Use local variable to avoid mutating instance state
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size < 1024.0:
                return f"{size:.1f} {unit}"
            size /= 1024.0
        return f"{size:.1f} TB"
    
    def can_view(self, user_id):
        """Check if user can view this attachment"""
        if self.is_public:
            return True
        if self.uploaded_by == user_id:
            return True
        if self.document and self.document.can_view(user_id):
            return True
        return False
    
    def can_delete(self, user_id):
        """Check if user can delete this attachment"""
        return self.uploaded_by == user_id
    
    def delete_file(self):
        """Delete the physical file"""
        try:
            if os.path.exists(self.file_path):
                os.remove(self.file_path)
                return True
        except Exception:
            pass
        return False
    
    @staticmethod
    def cleanup_orphaned_files():
        """Remove attachments not associated with any document"""
        orphaned = Attachment.query.filter_by(document_id=None).all()
        for attachment in orphaned:
            attachment.delete_file()
            db.session.delete(attachment)
        db.session.commit()
        return len(orphaned)
    
    @staticmethod
    def get_storage_stats():
        """Get storage statistics"""
        total_files = Attachment.query.count()
        total_size = db.session.query(db.func.sum(Attachment.file_size)).scalar() or 0
        
        stats_by_type = db.session.query(
            Attachment.mime_type,
            db.func.count(Attachment.id).label('count'),
            db.func.sum(Attachment.file_size).label('total_size')
        ).group_by(Attachment.mime_type).all()
        
        return {
            'total_files': total_files,
            'total_size': total_size,
            'by_type': [
                {
                    'mime_type': stat.mime_type,
                    'count': stat.count,
                    'total_size': stat.total_size
                }
                for stat in stats_by_type
            ]
        }
    
    def to_dict(self, include_hash=False):
        """Serialize attachment to dictionary.

        Args:
            include_hash: If True, include file_hash (internal use only)
        """
        result = {
            'id': self.id,
            'filename': self.filename,
            'original_filename': self.original_filename,
            'file_size': self.file_size,
            'human_readable_size': self.get_human_readable_size(),
            'mime_type': self.mime_type,
            'file_type': self.get_file_type(),
            'document_id': self.document_id,
            'uploaded_by': self.uploaded_by,
            'is_public': self.is_public,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'uploader': self.uploader.to_dict() if self.uploader else None,
            'download_url': f'/api/attachments/{self.id}/download',
            'is_image': self.is_image(),
            'is_document': self.is_document(),
            'is_video': self.is_video(),
            'is_audio': self.is_audio()
        }
        # Only include file_hash for internal operations (duplicate detection)
        if include_hash:
            result['file_hash'] = self.file_hash
        return result
    
    def __repr__(self):
        return f'<Attachment {self.original_filename}>'