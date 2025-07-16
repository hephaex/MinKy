from flask import Flask, send_from_directory
from flask_sqlalchemy import SQLAlchemy
from flask_migrate import Migrate
from flask_cors import CORS
from flask_jwt_extended import JWTManager
from flask_bcrypt import Bcrypt
from flask_limiter import Limiter
from flask_limiter.util import get_remote_address
from dotenv import load_dotenv
import os
from datetime import timedelta

load_dotenv()

db = SQLAlchemy()
migrate = Migrate()
jwt = JWTManager()
bcrypt = Bcrypt()
limiter = Limiter(key_func=get_remote_address)

def create_app():
    app = Flask(__name__)
    
    app.config['SQLALCHEMY_DATABASE_URI'] = os.getenv('DATABASE_URL', 'postgresql://localhost/minky_db')
    app.config['SQLALCHEMY_TRACK_MODIFICATIONS'] = False
    app.config['SECRET_KEY'] = os.getenv('SECRET_KEY', 'dev-secret-key')
    app.config['JWT_SECRET_KEY'] = os.getenv('JWT_SECRET_KEY', 'jwt-secret-key')
    app.config['JWT_ACCESS_TOKEN_EXPIRES'] = timedelta(hours=24)
    app.config['JWT_REFRESH_TOKEN_EXPIRES'] = timedelta(days=30)
    
    # Rate limiting configuration
    app.config['RATELIMIT_STORAGE_URL'] = os.getenv('REDIS_URL', 'memory://')
    app.config['RATELIMIT_DEFAULT'] = os.getenv('RATE_LIMIT_DEFAULT', '1000 per hour')
    app.config['RATELIMIT_HEADERS_ENABLED'] = True
    
    CORS(app)
    db.init_app(app)
    migrate.init_app(app, db)
    jwt.init_app(app)
    bcrypt.init_app(app)
    limiter.init_app(app)
    
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
    # from app.routes.org_roam import org_roam_bp
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
    # app.register_blueprint(org_roam_bp, url_prefix='/api')  # Temporarily disabled due to decorator conflicts
    
    # Add static route for serving images from backup/img directory
    @app.route('/img/<path:filename>')
    def serve_image(filename):
        """Serve images from backup/img directory"""
        backup_img_dir = os.path.join(os.getcwd(), 'backup', 'img')
        return send_from_directory(backup_img_dir, filename)
    
    return app