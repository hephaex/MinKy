from flask import Flask, send_from_directory
from flask_sqlalchemy import SQLAlchemy
from flask_migrate import Migrate
from flask_cors import CORS
from flask_jwt_extended import JWTManager
from flask_bcrypt import Bcrypt
from flask_limiter import Limiter
from flask_limiter.util import get_remote_address
from flask_socketio import SocketIO
from dotenv import load_dotenv
import os
import sys
import logging
from datetime import timedelta

load_dotenv()

logger = logging.getLogger(__name__)

db = SQLAlchemy()
migrate = Migrate()
jwt = JWTManager()
bcrypt = Bcrypt()
limiter = Limiter(
    app=None,
    key_func=get_remote_address,
    storage_uri=os.getenv('REDIS_URL', 'memory://'),
    default_limits=["1000 per hour"]
)
socketio = SocketIO()


def validate_security_config():
    """Validate that required security configuration is properly set."""
    errors = []

    secret_key = os.getenv('SECRET_KEY')
    jwt_secret = os.getenv('JWT_SECRET_KEY')
    flask_env = os.getenv('FLASK_ENV', 'production')

    # In production, require proper secrets
    if flask_env == 'production':
        if not secret_key:
            errors.append("SECRET_KEY environment variable is required in production")
        elif len(secret_key) < 32:
            errors.append("SECRET_KEY must be at least 32 characters in production")
        elif secret_key in ['dev-secret-key', 'secret-key', 'changeme']:
            errors.append("SECRET_KEY contains an unsafe default value")

        if not jwt_secret:
            errors.append("JWT_SECRET_KEY environment variable is required in production")
        elif len(jwt_secret) < 32:
            errors.append("JWT_SECRET_KEY must be at least 32 characters in production")
        elif jwt_secret in ['jwt-secret-key', 'secret-key', 'changeme']:
            errors.append("JWT_SECRET_KEY contains an unsafe default value")

    return errors


def create_app():
    app = Flask(__name__)

    flask_env = os.getenv('FLASK_ENV', 'production')

    # Validate security configuration
    security_errors = validate_security_config()
    if security_errors:
        for error in security_errors:
            logger.critical(f"SECURITY CONFIG ERROR: {error}")
        if flask_env == 'production':
            sys.exit(1)  # Fail fast in production with insecure config
        else:
            logger.warning("Running with insecure configuration in development mode")

    # Get secrets - NO unsafe defaults in production
    secret_key = os.getenv('SECRET_KEY')
    jwt_secret = os.getenv('JWT_SECRET_KEY')

    # Only use fallback in development/testing
    if flask_env != 'production':
        secret_key = secret_key or 'dev-secret-key-for-development-only'
        jwt_secret = jwt_secret or 'jwt-secret-key-for-development-only'

    app.config['SQLALCHEMY_DATABASE_URI'] = os.getenv('DATABASE_URL', 'postgresql://localhost/minky_db')
    app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False
    app.config['SECRET_KEY'] = secret_key
    app.config['JWT_SECRET_KEY'] = jwt_secret
    app.config['JWT_ACCESS_TOKEN_EXPIRES'] = timedelta(hours=24)
    app.config['JWT_REFRESH_TOKEN_EXPIRES'] = timedelta(days=30)
    
    # Rate limiting configuration
    app.config['RATELIMIT_DEFAULT'] = os.getenv('RATE_LIMIT_DEFAULT', '1000 per hour')
    app.config['RATELIMIT_HEADERS_ENABLED'] = True
    
    CORS(app, origins=["http://localhost:3000"])
    db.init_app(app)
    migrate.init_app(app, db)
    jwt.init_app(app)
    bcrypt.init_app(app)
    limiter.init_app(app)
    socketio.init_app(app, cors_allowed_origins=["http://localhost:3000"])
    
    from app.routes.documents import documents_bp
    from app.routes.auth import auth_bp
    from app.routes.health import health_bp
    from app.routes.tags import tags_bp
    from app.routes.comments import comments_bp
    from app.routes.versions import versions_bp
    from app.routes.templates import templates_bp
    from app.routes.attachments import attachments_bp
    from app.routes.export import export_bp
    from app.routes.notifications import notifications_bp
    from app.routes.security import security_bp
    from app.routes.workflows import workflows_bp
    from app.routes.korean_search import korean_search_bp
    from app.routes.analytics import analytics_bp
    from app.routes.admin import admin_bp
    from app.routes.categories import categories_bp
    from app.routes.ai_suggestions import ai_suggestions_bp
    from app.routes.ocr import ocr_bp
    from app.routes.ml_analytics import ml_analytics_bp
    from app.routes.document_clustering import clustering_bp
    from app.routes.git import git_bp
    from app.routes.org_roam import org_roam_bp
    app.register_blueprint(documents_bp, url_prefix='/api')
    app.register_blueprint(auth_bp, url_prefix='/api/auth')
    app.register_blueprint(health_bp, url_prefix='/api')
    app.register_blueprint(tags_bp, url_prefix='/api')
    app.register_blueprint(comments_bp, url_prefix='/api')
    app.register_blueprint(versions_bp, url_prefix='/api')
    app.register_blueprint(templates_bp, url_prefix='/api')
    app.register_blueprint(attachments_bp, url_prefix='/api')
    app.register_blueprint(export_bp, url_prefix='/api')
    app.register_blueprint(notifications_bp, url_prefix='/api')
    app.register_blueprint(security_bp, url_prefix='/api')
    app.register_blueprint(workflows_bp, url_prefix='/api')
    app.register_blueprint(korean_search_bp, url_prefix='/api')
    app.register_blueprint(analytics_bp, url_prefix='/api')
    app.register_blueprint(admin_bp, url_prefix='/api')
    app.register_blueprint(categories_bp, url_prefix='/api/categories')
    app.register_blueprint(ai_suggestions_bp, url_prefix='/api')
    app.register_blueprint(ocr_bp, url_prefix='/api')
    app.register_blueprint(ml_analytics_bp, url_prefix='/api')
    app.register_blueprint(clustering_bp, url_prefix='/api')
    app.register_blueprint(git_bp, url_prefix='/api')
    app.register_blueprint(org_roam_bp, url_prefix='/api')
    
    # Initialize collaboration service and register WebSocket events
    from app.services.collaboration_service import init_collaboration_service
    from app.routes.websocket_events import register_websocket_events
    
    init_collaboration_service(socketio)
    register_websocket_events(socketio)
    
    # Add static route for serving images from backup/img directory
    @app.route('/img/<path:filename>')
    def serve_image(filename):
        """Serve images from backup/img directory"""
        backup_img_dir = os.path.join(os.getcwd(), 'backup', 'img')
        return send_from_directory(backup_img_dir, filename)
    
    return app