import logging

from app import db
from sqlalchemy import text

logger = logging.getLogger(__name__)


def create_performance_indexes():
    """
    Create database indexes for better performance.
    Should be run after initial migration.
    """
    try:
        # Full-text search index
        db.session.execute(text("""
            CREATE INDEX IF NOT EXISTS idx_documents_search
            ON documents USING gin(to_tsvector('english', title || ' ' || markdown_content))
        """))

        # User-related indexes
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_documents_user_id ON documents(user_id)"))
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_documents_public ON documents(is_public)"))
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_documents_user_public ON documents(user_id, is_public)"))

        # Timestamp indexes
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_documents_updated_at ON documents(updated_at DESC)"))
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_documents_created_at ON documents(created_at DESC)"))

        # User table indexes
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)"))
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)"))
        db.session.execute(text("CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active)"))

        db.session.commit()
        logger.info("Performance indexes created successfully")

    except Exception as e:
        db.session.rollback()
        logger.error("Error creating indexes: %s", e)
        raise

def analyze_query_performance():
    """
    Analyze query performance and suggest optimizations
    """
    try:
        # Get table statistics
        result = db.session.execute(text("""
            SELECT
                schemaname,
                tablename,
                attname,
                n_distinct,
                correlation
            FROM pg_stats
            WHERE schemaname = 'public'
            AND tablename IN ('documents', 'users')
            ORDER BY tablename, attname
        """))

        stats = result.fetchall()
        logger.info("Database Statistics:")
        for stat in stats:
            logger.info("  %s.%s: %s distinct values, correlation: %s",
                       stat.tablename, stat.attname, stat.n_distinct, stat.correlation)

    except Exception as e:
        logger.error("Error analyzing performance: %s", e)

if __name__ == "__main__":
    from app import create_app
    app = create_app()
    with app.app_context():
        create_performance_indexes()
        analyze_query_performance()
