from flask import Blueprint, request, Response
from flask_jwt_extended import create_access_token, create_refresh_token, jwt_required, get_jwt_identity
from email_validator import validate_email, EmailNotValidError
from pydantic import ValidationError
from app import db, limiter
from app.models.user import User
from app.schemas.auth import RegisterRequest, LoginRequest
from app.utils.validation import format_validation_errors
from app.utils.responses import success_response, error_response
import logging

logger = logging.getLogger(__name__)

auth_bp = Blueprint('auth', __name__)

@auth_bp.route('/register', methods=['POST'])
@limiter.limit("20 per minute")
def register() -> Response | tuple[Response, int]:
    """Register a new user
    ---
    tags:
      - Auth
    parameters:
      - in: body
        name: body
        required: true
        schema:
          type: object
          required:
            - username
            - email
            - password
          properties:
            username:
              type: string
              description: Username (3-50 chars, alphanumeric and underscores)
              example: johndoe
            email:
              type: string
              format: email
              description: Email address
              example: john@example.com
            password:
              type: string
              format: password
              description: Password (min 8 chars)
              example: SecurePass123!
            full_name:
              type: string
              description: Full name (optional)
              example: John Doe
    responses:
      201:
        description: User registered successfully
        schema:
          type: object
          properties:
            success:
              type: boolean
              example: true
            data:
              type: object
              properties:
                message:
                  type: string
                user:
                  type: object
                access_token:
                  type: string
                refresh_token:
                  type: string
      400:
        description: Validation error
      409:
        description: Username or email already exists
    """
    try:
        data = request.get_json()

        if not data:
            return error_response('No data provided', 400)

        # Validate with Pydantic schema
        try:
            validated = RegisterRequest.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        # Check if user already exists
        if User.find_by_username(validated.username):
            return error_response('Username already exists', 409)

        if User.find_by_email(validated.email):
            return error_response('Email already registered', 409)

        # Create new user
        full_name = data.get('full_name', '').strip() if data.get('full_name') else None
        user = User(
            username=validated.username,
            email=validated.email,
            password=validated.password,
            full_name=full_name
        )

        db.session.add(user)
        db.session.commit()

        # Create tokens (use string identity for PyJWT 2.x compatibility)
        access_token = create_access_token(identity=str(user.id))
        refresh_token = create_refresh_token(identity=str(user.id))

        return success_response({
            'message': 'User registered successfully',
            'user': user.to_dict(),
            'access_token': access_token,
            'refresh_token': refresh_token
        }, status_code=201)

    except Exception as e:
        db.session.rollback()
        logger.error("Error during registration: %s", e)
        return error_response('Internal server error', 500)

@auth_bp.route('/login', methods=['POST'])
@limiter.limit("10 per minute")
def login() -> Response | tuple[Response, int]:
    """Authenticate user and get tokens
    ---
    tags:
      - Auth
    parameters:
      - in: body
        name: body
        required: true
        schema:
          type: object
          required:
            - username
            - password
          properties:
            username:
              type: string
              description: Username or email
              example: johndoe
            password:
              type: string
              format: password
              description: Password
              example: SecurePass123!
    responses:
      200:
        description: Login successful
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                message:
                  type: string
                user:
                  type: object
                access_token:
                  type: string
                refresh_token:
                  type: string
      400:
        description: Validation error
      401:
        description: Invalid credentials or account disabled
    """
    try:
        data = request.get_json()

        if not data:
            return error_response('No data provided', 400)

        # Validate with Pydantic schema
        try:
            validated = LoginRequest.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        # Find user by username or email
        user = User.find_by_username(validated.username)
        if not user:
            user = User.find_by_email(validated.username)

        if not user or not user.check_password(validated.password):
            return error_response('Invalid credentials', 401)

        if not user.is_active:
            return error_response('Account is disabled', 401)

        # Create tokens (use string identity for PyJWT 2.x compatibility)
        access_token = create_access_token(identity=str(user.id))
        refresh_token = create_refresh_token(identity=str(user.id))

        return success_response({
            'message': 'Login successful',
            'user': user.to_dict(),
            'access_token': access_token,
            'refresh_token': refresh_token
        })

    except Exception as e:
        logger.error("Error during login: %s", e)
        return error_response('Internal server error', 500)

