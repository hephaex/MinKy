"""
Structured JSON logging configuration for MinKy API.
Supports log aggregation with ELK/Loki.
"""

import logging
import sys
import os
from datetime import datetime, timezone
from pythonjsonlogger import jsonlogger
from flask import g, request


class CustomJsonFormatter(jsonlogger.JsonFormatter):
    """Custom JSON formatter with request context."""

    def add_fields(self, log_record, record, message_dict):
        super().add_fields(log_record, record, message_dict)

        # Add timestamp in ISO format
        log_record['timestamp'] = datetime.now(timezone.utc).isoformat()
        log_record['level'] = record.levelname
        log_record['logger'] = record.name

        # Add request context if available
        try:
            if request:
                log_record['request_id'] = getattr(g, 'request_id', None)
                log_record['method'] = request.method
                log_record['path'] = request.path
                log_record['remote_addr'] = request.remote_addr
                log_record['user_id'] = getattr(g, 'current_user_id', None)
        except RuntimeError:
            # Outside request context
            pass

        # Add exception info if present
        if record.exc_info:
            log_record['exception'] = self.formatException(record.exc_info)


def setup_logging(app):
    """Configure structured JSON logging for the Flask app."""
    log_level = os.getenv('LOG_LEVEL', 'INFO').upper()
    log_format = os.getenv('LOG_FORMAT', 'json')  # 'json' or 'text'

    # Remove default handlers
    root_logger = logging.getLogger()
    for handler in root_logger.handlers[:]:
        root_logger.removeHandler(handler)

    # Create handler
    handler = logging.StreamHandler(sys.stdout)

    if log_format == 'json':
        formatter = CustomJsonFormatter(
            '%(timestamp)s %(level)s %(name)s %(message)s'
        )
    else:
        formatter = logging.Formatter(
            '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        )

    handler.setFormatter(formatter)
    handler.setLevel(getattr(logging, log_level))

    # Configure root logger
    root_logger.addHandler(handler)
    root_logger.setLevel(getattr(logging, log_level))

    # Reduce noise from third-party libraries
    logging.getLogger('werkzeug').setLevel(logging.WARNING)
    logging.getLogger('urllib3').setLevel(logging.WARNING)
    logging.getLogger('sqlalchemy.engine').setLevel(logging.WARNING)

    return root_logger


def add_request_id_middleware(app):
    """Add request ID to each request for tracing."""
    import uuid

    @app.before_request
    def before_request():
        g.request_id = request.headers.get('X-Request-ID', str(uuid.uuid4()))
        g.request_start_time = datetime.now(timezone.utc)

    @app.after_request
    def after_request(response):
        # Add request ID to response headers
        response.headers['X-Request-ID'] = getattr(g, 'request_id', '')

        # Log request completion
        if hasattr(g, 'request_start_time'):
            duration = (datetime.now(timezone.utc) - g.request_start_time).total_seconds()
            logger = logging.getLogger('minky.access')
            logger.info(
                'Request completed',
                extra={
                    'duration_seconds': duration,
                    'status_code': response.status_code,
                    'content_length': response.content_length
                }
            )

        return response

    return app
