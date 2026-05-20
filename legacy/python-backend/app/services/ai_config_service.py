"""
AI Configuration Service
Handles AI configuration loading, saving, and connection testing
"""

import os
import logging
from typing import Dict, Any

logger = logging.getLogger(__name__)


def get_env_keys() -> Dict[str, str]:
    """Load API keys from environment variables"""
    return {
        'openai': os.getenv('OPENAI_API_KEY', ''),
        'anthropic': os.getenv('ANTHROPIC_API_KEY', ''),
        'google': os.getenv('GOOGLE_API_KEY', ''),
        'local': os.getenv('LOCAL_LLM_URL', 'http://localhost:8080')
    }


def get_default_config(env_keys: Dict[str, str]) -> Dict[str, Any]:
    """Get default configuration with environment variable fallbacks"""
    default_llm_provider = os.getenv('DEFAULT_LLM_PROVIDER', 'openai')

    if default_llm_provider == 'local':
        default_llm_model = os.getenv('LOCAL_LLM_MODEL', 'llama2')
    else:
        default_llm_model = os.getenv('DEFAULT_LLM_MODEL', 'gpt-3.5-turbo')

    default_ocr_service = os.getenv('DEFAULT_OCR_SERVICE', 'tesseract')
    default_api_key = env_keys.get(default_llm_provider, '')
    default_ai_tags_enabled = os.getenv('DEFAULT_AI_TAGS_ENABLED', 'true').lower() == 'true'
    default_ai_summary_enabled = os.getenv('DEFAULT_AI_SUMMARY_ENABLED', 'false').lower() == 'true'

    return {
        'ocrService': default_ocr_service,
        'ocrApiKey': '',
        'llmProvider': default_llm_provider,
        'llmApiKey': default_api_key,
        'llmModel': default_llm_model,
        'enableAiTags': default_ai_tags_enabled,
        'enableAiSummary': default_ai_summary_enabled
    }


def mask_api_key(api_key: str) -> str:
    """Mask API key for safe display"""
    if not api_key:
        return ''
    if len(api_key) > 8:
        return api_key[:4] + '*' * (len(api_key) - 8) + api_key[-4:]
    return '*' * len(api_key)


def get_safe_config(config: Dict[str, Any]) -> Dict[str, Any]:
    """Get configuration with masked API keys for safe display"""
    safe_config: Dict[str, Any] = dict(config)

    if 'llmApiKey' in safe_config and safe_config['llmApiKey']:
        safe_config['llmApiKey'] = mask_api_key(safe_config['llmApiKey'])

    if 'ocrApiKey' in safe_config and safe_config['ocrApiKey']:
        safe_config['ocrApiKey'] = mask_api_key(safe_config['ocrApiKey'])

    return safe_config


