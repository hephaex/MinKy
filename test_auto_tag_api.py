#!/usr/bin/env python3

import requests
import json

# API base URL
BASE_URL = "http://localhost:5000/api"

def test_tagless_documents():
    """Test getting documents without tags"""
    url = f"{BASE_URL}/tags/tagless-documents"
    response = requests.get(url)
    
    print(f"GET {url}")
    print(f"Status: {response.status_code}")
    
    if response.status_code == 200:
        data = response.json()
        print(f"Found {len(data['documents'])} tagless documents")
        print(f"Total: {data['pagination']['total']}")
        
        # Show first few documents with preview tags
        for doc in data['documents'][:3]:
            print(f"- {doc['title']}: {doc['preview_auto_tags']}")
    else:
        print(f"Error: {response.text}")
    
    print("-" * 50)

def test_preview_auto_tags(document_id):
    """Test previewing auto tags for a specific document"""
    url = f"{BASE_URL}/tags/preview-auto-tags/{document_id}"
    response = requests.get(url)
    
    print(f"GET {url}")
    print(f"Status: {response.status_code}")
    
    if response.status_code == 200:
        data = response.json()
        print(f"Document: {data['document']['title']}")
        print(f"Existing tags: {data['existing_tags']}")
        print(f"Detected auto tags: {data['detected_auto_tags']}")
        print(f"New tags: {data['new_tags']}")
    else:
        print(f"Error: {response.text}")
    
    print("-" * 50)

def test_auto_generate_dry_run():
    """Test auto-generating tags in dry run mode"""
    url = f"{BASE_URL}/tags/auto-generate"
    payload = {
        "dry_run": True,
        "limit": 5
    }
    
    response = requests.post(url, json=payload)
    
    print(f"POST {url}")
    print(f"Payload: {json.dumps(payload, indent=2)}")
    print(f"Status: {response.status_code}")
    
    if response.status_code == 200:
        data = response.json()
        print(f"Dry run: {data['dry_run']}")
        print(f"Summary: {data['summary']}")
        
        # Show results for first few documents
        for doc in data['results']['documents'][:3]:
            print(f"- {doc['title']}: {doc['status']}")
            if 'would_add_tags' in doc:
                print(f"  Would add: {doc['would_add_tags']}")
    else:
        print(f"Error: {response.text}")
    
    print("-" * 50)

def test_auto_generate_real():
    """Test actually generating tags (be careful!)"""
    url = f"{BASE_URL}/tags/auto-generate"
    payload = {
        "dry_run": False,
        "limit": 2  # Small limit for safety
    }
    
    print("⚠️  WARNING: This will actually modify the database!")
    confirm = input("Do you want to proceed? (y/N): ")
    
    if confirm.lower() != 'y':
        print("Skipped real auto-generation test")
        return
    
    response = requests.post(url, json=payload)
    
    print(f"POST {url}")
    print(f"Payload: {json.dumps(payload, indent=2)}")
    print(f"Status: {response.status_code}")
    
    if response.status_code == 200:
        data = response.json()
        print(f"Summary: {data['summary']}")
        
        for doc in data['results']['documents']:
            print(f"- {doc['title']}: {doc['status']}")
            if 'added_tags' in doc:
                print(f"  Added tags: {doc['added_tags']}")
    else:
        print(f"Error: {response.text}")

if __name__ == "__main__":
    print("Testing Auto Tag Generation API")
    print("=" * 50)
    
    # Test 1: Get tagless documents
    test_tagless_documents()
    
    # Test 2: Preview tags for a specific document (replace with actual ID)
    # test_preview_auto_tags(1)
    
    # Test 3: Dry run auto generation
    test_auto_generate_dry_run()
    
    # Test 4: Real auto generation (commented for safety)
    # test_auto_generate_real()
    
    print("Testing completed!")