@auth_bp.route('/refresh', methods=['POST'])
@limiter.limit("30 per hour")
@jwt_required(refresh=True)
def refresh() -> Response | tuple[Response, int]:
    """Refresh access token
    ---
    tags:
      - Auth
    security:
      - Bearer: []
    responses:
      200:
        description: Token refreshed
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                access_token:
                  type: string
                user:
                  type: object
      401:
        description: Invalid or expired refresh token
    """
    try:
        current_user_id = get_jwt_identity()
        user = db.session.get(User, current_user_id)

        if not user or not user.is_active:
            return error_response('User not found or inactive', 401)

        access_token = create_access_token(identity=str(current_user_id))

        return success_response({
            'access_token': access_token,
            'user': user.to_dict()
        })

    except Exception as e:
        logger.error("Error during token refresh: %s", e)
        return error_response('Internal server error', 500)

@auth_bp.route('/me', methods=['GET'])
@jwt_required()
def get_current_user() -> Response | tuple[Response, int]:
    """Get current user profile
    ---
    tags:
      - Auth
    security:
      - Bearer: []
    responses:
      200:
        description: User profile retrieved
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                user:
                  type: object
                  properties:
                    id:
                      type: integer
                    username:
                      type: string
                    email:
                      type: string
                    full_name:
                      type: string
                    is_active:
                      type: boolean
                    created_at:
                      type: string
                      format: date-time
      401:
        description: Unauthorized
      404:
        description: User not found
    """
    try:
        current_user_id = get_jwt_identity()
        user = db.session.get(User, current_user_id)

        if not user:
            return error_response('User not found', 404)

        return success_response({
            'user': user.to_dict(include_sensitive=True)
        })

    except Exception as e:
        logger.error("Error getting current user: %s", e)
        return error_response('Internal server error', 500)

@auth_bp.route('/profile', methods=['PUT'])
@jwt_required()
def update_profile() -> Response | tuple[Response, int]:
    """Update user profile
    ---
    tags:
      - Auth
    security:
      - Bearer: []
    parameters:
      - in: body
        name: body
        required: true
        schema:
          type: object
          properties:
            full_name:
              type: string
              description: Full name
              example: John Doe
            email:
              type: string
              format: email
              description: New email address
              example: newemail@example.com
    responses:
      200:
        description: Profile updated successfully
        schema:
          type: object
          properties:
            success:
              type: boolean
            data:
              type: object
              properties:
                message:
                  type: string
                user:
                  type: object
      400:
        description: Invalid data provided
      401:
        description: Unauthorized
      409:
        description: Email already registered
    """
    try:
        current_user_id = get_jwt_identity()
        user = db.session.get(User, current_user_id)

        if not user:
            return error_response('User not found', 404)

        data = request.get_json()
        if not data:
            return error_response('No data provided', 400)

        # Update allowed fields
        if 'full_name' in data:
            user.full_name = data['full_name'].strip() if data['full_name'] else None

        if 'email' in data:
            email = data['email'].strip()
            try:
                validate_email(email)
                if User.find_by_email(email) and User.find_by_email(email).id != user.id:
                    return error_response('Email already registered', 409)
                user.email = email
            except EmailNotValidError:
                return error_response('Invalid email address', 400)

        db.session.commit()

        return success_response({
            'message': 'Profile updated successfully',
            'user': user.to_dict(include_sensitive=True)
        })

    except Exception as e:
        db.session.rollback()
        logger.error("Error updating profile: %s", e)
        return error_response('Internal server error', 500)