"""
Git Integration Routes
Provides API endpoints for Git operations like pull, push, and sync
"""

import os
import re
import subprocess
import logging
from functools import wraps
from urllib.parse import urlparse
from flask import Blueprint, jsonify, request, current_app
from flask_jwt_extended import jwt_required
from app import limiter

logger = logging.getLogger(__name__)

git_bp = Blueprint('git', __name__)


def validate_origin(f):
    """CSRF protection via Origin/Referer header validation for state-changing operations"""
    @wraps(f)
    def decorated_function(*args, **kwargs):
        # Get allowed origins from config
        allowed_origins = current_app.config.get('CORS_ORIGINS', ['http://localhost:3000'])
        if isinstance(allowed_origins, str):
            allowed_origins = [allowed_origins]

        # Check Origin header first, then Referer
        origin = request.headers.get('Origin')
        referer = request.headers.get('Referer')

        request_origin = origin
        if not request_origin and referer:
            parsed = urlparse(referer)
            request_origin = f"{parsed.scheme}://{parsed.netloc}"

        # SECURITY: Require Origin or Referer header to prevent CSRF bypass
        if not request_origin:
            logger.warning("CSRF validation failed: no Origin or Referer header")
            return jsonify({'error': 'Origin header required'}), 403

        # Normalize and check against allowed origins
        if not any(request_origin.rstrip('/') == allowed.rstrip('/') for allowed in allowed_origins):
            logger.warning(f"CSRF validation failed: origin {request_origin} not in allowed list")
            return jsonify({'error': 'Invalid request origin'}), 403

        return f(*args, **kwargs)
    return decorated_function

# Maximum length for commit messages to prevent abuse
MAX_COMMIT_MESSAGE_LENGTH = 500

# Pattern to detect potentially dangerous characters in commit messages
DANGEROUS_PATTERNS = [
    r'[`$]',           # Shell substitution
    r'[\x00-\x1f]',    # Control characters (except allowed ones)
    r'\\x[0-9a-fA-F]', # Hex escape sequences
]


def sanitize_git_url(url: str) -> str:
    """Remove credentials from git URL to prevent exposure"""
    if not url:
        return ''
    # Remove username:password@ from URLs like https://user:pass@github.com/...
    return re.sub(r'://[^@]+@', '://', url)


def sanitize_commit_message(message):
    """
    Sanitize commit message to prevent command injection.
    Returns sanitized message or None if message is invalid.
    """
    if not message or not isinstance(message, str):
        return None

    # Truncate to max length
    message = message[:MAX_COMMIT_MESSAGE_LENGTH]

    # Check for dangerous patterns
    for pattern in DANGEROUS_PATTERNS:
        if re.search(pattern, message):
            logger.warning("Potentially dangerous pattern detected in commit message")
            # Remove the dangerous characters instead of rejecting entirely
            message = re.sub(pattern, '', message)

    # Strip leading/trailing whitespace
    message = message.strip()

    # SECURITY: Prevent argument injection - messages starting with '-' could be
    # interpreted as git flags (e.g., "-m" or "--amend")
    if message.startswith('-'):
        message = '_' + message

    # Ensure message is not empty after sanitization
    if not message:
        return None

    return message


def get_safe_backup_dir():
    """Get validated absolute backup directory path"""
    backup_dir = current_app.config.get('BACKUP_DIR', './backup')

    # SECURITY: Convert to absolute path to prevent relative path issues
    backup_dir = os.path.abspath(backup_dir)

    # SECURITY: Validate backup directory exists and is a directory
    if not os.path.isdir(backup_dir):
        raise ValueError(f"Backup directory does not exist: {backup_dir}")

    return backup_dir


def sanitize_git_error(stderr: str) -> str:
    """Sanitize git error output to prevent information disclosure"""
    if not stderr:
        return ''

    # SECURITY: Remove potentially sensitive information
    # Remove full file paths (keep just filename)
    sanitized = re.sub(r'/[^\s]+/([^\s/]+)', r'\1', stderr)

    # Remove usernames from URLs
    sanitized = re.sub(r'://[^@]+@', '://', sanitized)

    # Map common errors to generic messages
    error_mappings = {
        'not a git repository': 'Repository not initialized',
        'authentication failed': 'Authentication required',
        'could not resolve host': 'Remote server unreachable',
        'permission denied': 'Access denied',
    }

    for pattern, friendly_msg in error_mappings.items():
        if pattern.lower() in sanitized.lower():
            return friendly_msg

    # Truncate to prevent excessive information
    return sanitized[:200] if len(sanitized) > 200 else sanitized


