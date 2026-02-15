"""
Security-focused test cases for MinKy application.
Tests rate limiting, input validation, XSS prevention, and authorization.
"""
import pytest


class TestPasswordSecurity:
    """Test password security requirements"""

    def test_weak_password_rejected(self, client):
        """Test that weak passwords are rejected during registration"""
        register_data = {
            'username': 'testuser',
            'email': 'test@example.com',
            'password': '123'  # Too short
        }
        response = client.post('/api/auth/register', json=register_data)
        assert response.status_code == 400

    def test_password_requires_uppercase(self, client):
        """Test that passwords require at least one uppercase letter"""
        register_data = {
            'username': 'testuser',
            'email': 'test@example.com',
            'password': 'alllowercase123'  # No uppercase
        }
        response = client.post('/api/auth/register', json=register_data)
        assert response.status_code == 400

    def test_password_requires_digit(self, client):
        """Test that passwords require at least one digit"""
        register_data = {
            'username': 'testuser',
            'email': 'test@example.com',
            'password': 'NoDigitsHere'  # No digit
        }
        response = client.post('/api/auth/register', json=register_data)
        assert response.status_code == 400


class TestInputValidation:
    """Test input validation and sanitization"""

    def test_xss_in_username_handled(self, client):
        """Test that XSS payloads in username are handled safely"""
        register_data = {
            'username': '<script>alert(1)</script>',
            'email': 'test@example.com',
            'password': 'SecurePass123'
        }
        response = client.post('/api/auth/register', json=register_data)
        # Should either reject or sanitize the input
        if response.status_code == 201:
            # If accepted, verify it's sanitized in response
            data = response.get_json()
            assert '<script>' not in data.get('user', {}).get('username', '')

    def test_sql_injection_in_search_handled(self, client, auth_token):
        """Test that SQL injection attempts in search are handled safely"""
        headers = {'Authorization': f'Bearer {auth_token}'}
        response = client.get(
            "/api/documents/search?q='; DROP TABLE documents; --",
            headers=headers
        )
        # Should not cause server error
        assert response.status_code in [200, 400]


class TestAuthorization:
    """Test authorization controls"""

    def test_cannot_access_other_user_document(self, client, auth_token, other_user_private_document):
        """Test that users cannot access other users' private documents"""
        headers = {'Authorization': f'Bearer {auth_token}'}
        response = client.get(
            f'/api/documents/{other_user_private_document.id}',
            headers=headers
        )
        assert response.status_code in [403, 404]

    def test_non_admin_cannot_access_admin_routes(self, client, auth_token):
        """Test that non-admin users cannot access admin routes"""
        headers = {'Authorization': f'Bearer {auth_token}'}
        response = client.get('/api/admin/users', headers=headers)
        assert response.status_code == 403


class TestRateLimiting:
    """Test rate limiting functionality"""

    @pytest.mark.slow
    def test_login_rate_limiting(self, client):
        """Test that login endpoint is rate limited"""
        login_data = {'username': 'nonexistent', 'password': 'wrongpass'}

        # Make multiple rapid requests
        responses = []
        for _ in range(25):
            response = client.post('/api/auth/login', json=login_data)
            responses.append(response.status_code)

        # Should eventually get rate limited (429)
        assert 429 in responses, "Rate limiting should trigger after many requests"


class TestJWTSecurity:
    """Test JWT token security"""

    def test_expired_token_rejected(self, client, expired_token):
        """Test that expired JWT tokens are rejected"""
        headers = {'Authorization': f'Bearer {expired_token}'}
        response = client.get('/api/auth/me', headers=headers)
        assert response.status_code == 401

    def test_invalid_token_rejected(self, client):
        """Test that invalid JWT tokens are rejected"""
        headers = {'Authorization': 'Bearer invalid.token.here'}
        response = client.get('/api/auth/me', headers=headers)
        assert response.status_code == 422  # Unprocessable Entity for malformed JWT

    def test_missing_token_rejected(self, client):
        """Test that requests without tokens to protected endpoints are rejected"""
        response = client.get('/api/auth/me')
        assert response.status_code == 401


# Fixtures for security tests
@pytest.fixture
def other_user_private_document(app, db):
    """Create a private document owned by another user"""
    from app.models.document import Document
    from app.models.user import User

    with app.app_context():
        # Create another user
        other_user = User(
            username='otheruser',
            email='other@example.com',
            password='OtherPass123'
        )
        db.session.add(other_user)
        db.session.commit()

        # Create private document
        doc = Document(
            title='Private Doc',
            markdown_content='Secret content',
            user_id=other_user.id,
            is_public=False
        )
        db.session.add(doc)
        db.session.commit()

        yield doc

        # Cleanup
        db.session.delete(doc)
        db.session.delete(other_user)
        db.session.commit()


@pytest.fixture
def expired_token(app):
    """Generate an expired JWT token for testing"""
    from flask_jwt_extended import create_access_token
    from datetime import timedelta

    with app.app_context():
        # Create token with negative expiration (already expired)
        token = create_access_token(
            identity=1,
            expires_delta=timedelta(seconds=-1)
        )
        return token
