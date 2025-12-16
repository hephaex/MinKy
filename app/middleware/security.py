from flask import request, jsonify, current_app
from flask_jwt_extended import get_jwt_identity, verify_jwt_in_request
from functools import wraps
from datetime import datetime, timezone
from app import limiter
import ipaddress
import re

class SecurityMiddleware:
    """Advanced security middleware for API protection"""
    
    # Security headers
    SECURITY_HEADERS = {
        'X-Content-Type-Options': 'nosniff',
        'X-Frame-Options': 'DENY',
        'X-XSS-Protection': '1; mode=block',
        'Strict-Transport-Security': 'max-age=31536000; includeSubDomains',
        'Content-Security-Policy': "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'",
        'Referrer-Policy': 'strict-origin-when-cross-origin',
        'Permissions-Policy': 'geolocation=(), microphone=(), camera=()'
    }
    
    # Suspicious patterns in requests
    SUSPICIOUS_PATTERNS = [
        r'<script.*?>.*?</script>',  # XSS attempts
        r'javascript:',  # JavaScript execution
        r'data:text/html',  # Data URL XSS
        r'union.*select',  # SQL injection
        r'drop\s+table',  # SQL injection
        r'exec\(\s*["\']',  # Code execution
        r'eval\(\s*["\']',  # Code execution
        r'../../../',  # Path traversal
        r'\\x[0-9a-f]{2}',  # Hex encoding
        r'%[0-9a-f]{2}',  # URL encoding of suspicious chars
    ]
    
    @staticmethod
    def add_security_headers(response):
        """Add security headers to response"""
        for header, value in SecurityMiddleware.SECURITY_HEADERS.items():
            response.headers[header] = value
        return response
    
    @staticmethod
    def validate_request_content(request_data):
        """Validate request content for suspicious patterns"""
        if not request_data:
            return True
        
        # Convert to string for pattern matching
        content = str(request_data).lower()
        
        for pattern in SecurityMiddleware.SUSPICIOUS_PATTERNS:
            if re.search(pattern, content, re.IGNORECASE):
                current_app.logger.warning(f"Suspicious pattern detected: {pattern} in request from {request.remote_addr}")
                return False
        
        return True
    
    @staticmethod
    def check_ip_whitelist(ip_address):
        """Check if IP is in whitelist (if configured)"""
        whitelist = current_app.config.get('IP_WHITELIST', [])
        if not whitelist:
            return True  # No whitelist configured
        
        try:
            ip = ipaddress.ip_address(ip_address)
            for allowed_ip in whitelist:
                if ip in ipaddress.ip_network(allowed_ip, strict=False):
                    return True
            return False
        except ValueError:
            return False
    
    @staticmethod
    def check_ip_blacklist(ip_address):
        """Check if IP is in blacklist"""
        blacklist = current_app.config.get('IP_BLACKLIST', [])
        if not blacklist:
            return True  # No blacklist configured
        
        try:
            ip = ipaddress.ip_address(ip_address)
            for blocked_ip in blacklist:
                if ip in ipaddress.ip_network(blocked_ip, strict=False):
                    return False
            return True
        except ValueError:
            return True
    
    @staticmethod
    def log_security_event(event_type, details, severity='medium'):
        """Log security events for monitoring"""
        log_entry = {
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'event_type': event_type,
            'ip_address': request.remote_addr,
            'user_agent': request.headers.get('User-Agent', 'Unknown'),
            'endpoint': request.endpoint,
            'method': request.method,
            'details': details,
            'severity': severity
        }
        
        # Log to application logger
        if severity == 'high':
            current_app.logger.error(f"Security Alert: {log_entry}")
        elif severity == 'medium':
            current_app.logger.warning(f"Security Warning: {log_entry}")
        else:
            current_app.logger.info(f"Security Info: {log_entry}")
        
        # Store in database for analysis (optional)
        # You could create a SecurityLog model to store these events

# Rate limiting decorators for different endpoint types
def rate_limit_auth(limit="5 per minute"):
    """Rate limiting for authentication endpoints"""
    return limiter.limit(limit)

def rate_limit_api(limit="100 per minute"):
    """Rate limiting for general API endpoints"""
    return limiter.limit(limit)

def rate_limit_search(limit="30 per minute"):
    """Rate limiting for search endpoints"""
    return limiter.limit(limit)

def rate_limit_export(limit="10 per minute"):
    """Rate limiting for export endpoints"""
    return limiter.limit(limit)

def rate_limit_upload(limit="20 per hour"):
    """Rate limiting for file upload endpoints"""
    return limiter.limit(limit)

# Advanced security decorators
def require_secure_headers(f):
    """Decorator to add security headers to responses"""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        response = f(*args, **kwargs)
        if hasattr(response, 'headers'):
            return SecurityMiddleware.add_security_headers(response)
        return response
    return decorated_function

