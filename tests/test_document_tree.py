"""
Test suite for document tree API endpoint.

Tests the GET /api/documents/tree endpoint with different modes:
- by-tag: Groups documents by tags
- by-date: Groups documents by year/month
"""
import pytest
from datetime import datetime, timezone
from app import db
from app.models.document import Document
from app.models.tag import Tag
from app.models.user import User


def test_tree_by_tag_empty(app, client, auth_headers):
    """Test tree endpoint with no documents returns empty tree."""
    with app.app_context():
        response = client.get(
            '/api/documents/tree?mode=by-tag',
            headers=auth_headers
        )

        assert response.status_code == 200
        data = response.get_json()

        assert 'tree' in data
        assert 'mode' in data
        assert 'total_documents' in data

        assert data['mode'] == 'by-tag'
        assert data['total_documents'] == 0
        assert data['tree'] == []


def test_tree_by_tag_with_documents(app, client, auth_headers, sample_user):
    """Test tree endpoint groups documents by tags correctly."""
    with app.app_context():
        # Get the user object
        user = User.query.get(sample_user)

        # Create document with tags
        doc1 = Document(
            title='Python Tutorial',
            markdown_content='# Python Tutorial\n\nLearn Python basics.',
            user_id=user.id
        )
        db.session.add(doc1)
        db.session.commit()

        # Add tags to document
        python_tag = Tag.get_or_create('python', created_by=user.id)
        tutorial_tag = Tag.get_or_create('tutorial', created_by=user.id)
        doc1.tags.append(python_tag)
        doc1.tags.append(tutorial_tag)
        db.session.commit()

        # Create another document with one shared tag
        doc2 = Document(
            title='JavaScript Tutorial',
            markdown_content='# JavaScript Tutorial\n\nLearn JavaScript basics.',
            user_id=user.id
        )
        db.session.add(doc2)
        db.session.commit()

        js_tag = Tag.get_or_create('javascript', created_by=user.id)
        doc2.tags.append(tutorial_tag)
        doc2.tags.append(js_tag)
        db.session.commit()

        # Make request
        response = client.get(
            '/api/documents/tree?mode=by-tag',
            headers=auth_headers
        )

        assert response.status_code == 200
        data = response.get_json()

        assert data['mode'] == 'by-tag'
        # total_documents sums per-tag counts (docs may appear under multiple tags)
        assert data['total_documents'] >= 2
        assert len(data['tree']) == 3  # python, tutorial, javascript

        # Verify tag node structure
        tag_nodes = {node['label']: node for node in data['tree']}

        # Check python tag
        assert 'python' in tag_nodes
        python_node = tag_nodes['python']
        assert python_node['type'] == 'tag'
        assert python_node['count'] == 1
        assert len(python_node['children']) == 1
        assert python_node['children'][0]['type'] == 'document'
        assert python_node['children'][0]['documentId'] == doc1.id
        assert python_node['children'][0]['label'] == 'Python Tutorial'

        # Check tutorial tag (should have 2 documents)
        assert 'tutorial' in tag_nodes
        tutorial_node = tag_nodes['tutorial']
        assert tutorial_node['type'] == 'tag'
        assert tutorial_node['count'] == 2
        assert len(tutorial_node['children']) == 2

        # Check javascript tag
        assert 'javascript' in tag_nodes
        js_node = tag_nodes['javascript']
        assert js_node['type'] == 'tag'
        assert js_node['count'] == 1
        assert len(js_node['children']) == 1


