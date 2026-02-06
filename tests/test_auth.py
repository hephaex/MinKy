"""
Tests for authentication endpoints.
"""
import json
import pytest
from app import db


def test_register_success(client):
    """Test successful user registration."""
    register_data = {
        'username': 'newuser',
        'email': 'newuser@gmail.com',
        'password': 'SecurePass123',
        'full_name': 'New User'
    }

    response = client.post(
        '/api/auth/register',
        data=json.dumps(register_data),
        content_type='application/json'
    )

    assert response.status_code == 201
    data = json.loads(response.data)
    assert data['message'] == 'User registered successfully'
    assert 'access_token' in data
    assert 'refresh_token' in data
    assert 'user' in data
    assert data['user']['username'] == 'newuser'
    assert 'password_hash' not in data['user']


def test_register_duplicate_username(client, sample_user):
    """Test registration with duplicate username returns 409 conflict."""
    register_data = {
        'username': 'testuser',
        'email': 'different@gmail.com',
        'password': 'SecurePass123'
    }

    response = client.post(
        '/api/auth/register',
        data=json.dumps(register_data),
        content_type='application/json'
    )

    assert response.status_code == 409
    data = json.loads(response.data)
    assert 'error' in data
    assert 'Username already exists' in data['error']


def test_register_duplicate_email(client, sample_user, app):
    """Test registration with duplicate email returns 409 conflict."""
    # First update the sample user to have a real-domain email
    with app.app_context():
        from app.models.user import User
        user = db.session.get(User, sample_user)
        user.email = 'test@gmail.com'
        db.session.commit()

    register_data = {
        'username': 'differentuser',
        'email': 'test@gmail.com',
        'password': 'SecurePass123'
    }

    response = client.post(
        '/api/auth/register',
        data=json.dumps(register_data),
        content_type='application/json'
    )

    assert response.status_code == 409
    data = json.loads(response.data)
    assert 'error' in data
    assert 'Email already registered' in data['error']


def test_register_missing_fields(client):
    """Test registration with missing required fields returns 400."""
    test_cases = [
        {},
        {'username': 'testuser'},
        {'email': 'test@example.com'},
        {'password': 'SecurePass123'},
        {'username': 'testuser', 'email': 'test@example.com'},
        {'username': 'testuser', 'password': 'SecurePass123'},
        {'email': 'test@example.com', 'password': 'SecurePass123'},
    ]

    for register_data in test_cases:
        response = client.post(
            '/api/auth/register',
            data=json.dumps(register_data),
            content_type='application/json'
        )

        assert response.status_code == 400
        data = json.loads(response.data)
        assert 'error' in data


def test_login_success(client, sample_user):
    """Test successful login with valid credentials."""
    login_data = {
        'username': 'testuser',
        'password': 'TestPassword123'
    }

    response = client.post(
        '/api/auth/login',
        data=json.dumps(login_data),
        content_type='application/json'
    )

    assert response.status_code == 200
    data = json.loads(response.data)
    assert data['message'] == 'Login successful'
    assert 'access_token' in data
    assert 'refresh_token' in data
    assert 'user' in data
    assert data['user']['username'] == 'testuser'


def test_login_invalid_password(client, sample_user):
    """Test login with invalid password returns 401 unauthorized."""
    login_data = {
        'username': 'testuser',
        'password': 'WrongPassword123'
    }

    response = client.post(
        '/api/auth/login',
        data=json.dumps(login_data),
        content_type='application/json'
    )

    assert response.status_code == 401
    data = json.loads(response.data)
    assert 'error' in data
    assert 'Invalid credentials' in data['error']


def test_login_nonexistent_user(client):
    """Test login with nonexistent user returns 401 unauthorized."""
    login_data = {
        'username': 'nonexistentuser',
        'password': 'SomePassword123'
    }

    response = client.post(
        '/api/auth/login',
        data=json.dumps(login_data),
        content_type='application/json'
    )

    assert response.status_code == 401
    data = json.loads(response.data)
    assert 'error' in data
    assert 'Invalid credentials' in data['error']


def test_get_current_user(client, auth_headers, sample_user, app):
    """Test getting current user with valid JWT token."""
    response = client.get(
        '/api/auth/me',
        headers=auth_headers
    )

    assert response.status_code == 200
    data = json.loads(response.data)
    assert 'user' in data
    assert data['user']['username'] == 'testuser'
    assert data['user']['email'] == 'test@example.com'
    assert data['user']['is_active'] is True
