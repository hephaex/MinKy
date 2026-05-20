"""
Tests for Auto Tag Generation API
"""
import pytest
import json


def test_tagless_documents(client):
    """Test getting documents without tags"""
    response = client.get('/api/tags/tagless-documents')

    assert response.status_code == 200
    data = json.loads(response.data)

    assert 'documents' in data
    assert 'pagination' in data


def test_preview_auto_tags(client, sample_document):
    """Test previewing auto tags for a specific document"""
    response = client.get(f'/api/tags/preview-auto-tags/{sample_document}')

    assert response.status_code == 200
    data = json.loads(response.data)

    assert data['success'] is True
    assert 'document' in data['data']
    assert 'detected_auto_tags' in data['data']


def test_auto_generate_dry_run(client):
    """Test auto-generating tags in dry run mode"""
    payload = {
        "dry_run": True,
        "limit": 5
    }

    response = client.post('/api/tags/auto-generate',
                          data=json.dumps(payload),
                          content_type='application/json')

    assert response.status_code == 200
    data = json.loads(response.data)

    assert data['success'] is True
    assert 'dry_run' in data['data']
    assert data['data']['dry_run'] is True
    assert 'summary' in data['data']


@pytest.mark.skip(reason="Modifies database - run manually")
def test_auto_generate_real(client):
    """Test actually generating tags (skipped by default)"""
    payload = {
        "dry_run": False,
        "limit": 2
    }

    response = client.post('/api/tags/auto-generate',
                          data=json.dumps(payload),
                          content_type='application/json')

    assert response.status_code == 200
    data = json.loads(response.data)

    assert 'summary' in data
    assert 'results' in data
