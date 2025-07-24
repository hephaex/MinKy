#!/usr/bin/env python3
"""
Script to fix image URLs in existing documents
Changes https://localhost/img/ to http://localhost:5000/img/
"""

import os
import sys
import re

# Add the project directory to the path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from app import create_app, db
from app.models.document import Document

def fix_image_urls():
    """Fix image URLs in all existing documents"""
    app = create_app()
    
    with app.app_context():
        # Get all documents
        documents = Document.query.all()
        
        print(f"Found {len(documents)} documents to check")
        
        updated_count = 0
        
        for document in documents:
            if document.markdown_content:
                # Pattern to match https://localhost/img/ URLs
                old_pattern = r'https://localhost/img/'
                new_url = '/img/'
                
                # Check if the document contains the old URL pattern
                if re.search(old_pattern, document.markdown_content):
                    # Replace the URLs
                    updated_content = re.sub(old_pattern, new_url, document.markdown_content)
                    
                    # Update the document
                    document.markdown_content = updated_content
                    updated_count += 1
                    
                    print(f"Updated document {document.id}: {document.title}")
        
        if updated_count > 0:
            # Commit the changes
            db.session.commit()
            print(f"✅ Successfully updated {updated_count} documents")
        else:
            print("No documents needed updating")

if __name__ == "__main__":
    fix_image_urls()