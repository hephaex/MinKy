"""
Tests for Document CRUD operations
"""
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


def test_create_document_with_category(client, app):
    """Test creating a document with a category"""
    from app import db
    from app.models.category import Category

    with app.app_context():
        category = Category(name='Test Category', color='#007bff')
        db.session.add(category)
        db.session.commit()
        cat_id = category.id

    doc_data = {
        'title': 'Categorized Document',
        'markdown_content': '# Content',
        'is_public': True,
        'category_id': cat_id
    }

    response = client.post('/api/documents',
                          data=json.dumps(doc_data),
                          content_type='application/json')

    assert response.status_code == 201
    data = json.loads(response.data)
    assert data['category_id'] == cat_id
    assert data['category']['name'] == 'Test Category'


def test_create_document_with_invalid_category(client):
    """Test creating a document with non-existent category returns 404"""
    doc_data = {
        'title': 'Document',
        'markdown_content': '# Content',
        'category_id': 99999
    }

    response = client.post('/api/documents',
                          data=json.dumps(doc_data),
                          content_type='application/json')

    assert response.status_code == 404
    data = json.loads(response.data)
    assert 'error' in data


def test_update_document_category(client, app, sample_document_data):
    """Test updating a document's category"""
    from app import db
    from app.models.category import Category

    # Create document first
    create_response = client.post('/api/documents',
                                 data=json.dumps(sample_document_data),
                                 content_type='application/json')
    created_doc = json.loads(create_response.data)
    doc_id = created_doc['id']

    # Create category
    with app.app_context():
        category = Category(name='New Category', color='#ff0000')
        db.session.add(category)
        db.session.commit()
        cat_id = category.id

    # Update document with category
    update_data = {'category_id': cat_id}
    response = client.put(f'/api/documents/{doc_id}',
                         data=json.dumps(update_data),
                         content_type='application/json')

    assert response.status_code == 200
    data = json.loads(response.data)
    assert data['category_id'] == cat_id
    assert data['category']['name'] == 'New Category'


def test_remove_document_category(client, app, sample_document_data):
    """Test removing a document's category by setting to null"""
    from app import db
    from app.models.category import Category

    # Create category
    with app.app_context():
        category = Category(name='Category', color='#007bff')
        db.session.add(category)
        db.session.commit()
        cat_id = category.id

    # Create document with category
    doc_data = {**sample_document_data, 'category_id': cat_id}
    create_response = client.post('/api/documents',
                                 data=json.dumps(doc_data),
                                 content_type='application/json')
    created_doc = json.loads(create_response.data)
    doc_id = created_doc['id']
    assert created_doc['category_id'] == cat_id

    # Remove category
    update_data = {'category_id': None}
    response = client.put(f'/api/documents/{doc_id}',
                         data=json.dumps(update_data),
                         content_type='application/json')

    assert response.status_code == 200
    data = json.loads(response.data)
    assert data['category_id'] is None
    assert data['category'] is None
