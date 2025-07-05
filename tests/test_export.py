import pytest
import json
import tempfile
import zipfile
from app import create_app, db
from app.models.document import Document
from app.models.user import User

@pytest.fixture
def app():
    app = create_app()
    app.config['TESTING'] = True
    app.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///:memory:'
    app.config['WTF_CSRF_ENABLED'] = False
    
    with app.app_context():
        db.create_all()
        yield app
        db.drop_all()

@pytest.fixture
def client(app):
    return app.test_client()

@pytest.fixture
def auth_headers(client):
    # Create a test user
    user_data = {
        'username': 'testuser',
        'email': 'test@example.com',
        'password': 'testpassword123'
    }
    client.post('/api/auth/register', 
                data=json.dumps(user_data),
                content_type='application/json')
    
    # Login and get token
    login_data = {
        'username': 'testuser',
        'password': 'testpassword123'
    }
    response = client.post('/api/auth/login',
                          data=json.dumps(login_data),
                          content_type='application/json')
    
    token = json.loads(response.data)['access_token']
    return {'Authorization': f'Bearer {token}'}

@pytest.fixture
def sample_document(client, auth_headers):
    document_data = {
        'title': 'Test Document',
        'markdown_content': '# Test Title\n\nThis is a **test** document with *markdown* content.\n\n## Subsection\n\n- Item 1\n- Item 2\n\n```python\nprint("Hello World")\n```',
        'author': 'Test Author',
        'is_private': False
    }
    
    response = client.post('/api/documents',
                          data=json.dumps(document_data),
                          content_type='application/json',
                          headers=auth_headers)
    
    return json.loads(response.data)

def test_get_export_formats(client):
    """Test getting supported export formats"""
    response = client.get('/api/export/formats')
    
    assert response.status_code == 200
    data = json.loads(response.data)
    
    assert 'formats' in data
    assert 'html' in data['formats']
    assert 'pdf' in data['formats']
    assert 'docx' in data['formats']
    assert 'markdown' in data['formats']
    assert 'json' in data['formats']
    
    assert 'default_bundle_formats' in data

def test_export_document_html(client, auth_headers, sample_document):
    """Test exporting document as HTML"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/html',
                         headers=auth_headers)
    
    assert response.status_code == 200
    assert response.mimetype == 'text/html'
    assert b'Test Document' in response.data
    assert b'<h1>' in response.data
    assert b'<strong>test</strong>' in response.data

def test_export_document_json(client, auth_headers, sample_document):
    """Test exporting document as JSON"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/json',
                         headers=auth_headers)
    
    assert response.status_code == 200
    assert response.mimetype == 'application/json'
    
    # The response should contain the JSON file content
    data = json.loads(response.data.decode('utf-8'))
    assert 'document' in data
    assert 'export_info' in data
    assert data['document']['title'] == 'Test Document'

def test_export_document_markdown(client, auth_headers, sample_document):
    """Test exporting document as Markdown"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/markdown',
                         headers=auth_headers)
    
    assert response.status_code == 200
    assert response.mimetype == 'text/markdown'

def test_export_invalid_format(client, auth_headers, sample_document):
    """Test exporting document with invalid format"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/invalid',
                         headers=auth_headers)
    
    assert response.status_code == 400
    data = json.loads(response.data)
    assert 'error' in data
    assert 'Invalid format' in data['error']

def test_export_nonexistent_document(client, auth_headers):
    """Test exporting non-existent document"""
    response = client.get('/api/documents/999/export/html',
                         headers=auth_headers)
    
    assert response.status_code == 404

def test_export_unauthorized(client, sample_document):
    """Test exporting document without authentication"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/html')
    
    assert response.status_code == 422  # JWT missing

def test_export_bundle(client, auth_headers, sample_document):
    """Test exporting document bundle"""
    doc_id = sample_document['id']
    
    response = client.get(f'/api/documents/{doc_id}/export/bundle?formats=html,json,markdown',
                         headers=auth_headers)
    
    assert response.status_code == 200
    assert response.mimetype == 'application/zip'
    
    # Save the ZIP file and verify contents
    with tempfile.NamedTemporaryFile(suffix='.zip', delete=False) as tmp:
        tmp.write(response.data)
        tmp.flush()
        
        with zipfile.ZipFile(tmp.name, 'r') as zf:
            files = zf.namelist()
            assert any(f.endswith('.html') for f in files)
            assert any(f.endswith('.json') for f in files)
            assert any(f.endswith('.md') for f in files)

def test_bulk_export(client, auth_headers, sample_document):
    """Test bulk export of documents"""
    # Create another document
    document_data = {
        'title': 'Second Document',
        'markdown_content': '# Second Document\n\nAnother test document.',
        'author': 'Test Author',
        'is_private': False
    }
    
    response = client.post('/api/documents',
                          data=json.dumps(document_data),
                          content_type='application/json',
                          headers=auth_headers)
    
    doc2 = json.loads(response.data)
    
    # Bulk export both documents
    bulk_data = {
        'document_ids': [sample_document['id'], doc2['id']],
        'formats': ['html', 'json']
    }
    
    response = client.post('/api/documents/bulk-export',
                          data=json.dumps(bulk_data),
                          content_type='application/json',
                          headers=auth_headers)
    
    assert response.status_code == 200
    assert response.mimetype == 'application/zip'
    
    # Verify ZIP contains files for both documents
    with tempfile.NamedTemporaryFile(suffix='.zip', delete=False) as tmp:
        tmp.write(response.data)
        tmp.flush()
        
        with zipfile.ZipFile(tmp.name, 'r') as zf:
            files = zf.namelist()
            html_files = [f for f in files if f.endswith('.html')]
            json_files = [f for f in files if f.endswith('.json')]
            
            assert len(html_files) == 2
            assert len(json_files) == 2

def test_bulk_export_invalid_request(client, auth_headers):
    """Test bulk export with invalid request"""
    # Missing document_ids
    response = client.post('/api/documents/bulk-export',
                          data=json.dumps({}),
                          content_type='application/json',
                          headers=auth_headers)
    
    assert response.status_code == 400
    data = json.loads(response.data)
    assert 'document_ids required' in data['error']
    
    # Invalid format
    bulk_data = {
        'document_ids': [1],
        'formats': ['invalid_format']
    }
    
    response = client.post('/api/documents/bulk-export',
                          data=json.dumps(bulk_data),
                          content_type='application/json',
                          headers=auth_headers)
    
    assert response.status_code == 400
    data = json.loads(response.data)
    assert 'Invalid format' in data['error']