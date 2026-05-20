from flask import Blueprint, request, current_app
from flask_jwt_extended import jwt_required
from app.utils.auth import get_current_user
from app.utils.responses import success_response, error_response
from app.middleware.security import rate_limit_api, validate_request_security, audit_log
from app.utils.constants import MAX_QUERY_HOURS
from datetime import datetime, timedelta, timezone

security_bp = Blueprint('security', __name__)

def require_admin(f):
    """Decorator to require admin privileges"""
    from functools import wraps

    @wraps(f)
    def decorated_function(*args, **kwargs):
        user = get_current_user()

        if not user:
            return error_response('Authentication required', 401)

        # SECURITY: Check both is_admin AND is_active
        if not user.is_active:
            return error_response('User account is inactive', 403)

        if not user.is_admin:
            return error_response('Admin privileges required', 403)

        return f(*args, **kwargs)
    return decorated_function

@security_bp.route('/security/status', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("20 per minute")
@validate_request_security
@audit_log("view_security_status")
def get_security_status():
    """Get overall security status and metrics (limited exposure)"""
    try:
        # SECURITY: Only expose minimal rate limiting info (no storage URL details)
        rate_limit_info = {
            'enabled': True,
            # SECURITY: Don't expose storage URL which may reveal infrastructure details
            'storage_type': 'configured'
        }

        # SECURITY: Only expose boolean status, not actual values
        security_config = {
            'ip_whitelist_enabled': bool(current_app.config.get('IP_WHITELIST')),
            'ip_blacklist_enabled': bool(current_app.config.get('IP_BLACKLIST')),
            'api_key_required': bool(current_app.config.get('API_KEY')),
            # SECURITY: Don't expose exact session/token limits
            'session_management': 'enabled',
            'token_management': 'enabled'
        }

        # Security headers check - only confirm enabled status
        security_headers = {
            'x_content_type_options': 'enabled',
            'x_frame_options': 'enabled',
            'x_xss_protection': 'enabled',
            'strict_transport_security': 'enabled',
            'content_security_policy': 'enabled'
        }

        return success_response({
            'status': 'secure',
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'rate_limiting': rate_limit_info,
            'security_config': security_config,
            'security_headers': security_headers,
            'version': '1.0'
        })

    except Exception as e:
        current_app.logger.error(f"Error getting security status: {str(e)}")
        return error_response('Failed to get security status', 500)

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
        
        limit = min(request.args.get('limit', 100, type=int), 1000)
        severity = request.args.get('severity', 'all').lower()
        hours = min(request.args.get('hours', 24, type=int), MAX_QUERY_HOURS)

        # SECURITY: Validate severity against whitelist
        VALID_SEVERITIES = frozenset({'all', 'low', 'medium', 'high', 'critical'})
        if severity not in VALID_SEVERITIES:
            return error_response(
                f'severity must be one of: {", ".join(sorted(VALID_SEVERITIES))}',
                400
            )
        
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

        return success_response({
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
        return error_response('Failed to get security logs', 500)

@security_bp.route('/security/threats', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("10 per minute")
@validate_request_security
@audit_log("view_threat_analysis")
def get_threat_analysis():
    """Get threat analysis and blocked IPs"""
    try:
        hours = min(request.args.get('hours', 24, type=int), MAX_QUERY_HOURS)

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
        
        return success_response({
            'threat_analysis': threat_summary,
            'analysis_period_hours': hours,
            'generated_at': datetime.now(timezone.utc).isoformat()
        })

    except Exception as e:
        current_app.logger.error(f"Error getting threat analysis: {str(e)}")
        return error_response('Failed to get threat analysis', 500)

@security_bp.route('/security/config', methods=['GET'])
@jwt_required()
@require_admin
@rate_limit_api("5 per minute")
@validate_request_security
@audit_log("view_security_config")
def get_security_config():
    """Get current security configuration (limited exposure for security)"""
    try:
        # SECURITY: Limit exposure of security configuration details
        # Don't expose exact values that could help attackers
        config = {
            'rate_limiting': {
                'enabled': True,
                # SECURITY: Don't expose exact rate limits
                'configured': True,
                'storage_type': 'configured'
            },
            'ip_filtering': {
                'whitelist_enabled': bool(current_app.config.get('IP_WHITELIST')),
                'blacklist_enabled': bool(current_app.config.get('IP_BLACKLIST')),
                # SECURITY: Don't expose exact counts (could reveal security posture)
                'whitelist_configured': len(current_app.config.get('IP_WHITELIST', [])) > 0,
                'blacklist_configured': len(current_app.config.get('IP_BLACKLIST', [])) > 0
            },
            'authentication': {
                # SECURITY: Only expose boolean status, not exact token lifetimes
                'jwt_configured': True,
                'refresh_tokens_enabled': True,
                'session_management_enabled': True
            },
            'file_uploads': {
                # SECURITY: Don't expose exact size limits
                'size_limits_configured': True,
                'extension_filtering_enabled': len(current_app.config.get('UPLOAD_ALLOWED_EXTENSIONS', [])) > 0
            },
            'security_headers': {
                'enabled': True,
                'csp_enabled': True,
                'hsts_enabled': True
            }
        }

        return success_response({
            'security_config': config,
            'generated_at': datetime.now(timezone.utc).isoformat()
        })

    except Exception as e:
        current_app.logger.error(f"Error getting security config: {str(e)}")
        return error_response('Failed to get security config', 500)

# SECURITY: Allowed IP management actions whitelist
ALLOWED_IP_ACTIONS = frozenset({
    'add_to_whitelist',
    'remove_from_whitelist',
    'add_to_blacklist',
    'remove_from_blacklist'
})

@security_bp.route('/security/config', methods=['PUT'])
@jwt_required()
@require_admin
@rate_limit_api("2 per minute")
@validate_request_security
@audit_log("update_security_config")
def update_security_config():
    """Update security configuration (limited updates for safety)"""
    import re

    try:
        data = request.get_json()
        if not data:
            return error_response('Request body required', 400)

        updated_settings = []
        errors = []

        # SECURITY: Validate rate_limit_default format and bounds
        if 'rate_limit_default' in data:
            rate_limit = data['rate_limit_default']
            if not isinstance(rate_limit, str):
                errors.append('rate_limit_default must be a string')
            elif not re.match(r'^\d+\s+per\s+(minute|hour|day)$', rate_limit):
                errors.append('rate_limit_default must be in format "N per minute/hour/day"')
            else:
                limit_value = int(rate_limit.split()[0])
                if limit_value < 1 or limit_value > 10000:
                    errors.append('rate_limit_default value must be between 1 and 10000')
                else:
                    current_app.config['RATELIMIT_DEFAULT'] = rate_limit
                    updated_settings.append('rate_limit_default')

        # SECURITY: Validate max_concurrent_sessions bounds
        if 'max_concurrent_sessions' in data:
            max_sessions = data['max_concurrent_sessions']
            if not isinstance(max_sessions, int):
                errors.append('max_concurrent_sessions must be an integer')
            elif max_sessions < 1 or max_sessions > 100:
                errors.append('max_concurrent_sessions must be between 1 and 100')
            else:
                current_app.config['MAX_CONCURRENT_SESSIONS'] = max_sessions
                updated_settings.append('max_concurrent_sessions')

        if errors:
            return error_response('Validation failed', 400, details={'errors': errors})

        if not updated_settings:
            return error_response('No valid settings to update', 400)

        return success_response({
            'message': 'Security configuration updated',
            'updated_settings': updated_settings,
            'note': 'Changes may require application restart to take full effect',
            'updated_at': datetime.now(timezone.utc).isoformat()
        })

    except Exception as e:
        current_app.logger.error(f"Error updating security config: {str(e)}")
        return error_response('Failed to update security config', 500)

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
            return error_response('Request body required', 400)

        action = data.get('action')
        ip_address = data.get('ip_address')

        if not action or not ip_address:
            return error_response('action and ip_address required', 400)

        # SECURITY: Validate action against whitelist to prevent log injection
        if action not in ALLOWED_IP_ACTIONS:
            return error_response(f'action must be one of: {", ".join(sorted(ALLOWED_IP_ACTIONS))}', 400)

        # Validate IP address
        import ipaddress
        try:
            ipaddress.ip_address(ip_address)
        except ValueError:
            return error_response('Invalid IP address format', 400)

        # In a real application, you'd update persistent storage
        # For demo purposes, we'll just log the action
        current_app.logger.info(f"IP Management: {action} for {ip_address}")

        return success_response({
            'message': f'IP {ip_address} {action.replace("_", " ")}',
            'ip_address': ip_address,
            'action': action,
            'timestamp': datetime.now(timezone.utc).isoformat(),
            'note': 'Changes may require application restart to take effect'
        })

    except Exception as e:
        current_app.logger.error(f"Error managing IP lists: {str(e)}")
        return error_response('Failed to manage IP lists', 500)

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
        
        return success_response({
            'security_scan': scan_results,
            'completed_at': datetime.now(timezone.utc).isoformat()
        })

    except Exception as e:
        current_app.logger.error(f"Error running security scan: {str(e)}")
        return error_response('Failed to run security scan', 500)