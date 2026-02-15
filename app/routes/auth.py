from flask import Blueprint, request, Response
from flask_jwt_extended import create_access_token, create_refresh_token, jwt_required, get_jwt_identity, get_jwt
from email_validator import validate_email, EmailNotValidError
from pydantic import ValidationError
from app import db, limiter
from app.models.user import User
from app.schemas.auth import RegisterRequest, LoginRequest, PasswordChange
from app.utils.validation import format_validation_errors
from app.utils.responses import success_response, error_response
import logging
import secrets
import hmac

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

        # SECURITY: Check if user already exists - use generic message to prevent enumeration
        username_exists = User.find_by_username(validated.username) is not None
        email_exists = User.find_by_email(validated.email) is not None

        if username_exists or email_exists:
            return error_response('An account with this username or email already exists', 409)

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

        # SECURITY: Check if account is locked BEFORE password validation
        # This prevents attackers from determining correct passwords on locked accounts
        if user and user.is_locked():
            return error_response('Account is temporarily locked due to too many failed attempts', 401)

        # SECURITY: Prevent timing attacks - always perform password check
        # Use a dummy hash if user doesn't exist to prevent user enumeration
        if user:
            password_valid = user.check_password(validated.password)
            # SECURITY: Persist failed login attempts to database
            db.session.commit()
        else:
            # Perform dummy hash comparison to prevent timing-based user enumeration
            from flask_bcrypt import check_password_hash
            dummy_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4BoHnQ/VVelDdaGC"
            check_password_hash(dummy_hash, validated.password)
            password_valid = False

        if not password_valid:
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
        db.session.rollback()
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


# SECURITY: In-memory revoked token store (use Redis in production for distributed systems)
_revoked_tokens = set()

# SECURITY: Track password change timestamps to invalidate tokens issued before password change
# Key: user_id, Value: timestamp of last password change
_password_change_timestamps = {}


@auth_bp.route('/logout', methods=['POST'])
@limiter.limit("30 per minute")
@jwt_required()
def logout() -> Response | tuple[Response, int]:
    """Logout user and revoke current token
    ---
    tags:
      - Auth
    security:
      - Bearer: []
    responses:
      200:
        description: Logout successful
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
      401:
        description: Unauthorized
    """
    try:
        # Get the JWT ID (jti) to revoke this specific token
        jwt_data = get_jwt()
        jti = jwt_data.get('jti')

        if jti:
            _revoked_tokens.add(jti)
            logger.info(f"Token revoked: {jti[:8]}...")

        return success_response({
            'message': 'Logout successful'
        })

    except Exception as e:
        logger.error("Error during logout: %s", e)
        return error_response('Internal server error', 500)


def is_token_revoked(jti: str) -> bool:
    """Check if a token has been revoked"""
    return jti in _revoked_tokens


def is_token_invalidated_by_password_change(user_id: str, token_issued_at) -> bool:
    """Check if token was issued before the user's last password change"""
    from datetime import datetime as dt, timezone
    pwd_change_time = _password_change_timestamps.get(str(user_id))
    if pwd_change_time and token_issued_at:
        # Token is invalid if it was issued before password change
        return token_issued_at < pwd_change_time
    return False


def invalidate_user_sessions(user_id: int) -> None:
    """Invalidate all sessions for a user by recording password change time"""
    from datetime import datetime as dt, timezone
    _password_change_timestamps[str(user_id)] = dt.now(timezone.utc)
    logger.info(f"All sessions invalidated for user {user_id}")

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
@limiter.limit("10 per minute")
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


@auth_bp.route('/change-password', methods=['POST'])
@limiter.limit("3 per hour")  # Strict rate limiting for password changes
@jwt_required()
def change_password() -> Response | tuple[Response, int]:
    """Change user password with old password verification
    ---
    tags:
      - Auth
    security:
      - BearerAuth: []
    parameters:
      - in: body
        name: body
        required: true
        schema:
          type: object
          required:
            - current_password
            - new_password
          properties:
            current_password:
              type: string
            new_password:
              type: string
              minLength: 12
    responses:
      200:
        description: Password changed successfully
      400:
        description: Invalid request
      401:
        description: Current password incorrect
    """
    try:
        data = request.get_json()
        if not data:
            return error_response('Request body required', 400)

        # SECURITY: Validate with Pydantic schema for proper password strength validation
        try:
            validated = PasswordChange.model_validate(data)
        except ValidationError as e:
            errors = format_validation_errors(e)
            return error_response('Validation failed', 400, details={'validation_errors': errors})

        user_id = get_jwt_identity()
        user = db.session.get(User, user_id)

        if not user:
            return error_response('User not found', 404)

        # Verify current password
        if not user.check_password(validated.current_password):
            db.session.commit()  # Persist failed attempt
            logger.warning("Failed password change attempt for user %s", user_id)
            return error_response('Current password is incorrect', 401)

        # SECURITY: Verify new password is different from current password
        if user.check_password(validated.new_password):
            return error_response('New password must be different from current password', 400)

        # Set new password (schema already validated password strength)
        user.set_password(validated.new_password)

        db.session.commit()

        # SECURITY: Invalidate all existing sessions after password change
        invalidate_user_sessions(user.id)
        logger.info(f"Password changed successfully for user {user_id}")

        # Issue new tokens for current session
        new_access_token = create_access_token(identity=str(user.id))
        new_refresh_token = create_refresh_token(identity=str(user.id))

        return success_response({
            'message': 'Password changed successfully. All other sessions have been invalidated.',
            'access_token': new_access_token,
            'refresh_token': new_refresh_token
        })

    except Exception as e:
        db.session.rollback()
        logger.error("Error changing password: %s", e)
        return error_response('Internal server error', 500)