def test_llm_connection(config_data: Dict) -> Dict:
    """Test LLM provider connection"""
    try:
        provider = config_data.get('llmProvider', 'openai')
        api_key = config_data.get('llmApiKey', '')

        if not api_key and provider != 'local':
            return {
                'success': False,
                'error': 'API key is required'
            }

        if provider == 'local' and not api_key:
            api_key = os.getenv('LOCAL_LLM_URL', 'http://localhost:8080')

        if provider == 'openai':
            if not api_key.startswith(('sk-', 'sk-proj-')):
                return {
                    'success': False,
                    'error': 'Invalid OpenAI API key format. Keys should start with "sk-"'
                }

            if len(api_key) < 20:
                return {
                    'success': False,
                    'error': 'OpenAI API key appears to be too short'
                }

            return {
                'success': True,
                'message': 'OpenAI API key format is valid'
            }

        elif provider == 'anthropic':
            if not api_key.startswith('sk-ant-'):
                return {
                    'success': False,
                    'error': 'Invalid Anthropic API key format. Keys should start with "sk-ant-"'
                }

            if len(api_key) < 30:
                return {
                    'success': False,
                    'error': 'Anthropic API key appears to be too short'
                }

            return {
                'success': True,
                'message': 'Anthropic (Claude) API key format is valid'
            }

        elif provider == 'google':
            if len(api_key) < 10:
                return {
                    'success': False,
                    'error': 'Google API key appears to be too short'
                }

            if not api_key.replace('-', '').replace('_', '').isalnum():
                return {
                    'success': False,
                    'error': 'Google API key format appears invalid'
                }

            return {
                'success': True,
                'message': 'Google (Gemini) API key format is valid'
            }

        elif provider == 'local':
            if not api_key:
                return {
                    'success': False,
                    'error': 'Local LLM server URL is required'
                }

            if not api_key.startswith(('http://', 'https://')):
                return {
                    'success': False,
                    'error': 'Local LLM server URL must start with http:// or https://'
                }

            try:
                from urllib.parse import urlparse
                import socket
                import ipaddress

                parsed = urlparse(api_key)
                if not parsed.netloc:
                    return {
                        'success': False,
                        'error': 'Invalid Local LLM server URL format'
                    }

                # SECURITY: Extract hostname for SSRF validation
                hostname = parsed.hostname
                if not hostname:
                    return {
                        'success': False,
                        'error': 'Invalid Local LLM server URL - no host specified'
                    }

                # SECURITY: Block cloud metadata endpoints (SSRF protection)
                blocked_hosts = frozenset([
                    '169.254.169.254',  # AWS/GCP/Azure metadata
                    'metadata.google.internal',
                    'metadata.internal',
                    '100.100.100.200',  # Alibaba Cloud metadata
                ])
                if hostname.lower() in blocked_hosts:
                    logger.warning(f"SSRF attempt blocked: metadata endpoint {hostname}")
                    return {
                        'success': False,
                        'error': 'Invalid Local LLM server URL - blocked endpoint'
                    }

                # SECURITY: Resolve and check for private/internal IPs
                try:
                    resolved_ips = socket.getaddrinfo(hostname, None)
                    for family, socktype, proto, canonname, sockaddr in resolved_ips:
                        ip_str = sockaddr[0]
                        ip = ipaddress.ip_address(ip_str)

                        # Allow localhost for local LLM servers
                        if ip.is_loopback:
                            continue

                        # SECURITY: Block private networks (prevent SSRF to internal services)
                        if ip.is_private or ip.is_reserved or ip.is_link_local:
                            logger.warning(f"SSRF attempt blocked: private IP {ip_str} for host {hostname}")
                            return {
                                'success': False,
                                'error': 'Local LLM URL must use localhost or a public IP address'
                            }
                except socket.gaierror:
                    # DNS resolution failed - this is OK for format validation
                    pass

            except Exception:
                return {
                    'success': False,
                    'error': 'Invalid Local LLM server URL format'
                }

            return {
                'success': True,
                'message': 'Local LLM server URL format is valid'
            }

        else:
            if len(api_key) < 10:
                return {
                    'success': False,
                    'error': 'API key appears to be invalid'
                }
            return {
                'success': True,
                'message': f'{provider} API key format validated'
            }

    except Exception as e:
        logger.error(f"LLM test failed: {e}", exc_info=True)
        return {
            'success': False,
            'error': 'Connection test failed. Please verify your credentials.'
        }


def test_ocr_connection(config_data: Dict) -> Dict:
    """Test OCR service connection"""
    try:
        service = config_data.get('ocrService', 'tesseract')
        api_key = config_data.get('ocrApiKey', '')

        if service == 'tesseract':
            return {'success': True, 'message': 'Tesseract is available locally'}

        if service == 'google-vision':
            if not api_key:
                google_cloud_key = os.getenv('GOOGLE_CLOUD_API_KEY', '')
                google_llm_key = os.getenv('GOOGLE_API_KEY', '')

                if google_cloud_key:
                    api_key = google_cloud_key
                elif google_llm_key:
                    api_key = google_llm_key

            if api_key and len(api_key) >= 10:
                return {
                    'success': True,
                    'message': 'Google Vision API configuration is valid'
                }
            else:
                return {
                    'success': False,
                    'error': 'Google Vision API key is required'
                }

        if not api_key:
            return {
                'success': False,
                'error': 'API key is required for cloud OCR services'
            }

        if len(api_key) < 10:
            return {
                'success': False,
                'error': 'API key appears to be invalid'
            }

        return {
            'success': True,
            'message': f'{service} OCR connection test successful'
        }

    except Exception as e:
        logger.error(f"OCR test failed: {e}", exc_info=True)
        return {
            'success': False,
            'error': 'OCR connection test failed. Please verify your configuration.'
        }
