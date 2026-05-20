"""
Pytest configuration for Minky tests.
Sets up the test environment before importing the app.
"""
import os
import secrets
import pytest

# Set test environment variables BEFORE importing the app
os.environ['FLASK_ENV'] = 'testing'
os.environ['DATABASE_URL'] = 'sqlite:///:memory:'
# Use randomly generated secrets for tests to avoid weak secret patterns
# being copied to production. Prefix with 'test-' to ensure detection if misused.
os.environ['SECRET_KEY'] = f'test-{secrets.token_hex(32)}'
os.environ['JWT_SECRET_KEY'] = f'test-{secrets.token_hex(32)}'

from app import create_app, db
from app.models.document import Document
from app.models.user import User
from app.models.tag import Tag


@pytest.fixture(scope='function')
def app():
    """Create and configure a new app instance for each test."""
    app = create_app()
    app.config['TESTING'] = True
    app.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///:memory:'
    app.config['WTF_CSRF_ENABLED'] = False

    with app.app_context():
        db.create_all()
        yield app
        db.session.remove()
        db.drop_all()


@pytest.fixture
def client(app):
    """A test client for the app."""
    return app.test_client()


@pytest.fixture
def runner(app):
    """A test CLI runner for the app."""
    return app.test_cli_runner()


@pytest.fixture
def sample_document(app):
    """Create a sample document in the database."""
    with app.app_context():
        doc = Document(
            title='Test Document',
            markdown_content='# Test\n\nThis is a test document.',
            author='Test Author',
            is_public=True
        )
        db.session.add(doc)
        db.session.commit()

        # Return the ID so it can be used to fetch the document
        doc_id = doc.id
        return doc_id


@pytest.fixture
def sample_user(app):
    """Create a sample user in the database."""
    with app.app_context():
        # SECURITY: Use password that meets all requirements (12+ chars, special char)
        user = User(
            username='testuser',
            email='test@example.com',
            password='TestPassword123!'  # Meets: 12+ chars, upper, lower, digit, special
        )
        user.is_active = True
        db.session.add(user)
        db.session.commit()

        user_id = user.id
        return user_id


@pytest.fixture
def auth_headers(app, sample_user):
    """Get authentication headers for API requests."""
    with app.app_context():
        from flask_jwt_extended import create_access_token
        # PyJWT 2.x requires string identity
        access_token = create_access_token(identity=str(sample_user))
        return {'Authorization': f'Bearer {access_token}'}


@pytest.fixture
def auth_token(app, sample_user):
    """Get authentication token for API requests (used by test_security.py)."""
    with app.app_context():
        from flask_jwt_extended import create_access_token
        return create_access_token(identity=str(sample_user))


@pytest.fixture
def sample_document_data():
    """Sample document data for creating documents."""
    return {
        'title': 'Test Document',
        'markdown_content': '# Hello World\n\nThis is a test document.',
        'author': 'Test Author',
        'is_public': True
    }
