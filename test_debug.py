#!/usr/bin/env python3

# Simple test to debug Document creation issue
import sys
sys.path.append('.')

from app.models.document import Document
from app import create_app

app = create_app()

with app.app_context():
    try:
        # Test Document creation exactly as in import service
        document = Document(
            title="Test Document",
            markdown_content="# Test Content\n\nThis is a test.",
            document_metadata={
                'source': 'import',
                'original_filename': 'test.txt',
                'file_type': 'text/plain',
                'import_method': 'markitdown'
            }
        )
        print("Document created successfully!")
        print(f"Title: {document.title}")
        print(f"Content: {document.markdown_content}")
        
    except Exception as e:
        print(f"Error creating document: {e}")
        import traceback
        traceback.print_exc()