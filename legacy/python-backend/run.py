import os
from app import create_app, socketio

app = create_app()

if __name__ == '__main__':
    # SECURITY: Use environment-based configuration
    # Never use debug=True or host='0.0.0.0' in production
    flask_env = os.getenv('FLASK_ENV', 'production')
    debug = flask_env == 'development'

    # Only bind to all interfaces in development, use 127.0.0.1 in production
    host = '0.0.0.0' if flask_env == 'development' else '127.0.0.1'
    port = int(os.getenv('PORT', '5001'))

    socketio.run(app, debug=debug, host=host, port=port)
