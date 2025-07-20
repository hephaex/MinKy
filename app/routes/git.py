"""
Git Integration Routes
Provides API endpoints for Git operations like pull, push, and sync
"""

import os
import subprocess
import logging
from flask import Blueprint, jsonify, request, current_app
from flask_jwt_extended import jwt_required, get_jwt_identity

logger = logging.getLogger(__name__)

git_bp = Blueprint('git', __name__)


def run_git_command(command, cwd=None):
    """Execute a git command and return the result"""
    try:
        if cwd is None:
            cwd = current_app.config.get('BACKUP_DIR', './backup')
        
        # Ensure the directory exists
        os.makedirs(cwd, exist_ok=True)
        
        result = subprocess.run(
            command,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=30,
            shell=False
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
        return {
            'success': False,
            'stdout': '',
            'stderr': str(e),
            'returncode': -1
        }


@git_bp.route('/git/status', methods=['GET'])
def git_status():
    """Get git repository status"""
    try:
        backup_dir = current_app.config.get('BACKUP_DIR', './backup')
        
        # Check if it's a git repository
        result = run_git_command(['git', 'status', '--porcelain'], backup_dir)
        
        if not result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository or git not available',
                'details': result['stderr']
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
        logger.error(f"Git status error: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@git_bp.route('/git/pull', methods=['POST'])
@jwt_required(optional=True)
def git_pull():
    """Pull changes from remote repository"""
    try:
        backup_dir = current_app.config.get('BACKUP_DIR', './backup')
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': status_result['stderr']
            }), 400
        
        # Perform git pull
        pull_result = run_git_command(['git', 'pull'], backup_dir)
        
        if pull_result['success']:
            return jsonify({
                'success': True,
                'message': 'Successfully pulled changes from repository',
                'output': pull_result['stdout'],
                'details': pull_result['stderr'] if pull_result['stderr'] else None
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Git pull failed',
                'details': pull_result['stderr'],
                'output': pull_result['stdout']
            }), 400
            
    except Exception as e:
        logger.error(f"Git pull error: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@git_bp.route('/git/push', methods=['POST'])
@jwt_required(optional=True)
def git_push():
    """Push changes to remote repository"""
    try:
        backup_dir = current_app.config.get('BACKUP_DIR', './backup')
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': status_result['stderr']
            }), 400
        
        # Add all changes
        add_result = run_git_command(['git', 'add', '.'], backup_dir)
        if not add_result['success']:
            return jsonify({
                'success': False,
                'error': 'Failed to stage changes',
                'details': add_result['stderr']
            }), 400
        
        # Check if there are changes to commit
        status_check = run_git_command(['git', 'status', '--porcelain'], backup_dir)
        if not status_check['stdout'].strip():
            return jsonify({
                'success': True,
                'message': 'No changes to push',
                'output': 'Working tree clean'
            })
        
        # Commit changes
        commit_message = request.json.get('message', 'Auto-commit from minky application') if request.is_json else 'Auto-commit from minky application'
        commit_result = run_git_command(['git', 'commit', '-m', commit_message], backup_dir)
        
        if not commit_result['success'] and 'nothing to commit' not in commit_result['stdout']:
            return jsonify({
                'success': False,
                'error': 'Failed to commit changes',
                'details': commit_result['stderr'],
                'output': commit_result['stdout']
            }), 400
        
        # Push to remote
        push_result = run_git_command(['git', 'push'], backup_dir)
        
        if push_result['success']:
            return jsonify({
                'success': True,
                'message': 'Successfully pushed changes to repository',
                'output': push_result['stdout'],
                'details': push_result['stderr'] if push_result['stderr'] else None
            })
        else:
            return jsonify({
                'success': False,
                'error': 'Git push failed',
                'details': push_result['stderr'],
                'output': push_result['stdout']
            }), 400
            
    except Exception as e:
        logger.error(f"Git push error: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@git_bp.route('/git/sync', methods=['POST'])
@jwt_required(optional=True)
def git_sync():
    """Sync with remote repository (pull then push)"""
    try:
        backup_dir = current_app.config.get('BACKUP_DIR', './backup')
        
        # First, check if we're in a git repository
        status_result = run_git_command(['git', 'status'], backup_dir)
        if not status_result['success']:
            return jsonify({
                'success': False,
                'error': 'Not a git repository',
                'details': status_result['stderr']
            }), 400
        
        results = []
        
        # Step 1: Pull changes
        pull_result = run_git_command(['git', 'pull'], backup_dir)
        results.append({
            'operation': 'pull',
            'success': pull_result['success'],
            'output': pull_result['stdout'],
            'error': pull_result['stderr'] if not pull_result['success'] else None
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
                'error': add_result['stderr']
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
        
        # Step 4: Commit changes
        commit_message = request.json.get('message', 'Auto-sync from minky application') if request.is_json else 'Auto-sync from minky application'
        commit_result = run_git_command(['git', 'commit', '-m', commit_message], backup_dir)
        results.append({
            'operation': 'commit',
            'success': commit_result['success'],
            'output': commit_result['stdout'],
            'error': commit_result['stderr'] if not commit_result['success'] else None
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
            'error': push_result['stderr'] if not push_result['success'] else None
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
        logger.error(f"Git sync error: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@git_bp.route('/git/config', methods=['GET'])
def get_git_config():
    """Get git configuration"""
    try:
        backup_dir = current_app.config.get('BACKUP_DIR', './backup')
        
        # Get git user name
        name_result = run_git_command(['git', 'config', 'user.name'], backup_dir)
        # Get git user email
        email_result = run_git_command(['git', 'config', 'user.email'], backup_dir)
        # Get remote origin URL
        remote_result = run_git_command(['git', 'config', '--get', 'remote.origin.url'], backup_dir)
        
        return jsonify({
            'success': True,
            'config': {
                'username': name_result['stdout'] if name_result['success'] else '',
                'email': email_result['stdout'] if email_result['success'] else '',
                'repository': remote_result['stdout'] if remote_result['success'] else ''
            }
        })
        
    except Exception as e:
        logger.error(f"Git config error: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500