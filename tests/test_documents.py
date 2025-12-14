"""
Tests for Document CRUD operations
"""
import pytest
import json


def test_create_document(client, sample_document_data):
    """Test creating a new document"""
    response = client.post('/api/documents',
                          data=json.dumps(sample_document_data),
                          content_type='application/json')

    assert response.status_code == 201
    data = json.loads(response.data)
    assert data['title'] == sample_document_data['title']
    assert data['author'] == sample_document_data['author']
    assert '<h1>Hello World</h1>' in data['html_content']


def test_list_documents(client, sample_document_data):
    """Test listing documents"""
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
    """Test getting a specific document"""
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


def test_get_nonexistent_document(client):
    """Test getting a document that doesn't exist"""
    response = client.get('/api/documents/99999')
    assert response.status_code == 404


def test_update_document(client, sample_document_data):
    """Test updating a document"""
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
    """Test deleting a document"""
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
    """Test searching documents"""
    docs = [
        {'title': 'Python Tutorial', 'markdown_content': '# Python\nLearn Python programming', 'is_public': True},
        {'title': 'JavaScript Guide', 'markdown_content': '# JavaScript\nLearn JS', 'is_public': True},
        {'title': 'Flask Framework', 'markdown_content': '# Flask\nWeb framework for Python', 'is_public': True}
    ]

    for doc in docs:
        client.post('/api/documents',
                   data=json.dumps(doc),
                   content_type='application/json')

    response = client.get('/api/documents?search=Python')
    assert response.status_code == 200

    data = json.loads(response.data)
    assert len(data['documents']) == 2
