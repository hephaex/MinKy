from flask_bcrypt import generate_password_hash, check_password_hash
from app import db
from app.utils.datetime_utils import utc_now


class User(db.Model):
    __tablename__ = 'users'

    id = db.Column(db.Integer, primary_key=True)
    username = db.Column(db.String(80), unique=True, nullable=False)
    email = db.Column(db.String(120), unique=True, nullable=False)
    password_hash = db.Column(db.String(128), nullable=False)
    full_name = db.Column(db.String(200))
    is_active = db.Column(db.Boolean, default=True)
    is_admin = db.Column(db.Boolean, default=False)
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)
    # SECURITY: Track failed login attempts for brute force protection
    failed_login_attempts = db.Column(db.Integer, default=0)
    last_failed_login = db.Column(db.DateTime, nullable=True)
    locked_until = db.Column(db.DateTime, nullable=True)
    
    # Relationship with documents
    documents = db.relationship('Document', backref='owner', lazy=True, foreign_keys='Document.user_id')
    
    def __init__(self, username, email, password, full_name=None):
        self.username = username
        self.email = email
        self.full_name = full_name
        self.set_password(password)
    
    def set_password(self, password):
        # Validate password complexity (aligned with schema requirements)
        if not password or len(password) < 12:
            raise ValueError("Password must be at least 12 characters long")
        if not any(c.isupper() for c in password):
            raise ValueError("Password must contain at least one uppercase letter")
        if not any(c.islower() for c in password):
            raise ValueError("Password must contain at least one lowercase letter")
        if not any(c.isdigit() for c in password):
            raise ValueError("Password must contain at least one digit")
        # SECURITY: Require special character for stronger passwords
        special_chars = '!@#$%^&*()_+-=[]{}|;:,.<>?'
        if not any(c in special_chars for c in password):
            raise ValueError("Password must contain at least one special character (!@#$%^&*()_+-=[]{}|;:,.<>?)")
        self.password_hash = generate_password_hash(password).decode('utf-8')
    
    def check_password(self, password):
        """Check password with brute force protection"""
        # Check if account is locked
        if self.locked_until and self.locked_until > utc_now():
            return False

        is_valid = check_password_hash(self.password_hash, password)

        if is_valid:
            # Reset failed attempts on successful login
            self.failed_login_attempts = 0
            self.last_failed_login = None
            self.locked_until = None
        else:
            # Track failed attempt
            self.failed_login_attempts = (self.failed_login_attempts or 0) + 1
            self.last_failed_login = utc_now()

            # Lock account after 5 failed attempts for 15 minutes
            if self.failed_login_attempts >= 5:
                from datetime import timedelta
                self.locked_until = utc_now() + timedelta(minutes=15)

        return is_valid

    def is_locked(self):
        """Check if account is currently locked"""
        return self.locked_until is not None and self.locked_until > utc_now()
    
    def to_dict(self, include_sensitive=False):
        data = {
            'id': self.id,
            'username': self.username,
            'email': self.email if include_sensitive else None,
            'full_name': self.full_name,
            'is_active': self.is_active,
            'is_admin': self.is_admin if include_sensitive else None,
            'created_at': self.created_at.isoformat() if self.created_at else None,
            'updated_at': self.updated_at.isoformat() if self.updated_at else None
        }
        if not include_sensitive:
            # Remove None values for public view
            data = {k: v for k, v in data.items() if v is not None}
        return data
    
    @staticmethod
    def find_by_username(username):
        return User.query.filter_by(username=username).first()
    
    @staticmethod
    def find_by_email(email):
        return User.query.filter_by(email=email).first()
    
    def __repr__(self):
        return f'<User {self.username}>'