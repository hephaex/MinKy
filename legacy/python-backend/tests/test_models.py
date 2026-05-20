"""
Tests for data models.

Tests cover Document, Tag, Category, and User models including:
- Model creation and field validation
- Data serialization (to_dict)
- Business logic methods
- Slug generation
- Password hashing
- Hierarchical relationships
"""
from datetime import datetime, timezone

import pytest

from app import db
from app.models.category import Category
from app.models.document import Document
from app.models.tag import Tag
from app.models.user import User


def test_document_creation(app):
    """Test document creation with all required fields."""
    with app.app_context():
        doc = Document(
            title='Sample Document',
            markdown_content='# Header\n\nContent here.',
            author='John Doe',
            user_id=None,
            is_public=True,
            document_metadata={'version': '1.0', 'tags': ['test']}
        )
        db.session.add(doc)
        db.session.commit()

        assert doc.id is not None
        assert doc.title == 'Sample Document'
        assert doc.author == 'John Doe'
        assert doc.markdown_content == '# Header\n\nContent here.'
        assert doc.is_public is True
        assert doc.document_metadata == {'version': '1.0', 'tags': ['test']}
        assert doc.created_at is not None
        assert doc.updated_at is not None
        assert isinstance(doc.created_at, datetime)


def test_document_to_dict(app):
    """Test document serialization to dictionary."""
    with app.app_context():
        doc = Document(
            title='Test Doc',
            markdown_content='# Test\n\nContent.',
            author='Jane Smith',
            is_public=False
        )
        db.session.add(doc)
        db.session.commit()

        doc_dict = doc.to_dict()

        assert doc_dict['id'] == doc.id
        assert doc_dict['title'] == 'Test Doc'
        assert doc_dict['author'] == 'Jane Smith'
        assert doc_dict['markdown_content'] == '# Test\n\nContent.'
        assert doc_dict['html_content'] is not None
        assert doc_dict['is_public'] is False
        assert doc_dict['is_published'] is False
        assert 'created_at' in doc_dict
        assert 'updated_at' in doc_dict
        assert 'tags' in doc_dict
        assert 'tag_names' in doc_dict
        assert 'comment_count' in doc_dict
        assert 'rating_stats' in doc_dict
        assert 'version_count' in doc_dict
        assert 'latest_version' in doc_dict


def test_document_html_conversion(app):
    """Test markdown to HTML conversion."""
    with app.app_context():
        markdown_text = '# Title\n\n**Bold** and *italic* text.\n\n- List item 1\n- List item 2'
        doc = Document(
            title='Markdown Test',
            markdown_content=markdown_text
        )
        db.session.add(doc)
        db.session.commit()

        assert doc.html_content is not None
        assert '<h1>Title</h1>' in doc.html_content
        assert '<strong>Bold</strong>' in doc.html_content
        assert '<em>italic</em>' in doc.html_content
        assert '<li>List item 1</li>' in doc.html_content
        assert '<ul>' in doc.html_content


def test_tag_get_or_create(app):
    """Test tag get_or_create method creates tag with proper slug."""
    with app.app_context():
        tag1 = Tag.get_or_create('Python Programming')
        db.session.commit()

        assert tag1.id is not None
        assert tag1.name == 'Python Programming'
        assert tag1.slug == 'python-programming'
        assert tag1.color == '#007bff'

        # Get the same tag again
        tag2 = Tag.get_or_create('Python Programming')
        db.session.commit()

        assert tag1.id == tag2.id
        assert tag1.slug == tag2.slug


def test_tag_create_slug(app):
    """Test slug creation with special characters."""
    with app.app_context():
        # Test with special characters - consecutive special chars collapse to single hyphen
        slug1 = Tag.create_slug('C++ Programming!')
        assert '-' in slug1
        assert 'c' in slug1
        assert 'programming' in slug1

        # Test with spaces and underscores
        slug2 = Tag.create_slug('web_development basics')
        assert 'web' in slug2
        assert 'development' in slug2 or 'basics' in slug2

        # Test with multiple spaces
        slug3 = Tag.create_slug('   Machine   Learning   ')
        assert 'machine' in slug3
        assert 'learning' in slug3

        # Test lowercase conversion
        slug4 = Tag.create_slug('Python')
        assert slug4 == 'python'

        # Test with forward slash
        slug5 = Tag.create_slug('Frontend/Backend')
        assert 'frontend' in slug5
        assert 'backend' in slug5


