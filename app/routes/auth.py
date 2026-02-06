from flask import Blueprint, request, jsonify, Response
from flask_jwt_extended import create_access_token, create_refresh_token, jwt_required, get_jwt_identity
from email_validator import validate_email, EmailNotValidError
from pydantic import ValidationError
from app import db, limiter
from app.models.user import User
from app.schemas.auth import RegisterRequest, LoginRequest
from app.utils.validation import format_validation_errors
from app.utils.responses import success_response, error_response

auth_bp = Blueprint('auth', __name__)

@auth_bp.route('/register', methods=['POST'])
@limiter.limit("20 per minute")
def register() -> Response | tuple[Response, int]:
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
        return error_response(str(e), 500)

@auth_bp.route('/login', methods=['POST'])
@limiter.limit("10 per minute")
def login() -> Response | tuple[Response, int]:
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
        return error_response(str(e), 500)

@auth_bp.route('/refresh', methods=['POST'])
@limiter.limit("30 per hour")
@jwt_required(refresh=True)
def refresh() -> Response | tuple[Response, int]:
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
        return error_response(str(e), 500)

@auth_bp.route('/me', methods=['GET'])
@jwt_required()
def get_current_user() -> Response | tuple[Response, int]:
    try:
        current_user_id = get_jwt_identity()
        user = db.session.get(User, current_user_id)

        if not user:
            return error_response('User not found', 404)

        return success_response({
            'user': user.to_dict(include_sensitive=True)
        })

    except Exception as e:
        return error_response(str(e), 500)

@auth_bp.route('/profile', methods=['PUT'])
@jwt_required()
def update_profile() -> Response | tuple[Response, int]:
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
        return error_response(str(e), 500)