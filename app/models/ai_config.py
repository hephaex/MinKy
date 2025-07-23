"""
AI Configuration Model
Stores AI service configuration settings in the database
"""

from app import db
from datetime import datetime

class AIConfig(db.Model):
    __tablename__ = 'ai_config'
    
    id = db.Column(db.Integer, primary_key=True)
    key = db.Column(db.String(50), unique=True, nullable=False)
    value = db.Column(db.Text, nullable=True)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    def __repr__(self):
        return f'<AIConfig {self.key}: {self.value}>'
    
    @staticmethod
    def get_value(key, default=None):
        """Get configuration value by key"""
        config = AIConfig.query.filter_by(key=key).first()
        return config.value if config else default
    
    @staticmethod
    def set_value(key, value):
        """Set configuration value by key"""
        config = AIConfig.query.filter_by(key=key).first()
        if config:
            config.value = value
            config.updated_at = datetime.utcnow()
        else:
            config = AIConfig(key=key, value=value)
            db.session.add(config)
        
        try:
            db.session.commit()
            return True
        except Exception as e:
            db.session.rollback()
            return False