def test_tag_create_slug_korean(app):
    """Test slug creation with Korean characters."""
    with app.app_context():
        # Test Korean characters are preserved
        slug1 = Tag.create_slug('파이썬 프로그래밍')
        assert '파이썬' in slug1
        assert '프로그래밍' in slug1

        # Test mixed Korean and English
        slug2 = Tag.create_slug('Python 파이썬')
        assert 'python' in slug2
        assert '파이썬' in slug2

        # Test Korean with special characters
        slug3 = Tag.create_slug('웹개발 #2024')
        assert '웹개발' in slug3
        assert '2024' in slug3

        # Verify tag creation with Korean
        tag = Tag.get_or_create('머신러닝')
        db.session.commit()

        assert tag.name == '머신러닝'
        assert '머신러닝' in tag.slug


def test_category_creation(app):
    """Test category creation with all fields."""
    with app.app_context():
        category = Category(
            name='Technology',
            description='Tech-related documents',
            parent_id=None,
            created_by=None,
            color='#ff5733'
        )
        db.session.add(category)
        db.session.commit()

        assert category.id is not None
        assert category.name == 'Technology'
        assert category.slug == 'technology'
        assert category.description == 'Tech-related documents'
        assert category.parent_id is None
        assert category.color == '#ff5733'
        assert category.sort_order == 0
        assert category.is_active is True
        assert category.created_at is not None


def test_category_hierarchy(app):
    """Test category parent-child relationships and path methods."""
    with app.app_context():
        # Create parent category
        parent = Category(name='Programming', description='All programming topics')
        db.session.add(parent)
        db.session.commit()

        # Create child category
        child = Category(
            name='Python',
            description='Python programming',
            parent_id=parent.id
        )
        db.session.add(child)
        db.session.commit()

        # Create grandchild category
        grandchild = Category(
            name='Django',
            description='Django framework',
            parent_id=child.id
        )
        db.session.add(grandchild)
        db.session.commit()

        # Test get_full_path for grandchild
        path = grandchild.get_full_path()
        assert len(path) == 3
        assert path[0].name == 'Programming'
        assert path[1].name == 'Python'
        assert path[2].name == 'Django'

        # Test get_path_string
        path_string = grandchild.get_path_string()
        assert path_string == 'Programming > Python > Django'

        # Test custom separator
        path_string_custom = grandchild.get_path_string(separator=' / ')
        assert path_string_custom == 'Programming / Python / Django'

        # Test parent relationship
        assert child.parent.id == parent.id
        assert grandchild.parent.id == child.id

        # Test children relationship
        assert len(list(parent.children)) == 1
        assert list(parent.children)[0].id == child.id


def test_user_password_hashing(app):
    """Test user password hashing and verification."""
    with app.app_context():
        user = User(
            username='johndoe',
            email='john@example.com',
            password='SecurePassword123!',
            full_name='John Doe'
        )
        db.session.add(user)
        db.session.commit()

        # Password should be hashed
        assert user.password_hash is not None
        assert user.password_hash != 'SecurePassword123!'
        assert len(user.password_hash) > 50

        # Check password should work with correct password
        assert user.check_password('SecurePassword123!') is True

        # Check password should fail with incorrect password
        assert user.check_password('WrongPassword') is False
        assert user.check_password('') is False
        assert user.check_password('SecurePassword123') is False

        # Test password change
        user.set_password('NewPassword456!')
        db.session.commit()

        assert user.check_password('NewPassword456!') is True
        assert user.check_password('SecurePassword123!') is False


def test_user_find_methods(app):
    """Test user find_by_username and find_by_email methods."""
    with app.app_context():
        # Create test users
        user1 = User(
            username='alice',
            email='alice@example.com',
            password='password123',
            full_name='Alice Smith'
        )
        user2 = User(
            username='bob',
            email='bob@example.com',
            password='password456',
            full_name='Bob Johnson'
        )
        db.session.add(user1)
        db.session.add(user2)
        db.session.commit()

        # Test find_by_username
        found_user1 = User.find_by_username('alice')
        assert found_user1 is not None
        assert found_user1.username == 'alice'
        assert found_user1.email == 'alice@example.com'
        assert found_user1.full_name == 'Alice Smith'

        found_user2 = User.find_by_username('bob')
        assert found_user2 is not None
        assert found_user2.username == 'bob'

        # Test find_by_username with non-existent user
        not_found = User.find_by_username('charlie')
        assert not_found is None

        # Test find_by_email
        found_by_email1 = User.find_by_email('alice@example.com')
        assert found_by_email1 is not None
        assert found_by_email1.username == 'alice'
        assert found_by_email1.id == found_user1.id

        found_by_email2 = User.find_by_email('bob@example.com')
        assert found_by_email2 is not None
        assert found_by_email2.username == 'bob'

        # Test find_by_email with non-existent email
        not_found_email = User.find_by_email('notfound@example.com')
        assert not_found_email is None

        # Test case sensitivity
        case_test = User.find_by_username('ALICE')
        assert case_test is None  # Exact match required