def run_git_command(command, cwd=None):
    """Execute a git command and return the result"""
    try:
        if cwd is None:
            cwd = get_safe_backup_dir()
        else:
            # SECURITY: Validate provided cwd is absolute
            cwd = os.path.abspath(cwd)

        # Ensure the directory exists
        os.makedirs(cwd, exist_ok=True)

        # Validate that all command parts are strings (prevent injection)
        if not all(isinstance(part, str) for part in command):
            return {
                'success': False,
                'stdout': '',
                'stderr': 'Invalid command format',
                'returncode': -1
            }

        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=30,
            shell=False  # Important: Never use shell=True
        )

        return {
            'success': result.returncode == 0,
            'stdout': result.stdout.strip(),
            'stderr': result.stderr.strip(),
            'returncode': result.returncode
        }
    except subprocess.TimeoutExpired:
        return {
            'success': False,
            'stdout': '',
            'stderr': 'Git command timed out',
            'returncode': -1
        }
    except Exception as e:
        logger.error(f"Git command error: {e}")
        return {
            'success': False,
            'stdout': '',
            'stderr': 'Git command failed',
            'returncode': -1
        }


@git_bp.route('/git/status', methods=['GET'])
@limiter.limit("30 per minute")  # SECURITY: Rate limiting
@jwt_required()
def git_status():
    """Get git repository status"""
    try:
        backup_dir = get_safe_backup_dir()
        
        # Check if it's a git repository
        result = run_git_command(['git', 'status', '--porcelain'], backup_dir)
        
        if not result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository or git not available',
                'details': sanitize_git_error(result['stderr'])
            }), 400
        
        # Get branch info
        branch_result = run_git_command(['git', 'branch', '--show-current'], backup_dir)
        current_branch = branch_result['stdout'] if branch_result['success'] else 'unknown'
        
        # Count changes
        status_lines = result['stdout'].split('\n') if result['stdout'] else []
        modified_files = len([line for line in status_lines if line.strip()])
        
        return jsonify({
            'success': True,
            'current_branch': current_branch,
            'modified_files': modified_files,
            'has_changes': modified_files > 0,
            'status_output': result['stdout']
        })
        
    except Exception as e:
        logger.error("Git status error: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500


@git_bp.route('/git/pull', methods=['POST'])
@validate_origin
@jwt_required()
@limiter.limit("10 per hour")
def git_pull():
    """Pull changes from remote repository (requires authentication)"""
    try:
        backup_dir = get_safe_backup_dir()
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': sanitize_git_error(status_result['stderr'])
            }), 400

        # Perform git pull
        pull_result = run_git_command(['git', 'pull'], backup_dir)

        if pull_result['success']:
            return jsonify({
                'success': True,
                'message': 'Successfully pulled changes from repository',
                'output': pull_result['stdout'],
                'details': sanitize_git_error(pull_result['stderr']) if pull_result['stderr'] else None
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Git pull failed',
                'details': sanitize_git_error(pull_result['stderr'])
            }), 400
            
    except Exception as e:
        logger.error("Git pull error: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500


@git_bp.route('/git/push', methods=['POST'])
@validate_origin
@jwt_required()
@limiter.limit("10 per hour")
def git_push():
    """Push changes to remote repository (requires authentication)"""
    try:
        # SECURITY: Require admin privileges for git write operations
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required for git push'}), 403

        backup_dir = get_safe_backup_dir()
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': sanitize_git_error(status_result['stderr'])
            }), 400

        # Add all changes
        add_result = run_git_command(['git', 'add', '.'], backup_dir)
        if not add_result['success']:
            return jsonify({
                'success': False,
                'error': 'Failed to stage changes',
                'details': sanitize_git_error(add_result['stderr'])
            }), 400
        
        # Check if there are changes to commit
        status_check = run_git_command(['git', 'status', '--porcelain'], backup_dir)
        if not status_check['stdout'].strip():
            return jsonify({
                'success': True,
                'message': 'No changes to push',
                'output': 'Working tree clean'
            })
        
        # Commit changes with sanitized message
        raw_message = request.json.get('message', 'Auto-commit from minky application') if request.is_json else 'Auto-commit from minky application'
        commit_message = sanitize_commit_message(raw_message)
        if not commit_message:
            commit_message = 'Auto-commit from minky application'

        commit_result = run_git_command(['git', 'commit', '-m', commit_message], backup_dir)
        
        if not commit_result['success'] and 'nothing to commit' not in commit_result['stdout']:
            return jsonify({
                'success': False,
                'error': 'Failed to commit changes',
                'details': sanitize_git_error(commit_result['stderr'])
            }), 400

        # Push to remote
        push_result = run_git_command(['git', 'push'], backup_dir)

        if push_result['success']:
            return jsonify({
                'success': True,
                'message': 'Successfully pushed changes to repository',
                'output': push_result['stdout'],
                'details': sanitize_git_error(push_result['stderr']) if push_result['stderr'] else None
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Git push failed',
                'details': sanitize_git_error(push_result['stderr'])
            }), 400
            
    except Exception as e:
        logger.error("Git push error: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500


@git_bp.route('/git/sync', methods=['POST'])
@validate_origin
@jwt_required()
@limiter.limit("5 per hour")
def git_sync():
    """Sync with remote repository - pull then push (requires authentication)"""
    try:
        # SECURITY: Require admin privileges for git write operations
        from app.utils.auth import get_current_user
        user = get_current_user()
        if not user or not user.is_active:
            return jsonify({'error': 'Authentication required'}), 401
        if not user.is_admin:
            return jsonify({'error': 'Admin privileges required for git sync'}), 403

        backup_dir = get_safe_backup_dir()
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': sanitize_git_error(status_result['stderr'])
            }), 400

        results = []

        # Step 1: Pull changes
        pull_result = run_git_command(['git', 'pull'], backup_dir)
        results.append({
            'operation': 'pull',
            'success': pull_result['success'],
            'output': pull_result['stdout'],
            'error': sanitize_git_error(pull_result['stderr']) if not pull_result['success'] else None
        })
        
        if not pull_result['success']:
            return jsonify({
                'success': False,
                'error': 'Git pull failed during sync',
                'results': results
            }), 400
        
        # Step 2: Add all changes
        add_result = run_git_command(['git', 'add', '.'], backup_dir)
        if not add_result['success']:
            results.append({
                'operation': 'add',
                'success': False,
                'error': sanitize_git_error(add_result['stderr'])
            })
            return jsonify({
                'success': False,
                'error': 'Failed to stage changes during sync',
                'results': results
            }), 400
        
        # Step 3: Check if there are changes to commit
        status_check = run_git_command(['git', 'status', '--porcelain'], backup_dir)
        if not status_check['stdout'].strip():
            results.append({
                'operation': 'commit',
                'success': True,
                'output': 'No changes to commit'
            })
            return jsonify({
                'success': True,
                'message': 'Sync completed - no local changes to push',
                'results': results
            })
        
        # Step 4: Commit changes with sanitized message
        raw_message = request.json.get('message', 'Auto-sync from minky application') if request.is_json else 'Auto-sync from minky application'
        commit_message = sanitize_commit_message(raw_message)
        if not commit_message:
            commit_message = 'Auto-sync from minky application'

        commit_result = run_git_command(['git', 'commit', '-m', commit_message], backup_dir)
        results.append({
            'operation': 'commit',
            'success': commit_result['success'],
            'output': commit_result['stdout'],
            'error': sanitize_git_error(commit_result['stderr']) if not commit_result['success'] else None
        })
        
        if not commit_result['success'] and 'nothing to commit' not in commit_result['stdout']:
            return jsonify({
                'success': False,
                'error': 'Failed to commit changes during sync',
                'results': results
            }), 400
        
        # Step 5: Push to remote
        push_result = run_git_command(['git', 'push'], backup_dir)
        results.append({
            'operation': 'push',
            'success': push_result['success'],
            'output': push_result['stdout'],
            'error': sanitize_git_error(push_result['stderr']) if not push_result['success'] else None
        })
        
        if push_result['success']:
            return jsonify({
                'success': True,
                'message': 'Successfully synced with repository',
                'results': results
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Git push failed during sync',
                'results': results
            }), 400
            
    except Exception as e:
        logger.error("Git sync error: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500


@git_bp.route('/git/config', methods=['GET'])
@limiter.limit("30 per minute")  # SECURITY: Rate limiting
@jwt_required()
def get_git_config():
    """Get git configuration"""
    try:
        backup_dir = get_safe_backup_dir()
        
        # Get git user name
        name_result = run_git_command(['git', 'config', 'user.name'], backup_dir)
        # Get git user email
        email_result = run_git_command(['git', 'config', 'user.email'], backup_dir)
        # Get remote origin URL
        remote_result = run_git_command(['git', 'config', '--get', 'remote.origin.url'], backup_dir)
        
        # Sanitize repository URL to remove any embedded credentials
        repo_url = sanitize_git_url(remote_result['stdout']) if remote_result['success'] else ''

        return jsonify({
            'success': True,
            'config': {
                'username': name_result['stdout'] if name_result['success'] else '',
                'email': email_result['stdout'] if email_result['success'] else '',
                'repository': repo_url
            }
        })
        
    except Exception as e:
        logger.error("Git config error: %s", e)
        return jsonify({
            'success': False,
            'error': 'Internal server error'
        }), 500