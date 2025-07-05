from app import db
from sqlalchemy import text

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
        print("Performance indexes created successfully")
        
    except Exception as e:
        db.session.rollback()
        print(f"Error creating indexes: {e}")
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
        print("Database Statistics:")
        for stat in stats:
            print(f"  {stat.tablename}.{stat.attname}: {stat.n_distinct} distinct values, correlation: {stat.correlation}")
            
    except Exception as e:
        print(f"Error analyzing performance: {e}")

if __name__ == "__main__":
    from app import create_app
    app = create_app()
    with app.app_context():
        create_performance_indexes()
        analyze_query_performance()