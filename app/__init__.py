from flask import Flask, send_from_directory, jsonify
from werkzeug.utils import safe_join
from flask_sqlalchemy import SQLAlchemy
from flask_migrate import Migrate
from flask_cors import CORS
from flask_jwt_extended import JWTManager
from flask_bcrypt import Bcrypt
from flask_limiter import Limiter
from flask_limiter.util import get_remote_address
from flask_socketio import SocketIO
from flask_talisman import Talisman
from flask_caching import Cache
from flasgger import Swagger
from prometheus_flask_exporter import PrometheusMetrics
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
talisman = Talisman()
cache = Cache()


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

    # Setup structured logging
    from app.utils.logging_config import setup_logging, add_request_id_middleware
    setup_logging(app)
    add_request_id_middleware(app)

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
    # Security: Short-lived access tokens reduce impact of token theft
    app.config['JWT_ACCESS_TOKEN_EXPIRES'] = timedelta(hours=1)
    app.config['JWT_REFRESH_TOKEN_EXPIRES'] = timedelta(days=7)
    
    # Rate limiting configuration
    app.config['RATELIMIT_DEFAULT'] = os.getenv('RATE_LIMIT_DEFAULT', '1000 per hour')
    app.config['RATELIMIT_HEADERS_ENABLED'] = True

    # SECURITY: Session cookie configuration
    app.config['SESSION_COOKIE_SECURE'] = flask_env == 'production'  # Require HTTPS in production
    app.config['SESSION_COOKIE_HTTPONLY'] = True  # Prevent JavaScript access
    app.config['SESSION_COOKIE_SAMESITE'] = 'Lax'  # CSRF protection

    # CORS configuration - environment-based
    cors_origins = os.getenv('CORS_ORIGINS', 'http://localhost:3000').split(',')
    cors_origins = [origin.strip() for origin in cors_origins]

    # Caching configuration
    cache_type = os.getenv('CACHE_TYPE', 'SimpleCache')
    app.config['CACHE_TYPE'] = cache_type
    app.config['CACHE_DEFAULT_TIMEOUT'] = int(os.getenv('CACHE_DEFAULT_TIMEOUT', '300'))
    if cache_type == 'RedisCache':
        app.config['CACHE_REDIS_URL'] = os.getenv('REDIS_URL', 'redis://localhost:6379/0')

    CORS(app, origins=cors_origins, supports_credentials=True)
    db.init_app(app)
    migrate.init_app(app, db)
    jwt.init_app(app)
    bcrypt.init_app(app)
    limiter.init_app(app)

    # SECURITY: Register JWT token revocation callback for logout functionality
    @jwt.token_in_blocklist_loader
    def check_if_token_revoked(jwt_header, jwt_payload):
        jti = jwt_payload.get('jti')
        if jti:
            from app.routes.auth import is_token_revoked
            return is_token_revoked(jti)
        return False
    cache.init_app(app)
    socketio.init_app(app, cors_allowed_origins=cors_origins)

    # Prometheus metrics (disabled during testing to avoid registry conflicts)
    metrics_enabled = os.getenv('METRICS_ENABLED', 'true').lower() == 'true'
    if metrics_enabled and flask_env not in ('testing', 'test'):
        metrics = PrometheusMetrics(app)
        metrics.info('app_info', 'MinKy API', version='1.0.0')

    # Security headers with Flask-Talisman (disabled in development for easier debugging)
    # SECURITY TODO: 'unsafe-inline' weakens XSS protection. Consider:
    # 1. Using nonces or hashes for inline scripts/styles
    # 2. Migrating inline code to external files
    if flask_env == 'production':
        csp = {
            'default-src': "'self'",
            'script-src': ["'self'", "'unsafe-inline'"],
            'style-src': ["'self'", "'unsafe-inline'"],
            'img-src': ["'self'", "data:", "blob:"],
            'font-src': ["'self'"],
            'connect-src': ["'self'"] + cors_origins,
        }
        talisman.init_app(
            app,
            content_security_policy=csp,
            force_https=True,
            strict_transport_security=True,
            strict_transport_security_max_age=31536000,
            x_content_type_options=True,
            x_xss_protection=True,
        )
    else:
        # In development, only add basic security headers without HTTPS enforcement
        talisman.init_app(
            app,
            content_security_policy=None,
            force_https=False,
            strict_transport_security=False,
        )

    # Swagger/OpenAPI configuration
    swagger_config = {
        "headers": [],
        "specs": [
            {
                "endpoint": 'apispec',
                "route": '/api/docs/apispec.json',
                "rule_filter": lambda rule: True,
                "model_filter": lambda tag: True,
            }
        ],
        "static_url_path": "/flasgger_static",
        "swagger_ui": True,
        "specs_route": "/api/docs/"
    }

    swagger_template = {
        "swagger": "2.0",
        "info": {
            "title": "MinKy API",
            "description": "MinKy Document Management System API",
            "version": "1.0.0",
            "contact": {
                "name": "MinKy Team",
                "email": "support@minky.dev"
            }
        },
        "basePath": "/api",
        "schemes": ["http", "https"],
        "securityDefinitions": {
            "Bearer": {
                "type": "apiKey",
                "name": "Authorization",
                "in": "header",
                "description": "JWT Authorization header using the Bearer scheme. Example: 'Bearer {token}'"
            }
        },
        "tags": [
            {"name": "Auth", "description": "Authentication endpoints"},
            {"name": "Documents", "description": "Document management"},
            {"name": "Tags", "description": "Tag management"},
            {"name": "Categories", "description": "Category management"},
            {"name": "Comments", "description": "Comment management"},
            {"name": "Versions", "description": "Document version control"},
            {"name": "Export", "description": "Document export"},
            {"name": "OCR", "description": "Optical Character Recognition"},
            {"name": "Analytics", "description": "Analytics and statistics"},
            {"name": "Health", "description": "Health check endpoints"}
        ]
    }

    Swagger(app, config=swagger_config, template=swagger_template)
    
    from app.routes.documents import documents_bp
    from app.routes.documents_search import documents_search_bp
    from app.routes.documents_sync import documents_sync_bp
    from app.routes.documents_timeline import documents_timeline_bp
    from app.routes.documents_import import documents_import_bp
    from app.routes.auth import auth_bp
    from app.routes.health import health_bp
    from app.routes.tags_crud import tags_crud_bp
    from app.routes.tags_statistics import tags_statistics_bp
    from app.routes.tags_auto import tags_auto_bp
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
    from app.routes.agents import agents_bp
    app.register_blueprint(documents_bp, url_prefix='/api')
    app.register_blueprint(documents_search_bp, url_prefix='/api')
    app.register_blueprint(documents_sync_bp, url_prefix='/api')
    app.register_blueprint(documents_timeline_bp, url_prefix='/api')
    app.register_blueprint(documents_import_bp, url_prefix='/api')
    app.register_blueprint(auth_bp, url_prefix='/api/auth')
    app.register_blueprint(health_bp, url_prefix='/api')
    app.register_blueprint(tags_crud_bp, url_prefix='/api')
    app.register_blueprint(tags_statistics_bp, url_prefix='/api')
    app.register_blueprint(tags_auto_bp, url_prefix='/api')
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
    app.register_blueprint(agents_bp, url_prefix='/api')

    # Initialize collaboration service and register WebSocket events
    from app.services.collaboration_service import init_collaboration_service
    from app.routes.websocket_events import register_websocket_events
    
    init_collaboration_service(socketio)
    register_websocket_events(socketio)
    
    # Add static route for serving images from backup/img directory
    @app.route('/img/<path:filename>')
    def serve_image(filename):
        """Serve images from backup/img directory with path traversal protection"""
        backup_img_dir = os.path.join(os.getcwd(), 'backup', 'img')
        # Validate path to prevent traversal attacks
        safe_path = safe_join(backup_img_dir, filename)
        if safe_path is None or not os.path.isfile(safe_path):
            return jsonify({'error': 'File not found'}), 404
        return send_from_directory(backup_img_dir, filename)
    
    return app