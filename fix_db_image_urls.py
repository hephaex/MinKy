#!/usr/bin/env python3
"""
Script to fix image URLs in database documents
Changes https://localhost/img/ to /img/
"""

import os
import re
import psycopg2
from urllib.parse import urlparse

def fix_db_image_urls():
    """Fix image URLs in database documents"""
    
    # Database connection
    database_url = os.getenv('DATABASE_URL', 'postgresql://localhost/minky_db')
    
    try:
        # Parse the database URL
        parsed = urlparse(database_url)
        
        # Connect to database
        conn = psycopg2.connect(
            host=parsed.hostname,
            database=parsed.path[1:],  # Remove leading /
            user=parsed.username,
            password=parsed.password,
            port=parsed.port
        )
        
        cursor = conn.cursor()
        
        # Get all documents with image URLs
        cursor.execute("""
            SELECT id, title, markdown_content 
            FROM documents 
            WHERE markdown_content LIKE '%https://localhost/img/%'
        """)
        
        documents = cursor.fetchall()
        print(f"Found {len(documents)} documents with old image URLs")
        
        updated_count = 0
        
        for doc_id, title, content in documents:
            if content:
                # Replace the URLs
                old_pattern = r'https://localhost/img/'
                new_url = '/img/'
                
                updated_content = re.sub(old_pattern, new_url, content)
                
                # Update the document
                cursor.execute("""
                    UPDATE documents 
                    SET markdown_content = %s 
                    WHERE id = %s
                """, (updated_content, doc_id))
                
                updated_count += 1
                print(f"Updated document {doc_id}: {title}")
        
        # Commit changes
        conn.commit()
        print(f"✅ Successfully updated {updated_count} documents")
        
    except Exception as e:
        print(f"Error: {e}")
        print("Make sure the database is running and accessible")
    
    finally:
        if 'conn' in locals():
            conn.close()

if __name__ == "__main__":
    fix_db_image_urls()