"""
AI Configuration Model
Stores AI service configuration settings in the database
"""

from app import db
from app.utils.datetime_utils import utc_now
import logging
import os
import base64

logger = logging.getLogger(__name__)

# Whitelist of allowed configuration keys
ALLOWED_CONFIG_KEYS = {
    'llmEnabled', 'llmProvider', 'llmModel', 'llmApiKey',
    'ocrEnabled', 'ocrService', 'ocrApiKey', 'ocrLanguage',
    'openaiApiKey', 'anthropicApiKey', 'googleApiKey',
    'maxTokens', 'temperature', 'summaryEnabled', 'tagsEnabled'
}

# Keys that contain sensitive data and should be encrypted
SENSITIVE_KEYS = {
    'llmApiKey', 'ocrApiKey', 'openaiApiKey', 'anthropicApiKey', 'googleApiKey'
}

# Maximum value length to prevent DoS
MAX_VALUE_LENGTH = 10000


def _get_encryption_key():
    """Get or generate encryption key for API keys.

    SECURITY: The encryption key should be stored in environment variables
    and rotated periodically. If not set, encryption is disabled with a warning.
    """
    key = os.environ.get('AI_CONFIG_ENCRYPTION_KEY')
    if not key:
        logger.warning("AI_CONFIG_ENCRYPTION_KEY not set - API keys stored without encryption")
        return None
    try:
        # Validate key is valid Fernet key (32 url-safe base64-encoded bytes)
        from cryptography.fernet import Fernet
        Fernet(key.encode() if isinstance(key, str) else key)
        return key.encode() if isinstance(key, str) else key
    except Exception as e:
        logger.error(f"Invalid encryption key format: {e}")
        return None


def _encrypt_value(value: str) -> str:
    """Encrypt a sensitive value using Fernet symmetric encryption."""
    key = _get_encryption_key()
    if not key or not value:
        return value
    try:
        from cryptography.fernet import Fernet
        f = Fernet(key)
        encrypted = f.encrypt(value.encode())
        # Prefix with 'enc:' to identify encrypted values
        return 'enc:' + base64.urlsafe_b64encode(encrypted).decode()
    except Exception as e:
        logger.error(f"Encryption failed: {e}")
        return value


def _decrypt_value(value: str) -> str:
    """Decrypt a value that was encrypted with Fernet."""
    if not value or not value.startswith('enc:'):
        return value
    key = _get_encryption_key()
    if not key:
        logger.warning("Cannot decrypt value - encryption key not available")
        return value
    try:
        from cryptography.fernet import Fernet
        f = Fernet(key)
        encrypted_data = base64.urlsafe_b64decode(value[4:])  # Remove 'enc:' prefix
        return f.decrypt(encrypted_data).decode()
    except Exception as e:
        logger.error(f"Decryption failed: {e}")
        return value


class AIConfig(db.Model):
    __tablename__ = 'ai_config'

    id = db.Column(db.Integer, primary_key=True)
    key = db.Column(db.String(50), unique=True, nullable=False)
    value = db.Column(db.Text, nullable=True)
    created_at = db.Column(db.DateTime, default=utc_now)
    updated_at = db.Column(db.DateTime, default=utc_now, onupdate=utc_now)

    def __repr__(self):
        return f'<AIConfig {self.key}: [REDACTED]>'

    @staticmethod
    def get_value(key, default=None):
        """Get configuration value by key, decrypting if necessary"""
        config = AIConfig.query.filter_by(key=key).first()
        if not config:
            return default
        value = config.value
        # SECURITY: Decrypt sensitive values
        if key in SENSITIVE_KEYS and value:
            value = _decrypt_value(value)
        return value

    @staticmethod
    def set_value(key, value):
        """Set configuration value by key with validation and encryption"""
        # Validate key is in whitelist
        if key not in ALLOWED_CONFIG_KEYS:
            logger.warning(f"Rejected invalid config key: {key}")
            return False

        # Validate value length
        if value and len(str(value)) > MAX_VALUE_LENGTH:
            logger.warning(f"Rejected config value for {key}: exceeds max length")
            return False

        # SECURITY: Encrypt sensitive values (API keys)
        stored_value = value
        if key in SENSITIVE_KEYS and value:
            stored_value = _encrypt_value(value)

        config = AIConfig.query.filter_by(key=key).first()
        if config:
            config.value = stored_value
            config.updated_at = utc_now()
        else:
            config = AIConfig(key=key, value=stored_value)
            db.session.add(config)

        try:
            db.session.commit()
            return True
        except Exception:
            db.session.rollback()
            return False