def test_tree_by_date_with_documents(app, client, auth_headers, sample_user):
    """Test tree endpoint groups documents by year/month correctly."""
    with app.app_context():
        # Get the user object
        user = User.query.get(sample_user)

        # Create documents with specific dates
        doc1 = Document(
            title='January Document',
            markdown_content='# January\n\nCreated in January.',
            user_id=user.id
        )
        doc1.created_at = datetime(2024, 1, 15, tzinfo=timezone.utc)
        db.session.add(doc1)

        doc2 = Document(
            title='February Document',
            markdown_content='# February\n\nCreated in February.',
            user_id=user.id
        )
        doc2.created_at = datetime(2024, 2, 20, tzinfo=timezone.utc)
        db.session.add(doc2)

        doc3 = Document(
            title='Another January Document',
            markdown_content='# Another January\n\nAlso in January.',
            user_id=user.id
        )
        doc3.created_at = datetime(2024, 1, 25, tzinfo=timezone.utc)
        db.session.add(doc3)

        # Document in different year
        doc4 = Document(
            title='2023 Document',
            markdown_content='# 2023\n\nFrom last year.',
            user_id=user.id
        )
        doc4.created_at = datetime(2023, 12, 10, tzinfo=timezone.utc)
        db.session.add(doc4)

        db.session.commit()

        # Make request
        response = client.get(
            '/api/documents/tree?mode=by-date',
            headers=auth_headers
        )

        assert response.status_code == 200
        data = response.get_json()

        assert data['mode'] == 'by-date'
        assert data['total_documents'] == 4
        assert len(data['tree']) == 2  # 2023, 2024

        # Find year nodes (labels are like '2024년')
        year_nodes = {node['id']: node for node in data['tree']}

        # Check 2024 year node
        assert 'date-2024' in year_nodes
        year_2024 = year_nodes['date-2024']
        assert year_2024['type'] == 'year'
        assert year_2024['count'] == 3
        assert len(year_2024['children']) == 2  # January, February

        # Check months in 2024 by id
        month_nodes = {node['id']: node for node in year_2024['children']}

        # January should have 2 documents
        assert 'date-2024-01' in month_nodes
        january_node = month_nodes['date-2024-01']
        assert january_node['type'] == 'month'
        assert january_node['count'] == 2
        assert len(january_node['children']) == 2

        # February should have 1 document
        assert 'date-2024-02' in month_nodes
        february_node = month_nodes['date-2024-02']
        assert february_node['type'] == 'month'
        assert february_node['count'] == 1
        assert len(february_node['children']) == 1

        # Check 2023 year node
        assert 'date-2023' in year_nodes
        year_2023 = year_nodes['date-2023']
        assert year_2023['type'] == 'year'
        assert year_2023['count'] == 1
        assert len(year_2023['children']) == 1  # December

        # Verify document nodes have correct structure
        doc_node = january_node['children'][0]
        assert doc_node['type'] == 'document'
        assert 'documentId' in doc_node
        assert 'label' in doc_node
        assert doc_node['documentId'] in [doc1.id, doc3.id]


def test_tree_invalid_mode(app, client, auth_headers):
    """Test tree endpoint with invalid mode returns 400 error."""
    with app.app_context():
        response = client.get(
            '/api/documents/tree?mode=invalid',
            headers=auth_headers
        )

        assert response.status_code == 400
        data = response.get_json()

        assert 'error' in data
        assert 'mode' in data['error'].lower() or 'invalid' in data['error'].lower()


def test_tree_by_tag_untagged_documents(app, client, auth_headers, sample_user):
    """Test tree endpoint groups untagged documents under '태그 없음' node."""
    with app.app_context():
        # Get the user object
        user = User.query.get(sample_user)

        # Create document without tags
        doc1 = Document(
            title='Untagged Document',
            markdown_content='# Untagged\n\nThis document has no tags.',
            user_id=user.id
        )
        db.session.add(doc1)

        # Create another untagged document
        doc2 = Document(
            title='Another Untagged',
            markdown_content='# Also Untagged\n\nNo tags here either.',
            user_id=user.id
        )
        db.session.add(doc2)

        # Create one tagged document for comparison
        doc3 = Document(
            title='Tagged Document',
            markdown_content='# Tagged\n\nThis one has tags.',
            user_id=user.id
        )
        db.session.add(doc3)
        db.session.commit()

        # Add tag to doc3
        python_tag = Tag.get_or_create('python', created_by=user.id)
        doc3.tags.append(python_tag)
        db.session.commit()

        # Make request
        response = client.get(
            '/api/documents/tree?mode=by-tag',
            headers=auth_headers
        )

        assert response.status_code == 200
        data = response.get_json()

        assert data['mode'] == 'by-tag'
        assert data['total_documents'] == 3
        assert len(data['tree']) == 2  # python tag + 태그 없음

        # Find untagged node
        untagged_node = next(
            (node for node in data['tree'] if '태그 없음' in node['label'] or 'Untagged' in node['label']),
            None
        )

        assert untagged_node is not None
        assert untagged_node['type'] == 'tag'
        assert untagged_node['count'] == 2
        assert len(untagged_node['children']) == 2

        # Verify untagged documents are in the children
        untagged_doc_ids = {child['documentId'] for child in untagged_node['children']}
        assert doc1.id in untagged_doc_ids
        assert doc2.id in untagged_doc_ids

        # Verify tagged document is separate
        python_node = next(node for node in data['tree'] if node['label'] == 'python')
        assert python_node['count'] == 1
        assert len(python_node['children']) == 1
        assert python_node['children'][0]['documentId'] == doc3.id