def validate_request_security(f):
    """Decorator to validate request for security threats"""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        # Check IP whitelist/blacklist
        client_ip = request.remote_addr
        
        if not SecurityMiddleware.check_ip_whitelist(client_ip):
            SecurityMiddleware.log_security_event(
                'ip_not_whitelisted',
                f'IP {client_ip} not in whitelist',
                'high'
            )
            return jsonify({'error': 'Access denied'}), 403
        
        if not SecurityMiddleware.check_ip_blacklist(client_ip):
            SecurityMiddleware.log_security_event(
                'ip_blacklisted',
                f'IP {client_ip} is blacklisted',
                'high'
            )
            return jsonify({'error': 'Access denied'}), 403
        
        # Validate request content
        request_data = None
        if request.is_json:
            request_data = request.get_json(silent=True)
        elif request.form:
            request_data = request.form.to_dict()
        elif request.args:
            request_data = request.args.to_dict()
        
        if not SecurityMiddleware.validate_request_content(request_data):
            SecurityMiddleware.log_security_event(
                'suspicious_content',
                'Suspicious patterns detected in request',
                'high'
            )
            return jsonify({'error': 'Invalid request content'}), 400
        
        return f(*args, **kwargs)
    return decorated_function

def require_api_key(f):
    """Decorator to require API key for sensitive operations"""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        api_key = request.headers.get('X-API-Key')
        expected_key = current_app.config.get('API_KEY')
        
        if expected_key and api_key != expected_key:
            SecurityMiddleware.log_security_event(
                'invalid_api_key',
                'Invalid or missing API key',
                'medium'
            )
            return jsonify({'error': 'Invalid API key'}), 401
        
        return f(*args, **kwargs)
    return decorated_function

def enhanced_jwt_required(f):
    """Enhanced JWT validation with additional security checks"""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        try:
            verify_jwt_in_request()
            current_user_id = get_jwt_identity()
            
            # Check for concurrent sessions (optional)
            session_limit = current_app.config.get('MAX_CONCURRENT_SESSIONS', 5)
            if session_limit:
                # This would require tracking active sessions
                pass
            
            # Check for token age
            max_token_age = current_app.config.get('MAX_TOKEN_AGE_HOURS', 24)
            # Could implement token age checking here
            
            return f(*args, **kwargs)
            
        except Exception as e:
            SecurityMiddleware.log_security_event(
                'jwt_validation_failed',
                f'JWT validation failed: {str(e)}',
                'medium'
            )
            return jsonify({'error': 'Authentication failed'}), 401
    
    return decorated_function

def audit_log(action):
    """Decorator to log user actions for audit trail"""
    def decorator(f):
        @wraps(f)
        def decorated_function(*args, **kwargs):
            start_time = datetime.now(timezone.utc)
            
            try:
                result = f(*args, **kwargs)
                
                # Log successful action
                user_id = None
                try:
                    user_id = get_jwt_identity()
                except Exception:
                    pass
                
                audit_entry = {
                    'timestamp': start_time.isoformat(),
                    'user_id': user_id,
                    'action': action,
                    'endpoint': request.endpoint,
                    'method': request.method,
                    'ip_address': request.remote_addr,
                    'success': True,
                    'duration_ms': (datetime.now(timezone.utc) - start_time).total_seconds() * 1000
                }
                
                current_app.logger.info(f"Audit: {audit_entry}")
                return result
                
            except Exception as e:
                # Log failed action
                audit_entry = {
                    'timestamp': start_time.isoformat(),
                    'action': action,
                    'endpoint': request.endpoint,
                    'method': request.method,
                    'ip_address': request.remote_addr,
                    'success': False,
                    'error': str(e),
                    'duration_ms': (datetime.now(timezone.utc) - start_time).total_seconds() * 1000
                }
                
                current_app.logger.warning(f"Audit Failed: {audit_entry}")
                raise
                
        return decorated_function
    return decorator

# Input validation helpers
def validate_file_upload(file, allowed_extensions=None, max_size_mb=10):
    """Validate uploaded file for security"""
    if not file or not file.filename:
        return False, "No file provided"
    
    # Check file extension
    if allowed_extensions:
        ext = file.filename.rsplit('.', 1)[1].lower() if '.' in file.filename else ''
        if ext not in allowed_extensions:
            return False, f"File type not allowed. Allowed: {', '.join(allowed_extensions)}"
    
    # Check file size
    file.seek(0, 2)  # Seek to end
    size = file.tell()
    file.seek(0)  # Reset to beginning
    
    max_size_bytes = max_size_mb * 1024 * 1024
    if size > max_size_bytes:
        return False, f"File too large. Maximum size: {max_size_mb}MB"
    
    # Check for suspicious file content (basic)
    file_header = file.read(1024)
    file.seek(0)
    
    # Check for executable signatures
    suspicious_signatures = [
        b'\x4d\x5a',  # PE executable
        b'\x7f\x45\x4c\x46',  # ELF executable
        b'\xfe\xed\xfa',  # Mach-O executable
    ]
    
    for sig in suspicious_signatures:
        if file_header.startswith(sig):
            return False, "Executable files not allowed"
    
    return True, "File is valid"

def sanitize_filename(filename):
    """Sanitize filename for safe storage"""
    # Remove dangerous characters
    filename = re.sub(r'[^\w\s.-]', '', filename)
    # Limit length
    filename = filename[:255]
    # Prevent hidden files
    if filename.startswith('.'):
        filename = 'file' + filename
    return filename