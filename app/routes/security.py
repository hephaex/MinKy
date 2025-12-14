from flask import Blueprint, request, jsonify, current_app
from flask_jwt_extended import jwt_required
from app.models.user import User
from app.utils.auth import get_current_user
from app.middleware.security import rate_limit_api, validate_request_security, audit_log
from datetime import datetime, timedelta, timezone
import json
import os

security_bp = Blueprint('security', __name__)

def require_admin(f):
    """Decorator to require admin privileges"""
    from functools import wraps

    @wraps(f)
    def decorated_function(*args, **kwargs):
        user = get_current_user()

        if not user or not user.is_admin:
            return jsonify({'error': 'Admin privileges required'}), 403

        return f(*args, **kwargs)
    return decorated_function

@security_bp.route('/security/status', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("view_security_status")
def get_security_status():
    """Get overall security status and metrics"""
    try:
        # Get rate limiting status
        rate_limit_info = {
            'enabled': True,
            'storage': current_app.config.get('RATELIMIT_STORAGE_URL', 'memory://'),
            'default_limit': current_app.config.get('RATELIMIT_DEFAULT', '1000 per hour')
        }
        
        # Get security configuration
        security_config = {
            'ip_whitelist_enabled': bool(current_app.config.get('IP_WHITELIST')),
            'ip_blacklist_enabled': bool(current_app.config.get('IP_BLACKLIST')),
            'api_key_required': bool(current_app.config.get('API_KEY')),
            'max_concurrent_sessions': current_app.config.get('MAX_CONCURRENT_SESSIONS', 5),
            'max_token_age_hours': current_app.config.get('MAX_TOKEN_AGE_HOURS', 24)
        }
        
        # Security headers check
        security_headers = {
            'x_content_type_options': 'nosniff',
            'x_frame_options': 'DENY',
            'x_xss_protection': '1; mode=block',
            'strict_transport_security': 'enabled',
            'content_security_policy': 'enabled'
        }
        
        return jsonify({
            'status': 'secure',
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'rate_limiting': rate_limit_info,
            'security_config': security_config,
            'security_headers': security_headers,
            'version': '1.0'
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting security status: {str(e)}")
        return jsonify({'error': 'Failed to get security status'}), 500

@security_bp.route('/security/logs', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("10 per minute")
@validate_request_security
@audit_log("view_security_logs")
def get_security_logs():
    """Get recent security events and logs"""
    try:
        # In a production system, you would read from a proper log storage
        # For this demo, we'll simulate some security log entries
        
        limit = min(int(request.args.get('limit', 100)), 1000)
        severity = request.args.get('severity', 'all')  # all, low, medium, high
        hours = int(request.args.get('hours', 24))
        
        # Simulate security log entries
        # In practice, these would come from your logging system
        sample_logs = [
            {
                'timestamp': (datetime.now(timezone.utc) - timedelta(hours=1)).isoformat(),
                'event_type': 'rate_limit_exceeded',
                'severity': 'medium',
                'ip_address': '192.168.1.100',
                'endpoint': '/api/documents',
                'details': 'Rate limit exceeded for IP',
                'action_taken': 'request_blocked'
            },
            {
                'timestamp': (datetime.now(timezone.utc) - timedelta(hours=2)).isoformat(),
                'event_type': 'suspicious_content',
                'severity': 'high',
                'ip_address': '10.0.0.50',
                'endpoint': '/api/comments',
                'details': 'XSS pattern detected in request',
                'action_taken': 'request_blocked'
            },
            {
                'timestamp': (datetime.now(timezone.utc) - timedelta(hours=3)).isoformat(),
                'event_type': 'invalid_jwt',
                'severity': 'low',
                'ip_address': '172.16.0.25',
                'endpoint': '/api/documents/1',
                'details': 'Expired JWT token',
                'action_taken': 'authentication_failed'
            }
        ]
        
        # Filter by severity if specified
        if severity != 'all':
            sample_logs = [log for log in sample_logs if log['severity'] == severity]
        
        # Limit results
        sample_logs = sample_logs[:limit]
        
        return jsonify({
            'logs': sample_logs,
            'total_count': len(sample_logs),
            'filters': {
                'severity': severity,
                'hours': hours,
                'limit': limit
            },
            'generated_at': datetime.now(timezone.utc).isoformat()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting security logs: {str(e)}")
        return jsonify({'error': 'Failed to get security logs'}), 500

@security_bp.route('/security/threats', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("10 per minute")
@validate_request_security
@audit_log("view_threat_analysis")
def get_threat_analysis():
    """Get threat analysis and blocked IPs"""
    try:
        hours = int(request.args.get('hours', 24))
        
        # In a real system, this would analyze actual security logs
        threat_summary = {
            'total_threats_detected': 15,
            'blocked_ips': [
                {
                    'ip': '192.168.1.100',
                    'threat_count': 8,
                    'first_seen': (datetime.now(timezone.utc) - timedelta(hours=6)).isoformat(),
                    'last_seen': (datetime.now(timezone.utc) - timedelta(minutes=30)).isoformat(),
                    'threat_types': ['rate_limit_exceeded', 'suspicious_content']
                },
                {
                    'ip': '10.0.0.50',
                    'threat_count': 3,
                    'first_seen': (datetime.now(timezone.utc) - timedelta(hours=2)).isoformat(),
                    'last_seen': (datetime.now(timezone.utc) - timedelta(hours=1)).isoformat(),
                    'threat_types': ['xss_attempt', 'sql_injection_attempt']
                }
            ],
            'threat_types': {
                'rate_limit_exceeded': 8,
                'suspicious_content': 4,
                'xss_attempt': 2,
                'sql_injection_attempt': 1
            },
            'geographic_distribution': {
                'US': 10,
                'CN': 3,
                'RU': 2
            }
        }
        
        return jsonify({
            'threat_analysis': threat_summary,
            'analysis_period_hours': hours,
            'generated_at': datetime.now(timezone.utc).isoformat()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting threat analysis: {str(e)}")
        return jsonify({'error': 'Failed to get threat analysis'}), 500

@security_bp.route('/security/config', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("5 per minute")
@validate_request_security
@audit_log("view_security_config")
def get_security_config():
    """Get current security configuration"""
    try:
        config = {
            'rate_limiting': {
                'enabled': True,
                'default_limit': current_app.config.get('RATELIMIT_DEFAULT', '1000 per hour'),
                'storage_type': current_app.config.get('RATELIMIT_STORAGE_URL', 'memory://').split('://')[0]
            },
            'ip_filtering': {
                'whitelist_enabled': bool(current_app.config.get('IP_WHITELIST')),
                'blacklist_enabled': bool(current_app.config.get('IP_BLACKLIST')),
                'whitelist_count': len(current_app.config.get('IP_WHITELIST', [])),
                'blacklist_count': len(current_app.config.get('IP_BLACKLIST', []))
            },
            'authentication': {
                'jwt_expiry_hours': current_app.config.get('JWT_ACCESS_TOKEN_EXPIRES', timedelta(hours=24)).total_seconds() / 3600,
                'refresh_token_expiry_days': current_app.config.get('JWT_REFRESH_TOKEN_EXPIRES', timedelta(days=30)).days,
                'max_concurrent_sessions': current_app.config.get('MAX_CONCURRENT_SESSIONS', 5)
            },
            'file_uploads': {
                'max_size_mb': current_app.config.get('MAX_CONTENT_LENGTH', 16 * 1024 * 1024) / (1024 * 1024),
                'allowed_extensions': current_app.config.get('UPLOAD_ALLOWED_EXTENSIONS', [])
            },
            'security_headers': {
                'enabled': True,
                'csp_enabled': True,
                'hsts_enabled': True
            }
        }
        
        return jsonify({
            'security_config': config,
            'generated_at': datetime.now(timezone.utc).isoformat()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error getting security config: {str(e)}")
        return jsonify({'error': 'Failed to get security config'}), 500

@security_bp.route('/security/config', methods=['PUT'])
@jwt_required()
@require_admin
@rate_limit_api("2 per minute")
@validate_request_security
@audit_log("update_security_config")
def update_security_config():
    """Update security configuration (limited updates for safety)"""
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'Request body required'}), 400
        
        updated_settings = []
        
        # Only allow updating certain safe settings
        safe_updates = {
            'rate_limit_default': 'RATELIMIT_DEFAULT',
            'max_concurrent_sessions': 'MAX_CONCURRENT_SESSIONS'
        }
        
        for key, config_key in safe_updates.items():
            if key in data:
                # In a real application, you'd want to persist these changes
                # to a configuration file or database
                current_app.config[config_key] = data[key]
                updated_settings.append(key)
        
        if not updated_settings:
            return jsonify({'message': 'No valid settings to update'}), 400
        
        return jsonify({
            'message': 'Security configuration updated',
            'updated_settings': updated_settings,
            'note': 'Changes may require application restart to take full effect',
            'updated_at': datetime.now(timezone.utc).isoformat()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error updating security config: {str(e)}")
        return jsonify({'error': 'Failed to update security config'}), 500

@security_bp.route('/security/ip-management', methods=['POST'])
@jwt_required()
@require_admin
@rate_limit_api("5 per minute")
@validate_request_security
@audit_log("manage_ip_lists")
def manage_ip_lists():
    """Add or remove IPs from whitelist/blacklist"""
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'Request body required'}), 400
        
        action = data.get('action')  # 'add_to_whitelist', 'remove_from_whitelist', 'add_to_blacklist', 'remove_from_blacklist'
        ip_address = data.get('ip_address')
        
        if not action or not ip_address:
            return jsonify({'error': 'action and ip_address required'}), 400
        
        # Validate IP address
        import ipaddress
        try:
            ipaddress.ip_address(ip_address)
        except ValueError:
            return jsonify({'error': 'Invalid IP address format'}), 400
        
        # In a real application, you'd update persistent storage
        # For demo purposes, we'll just log the action
        current_app.logger.info(f"IP Management: {action} for {ip_address}")
        
        return jsonify({
            'message': f'IP {ip_address} {action.replace("_", " ")}',
            'ip_address': ip_address,
            'action': action,
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'note': 'Changes may require application restart to take effect'
        })
        
    except Exception as e:
        current_app.logger.error(f"Error managing IP lists: {str(e)}")
        return jsonify({'error': 'Failed to manage IP lists'}), 500

@security_bp.route('/security/scan', methods=['POST'])
@jwt_required()
@require_admin
@rate_limit_api("1 per hour")
@validate_request_security
@audit_log("security_scan")
def run_security_scan():
    """Run a security scan on the application"""
    try:
        # This would run various security checks
        scan_results = {
            'scan_id': f"scan_{datetime.now(timezone.utc).strftime('%Y%m%d_%H%M%S')}",
            'started_at': datetime.now(timezone.utc).isoformat(),
            'checks': [
                {
                    'name': 'Rate Limiting Configuration',
                    'status': 'pass',
                    'details': 'Rate limiting is properly configured'
                },
                {
                    'name': 'Security Headers',
                    'status': 'pass',
                    'details': 'All security headers are present'
                },
                {
                    'name': 'JWT Configuration',
                    'status': 'pass',
                    'details': 'JWT tokens have appropriate expiry times'
                },
                {
                    'name': 'Input Validation',
                    'status': 'pass',
                    'details': 'Input validation is active on all endpoints'
                },
                {
                    'name': 'File Upload Security',
                    'status': 'warning',
                    'details': 'Consider implementing virus scanning for uploads'
                }
            ],
            'summary': {
                'total_checks': 5,
                'passed': 4,
                'warnings': 1,
                'failed': 0,
                'score': 85
            }
        }
        
        return jsonify({
            'security_scan': scan_results,
            'completed_at': datetime.now(timezone.utc).isoformat()
        })
        
    except Exception as e:
        current_app.logger.error(f"Error running security scan: {str(e)}")
        return jsonify({'error': 'Failed to run security scan'}), 500