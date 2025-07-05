import pytest
import json
from app import create_app, db
from app.models.document import Document

@pytest.fixture
def app():
    app = create_app()
    app.config['TESTING'] = True
    app.config['SQLALCHEMY_DATABASE_URI'] = 'sqlite:///:memory:'
    
    with app.app_context():
        db.create_all()
        yield app
        db.drop_all()

@pytest.fixture
def client(app):
    return app.test_client()

@pytest.fixture
def sample_document_data():
    return {
        'title': 'Test Document',
        'markdown_content': '# Hello World\n\nThis is a test document.',
        'author': 'Test Author'
    }

def test_create_document(client, sample_document_data):
    response = client.post('/api/documents', 
                          data=json.dumps(sample_document_data),
                          content_type='application/json')
    
    assert response.status_code == 201
    data = json.loads(response.data)
    assert data['title'] == sample_document_data['title']
    assert data['author'] == sample_document_data['author']
    assert data['html_content'] == '<h1>Hello World</h1>\n<p>This is a test document.</p>'

def test_list_documents(client, sample_document_data):
    client.post('/api/documents', 
                data=json.dumps(sample_document_data),
                content_type='application/json')
    
    response = client.get('/api/documents')
    assert response.status_code == 200
    
    data = json.loads(response.data)
    assert 'documents' in data
    assert 'pagination' in data
    assert len(data['documents']) == 1

def test_get_document(client, sample_document_data):
    create_response = client.post('/api/documents', 
                                 data=json.dumps(sample_document_data),
                                 content_type='application/json')
    
    created_doc = json.loads(create_response.data)
    doc_id = created_doc['id']
    
    response = client.get(f'/api/documents/{doc_id}')
    assert response.status_code == 200
    
    data = json.loads(response.data)
    assert data['id'] == doc_id
    assert data['title'] == sample_document_data['title']

def test_update_document(client, sample_document_data):
    create_response = client.post('/api/documents', 
                                 data=json.dumps(sample_document_data),
                                 content_type='application/json')
    
    created_doc = json.loads(create_response.data)
    doc_id = created_doc['id']
    
    update_data = {'title': 'Updated Title'}
    response = client.put(f'/api/documents/{doc_id}',
                         data=json.dumps(update_data),
                         content_type='application/json')
    
    assert response.status_code == 200
    data = json.loads(response.data)
    assert data['title'] == 'Updated Title'

def test_delete_document(client, sample_document_data):
    create_response = client.post('/api/documents', 
                                 data=json.dumps(sample_document_data),
                                 content_type='application/json')
    
    created_doc = json.loads(create_response.data)
    doc_id = created_doc['id']
    
    response = client.delete(f'/api/documents/{doc_id}')
    assert response.status_code == 200
    
    get_response = client.get(f'/api/documents/{doc_id}')
    assert get_response.status_code == 404

def test_search_documents(client):
    docs = [
        {'title': 'Python Tutorial', 'markdown_content': '# Python\nLearn Python programming'},
        {'title': 'JavaScript Guide', 'markdown_content': '# JavaScript\nLearn JS'},
        {'title': 'Flask Framework', 'markdown_content': '# Flask\nWeb framework for Python'}
    ]
    
    for doc in docs:
        client.post('/api/documents', 
                   data=json.dumps(doc),
                   content_type='application/json')
    
    response = client.get('/api/documents?search=Python')
    assert response.status_code == 200
    
    data = json.loads(response.data)
    assert len(data['documents']) == 2