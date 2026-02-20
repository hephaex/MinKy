# MinKy API Examples

## Overview

This document provides practical examples for common API use cases in MinKy. All examples use curl and can be adapted to any HTTP client or programming language.

## Table of Contents

1. [Setup](#setup)
2. [Authentication Flow](#authentication-flow)
3. [Document Management](#document-management)
4. [AI-Powered Features](#ai-powered-features)
5. [Semantic Search](#semantic-search)
6. [Complete Workflows](#complete-workflows)
7. [JavaScript/TypeScript Examples](#javascripttypescript-examples)
8. [Python Examples](#python-examples)

---

## Setup

### Environment Variables

```bash
export MINKY_BASE_URL="http://localhost:3000/api"
export MINKY_TOKEN=""  # Set after login
```

### Helper Function

```bash
# Helper function for authenticated requests
minky_api() {
  local method=$1
  local endpoint=$2
  local data=$3

  if [ -n "$data" ]; then
    curl -s -X "$method" "${MINKY_BASE_URL}${endpoint}" \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $MINKY_TOKEN" \
      -d "$data"
  else
    curl -s -X "$method" "${MINKY_BASE_URL}${endpoint}" \
      -H "Authorization: Bearer $MINKY_TOKEN"
  fi
}
```

---

## Authentication Flow

### Register a New User

```bash
curl -X POST "$MINKY_BASE_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "developer@example.com",
    "username": "developer",
    "password": "secure_password_123"
  }'
```

### Login and Store Token

```bash
# Login and extract token
RESPONSE=$(curl -s -X POST "$MINKY_BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "developer@example.com",
    "password": "secure_password_123"
  }')

export MINKY_TOKEN=$(echo $RESPONSE | jq -r '.access_token')
export REFRESH_TOKEN=$(echo $RESPONSE | jq -r '.refresh_token')

echo "Logged in! Token: ${MINKY_TOKEN:0:20}..."
```

### Refresh Token

```bash
RESPONSE=$(curl -s -X POST "$MINKY_BASE_URL/auth/refresh" \
  -H "Content-Type: application/json" \
  -d "{\"refresh_token\": \"$REFRESH_TOKEN\"}")

export MINKY_TOKEN=$(echo $RESPONSE | jq -r '.access_token')
```

---

## Document Management

### Create a Document

```bash
DOC_RESPONSE=$(curl -s -X POST "$MINKY_BASE_URL/documents" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "title": "Team Onboarding Guide",
    "content": "# Welcome to the Team\n\n## First Week\n\n1. Set up development environment\n2. Review codebase architecture\n3. Complete security training\n\n## Development Setup\n\n### Prerequisites\n\n- Node.js 18+\n- Docker Desktop\n- VS Code with recommended extensions\n\n### Getting Started\n\n```bash\ngit clone https://github.com/team/project\ncd project\nnpm install\nnpm run dev\n```\n\n## Key Contacts\n\n- Tech Lead: Alice (alice@example.com)\n- DevOps: Bob (bob@example.com)",
    "category_id": 1
  }')

DOC_ID=$(echo $DOC_RESPONSE | jq -r '.data.id')
echo "Created document: $DOC_ID"
```

### List Documents with Pagination

```bash
# Page 1, 10 items per page
curl -s "$MINKY_BASE_URL/documents?page=1&limit=10" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data[] | {id, title}'
```

### Search Documents

```bash
# Search for "onboarding" in category 1
curl -s "$MINKY_BASE_URL/documents?search=onboarding&category_id=1" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data'
```

### Update a Document

```bash
curl -X PUT "$MINKY_BASE_URL/documents/$DOC_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "title": "Team Onboarding Guide (Updated)",
    "content": "# Welcome to the Team\n\nUpdated content..."
  }'
```

### Delete a Document

```bash
curl -X DELETE "$MINKY_BASE_URL/documents/$DOC_ID" \
  -H "Authorization: Bearer $MINKY_TOKEN"
```

---

## AI-Powered Features

### Generate AI Suggestions

#### Title Suggestion

```bash
curl -X POST "$MINKY_BASE_URL/ai/suggest/title" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "content": "This document explains how to set up continuous integration pipelines using GitHub Actions. It covers workflow syntax, job configuration, matrix builds, and secrets management."
  }' | jq '.data.suggestion'

# Output: "GitHub Actions CI/CD Pipeline Configuration Guide"
```

#### Summary Generation

```bash
curl -X POST "$MINKY_BASE_URL/ai/suggest/summary" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "content": "# Error Handling in Rust\n\nRust provides two main approaches to error handling: the Result type for recoverable errors and panic! for unrecoverable errors. The Result enum has two variants: Ok(T) for success and Err(E) for errors. The ? operator provides ergonomic error propagation. Custom error types can be created using thiserror or anyhow crates."
  }' | jq '.data.suggestion'

# Output: "A concise overview of Rust error handling patterns using Result types, the ? operator, and popular error handling crates."
```

#### Tag Suggestions

```bash
curl -X POST "$MINKY_BASE_URL/ai/suggest/tags" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "content": "Deploying to Kubernetes using Helm charts. This guide covers chart structure, values files, and release management."
  }' | jq '.data.suggestion'

# Output: "kubernetes, helm, deployment, devops, containers"
```

#### Text Improvement

```bash
curl -X POST "$MINKY_BASE_URL/ai/improve" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "content": "the app sometime crash when user click button fast, we need fix this bug asap",
    "context": "Bug report for mobile app"
  }' | jq '.data.suggestion'
```

### Document Understanding

```bash
# Trigger AI analysis
curl -X POST "$MINKY_BASE_URL/documents/$DOC_ID/understand" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data'

# Get cached understanding
curl -s "$MINKY_BASE_URL/documents/$DOC_ID/understanding" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data | {topics, summary, technologies, relevant_for}'
```

---

## Semantic Search

### Generate Embeddings

```bash
# Generate document-level embedding
curl -X POST "$MINKY_BASE_URL/embeddings/document/$DOC_ID" \
  -H "Authorization: Bearer $MINKY_TOKEN"

# Generate chunk embeddings
curl -X POST "$MINKY_BASE_URL/embeddings/chunks/$DOC_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "document_id": "'$DOC_ID'",
    "chunks": [
      {"text": "Set up development environment with Node.js and Docker...", "start_offset": 0, "end_offset": 200},
      {"text": "Review codebase architecture and understand the module structure...", "start_offset": 201, "end_offset": 400}
    ]
  }'
```

### Semantic Search

```bash
# Natural language search
curl -X POST "$MINKY_BASE_URL/embeddings/search" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "query": "How do I set up my development environment?",
    "limit": 5,
    "threshold": 0.7
  }' | jq '.data[] | {document_title, similarity, chunk_text}'
```

### Find Similar Documents

```bash
curl -s "$MINKY_BASE_URL/embeddings/similar/$DOC_ID?limit=5" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data[] | {document_title, similarity}'
```

### Embedding Statistics

```bash
curl -s "$MINKY_BASE_URL/embeddings/stats" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.'
```

---

## Complete Workflows

### Workflow 1: New Document with Full AI Processing

```bash
#!/bin/bash
# Complete document creation workflow with AI processing

# 1. Create the document
DOC=$(curl -s -X POST "$MINKY_BASE_URL/documents" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{
    "title": "Microservices Communication Patterns",
    "content": "# Microservices Communication\n\n## Synchronous Patterns\n\n### REST APIs\nREST provides simple, stateless communication...\n\n### gRPC\ngRPC offers high-performance binary communication...\n\n## Asynchronous Patterns\n\n### Message Queues\nUse RabbitMQ or Kafka for decoupled communication...\n\n### Event Sourcing\nStore events as the source of truth..."
  }')

DOC_ID=$(echo $DOC | jq -r '.data.id')
echo "1. Created document: $DOC_ID"

# 2. Generate AI understanding
echo "2. Analyzing document..."
curl -s -X POST "$MINKY_BASE_URL/documents/$DOC_ID/understand" \
  -H "Authorization: Bearer $MINKY_TOKEN" > /dev/null

# 3. Generate embeddings for search
echo "3. Generating embeddings..."
curl -s -X POST "$MINKY_BASE_URL/embeddings/document/$DOC_ID" \
  -H "Authorization: Bearer $MINKY_TOKEN" > /dev/null

# 4. Get the complete document with understanding
echo "4. Document ready!"
echo ""
echo "=== Document Info ==="
curl -s "$MINKY_BASE_URL/documents/$DOC_ID" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data | {id, title}'

echo ""
echo "=== AI Understanding ==="
curl -s "$MINKY_BASE_URL/documents/$DOC_ID/understanding" \
  -H "Authorization: Bearer $MINKY_TOKEN" | jq '.data | {topics, summary, technologies}'

echo ""
echo "=== Now Searchable! ==="
curl -s -X POST "$MINKY_BASE_URL/embeddings/search" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MINKY_TOKEN" \
  -d '{"query": "What patterns can I use for microservice communication?", "limit": 3}' | \
  jq '.data[] | {document_title, similarity}'
```

### Workflow 2: Batch Import and Process

```bash
#!/bin/bash
# Import multiple documents and process them

DOCS=(
  '{"title": "Docker Basics", "content": "# Docker\n\n## Images\n\nDocker images are..."}'
  '{"title": "Kubernetes Pods", "content": "# Pods\n\n## Overview\n\nPods are the smallest deployable units..."}'
  '{"title": "Helm Charts", "content": "# Helm\n\n## Chart Structure\n\nA Helm chart contains..."}'
)

for doc in "${DOCS[@]}"; do
  # Create document
  RESULT=$(curl -s -X POST "$MINKY_BASE_URL/documents" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $MINKY_TOKEN" \
    -d "$doc")

  DOC_ID=$(echo $RESULT | jq -r '.data.id')
  TITLE=$(echo $RESULT | jq -r '.data.title')
  echo "Created: $TITLE ($DOC_ID)"

  # Queue for embedding (async)
  curl -s -X POST "$MINKY_BASE_URL/embeddings/queue/$DOC_ID" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $MINKY_TOKEN" \
    -d '{"priority": 5}' > /dev/null

  # Trigger understanding (async)
  curl -s -X POST "$MINKY_BASE_URL/documents/$DOC_ID/understand" \
    -H "Authorization: Bearer $MINKY_TOKEN" > /dev/null &

  sleep 1  # Rate limiting
done

echo ""
echo "All documents queued for processing!"
```

### Workflow 3: Knowledge Q&A System

```bash
#!/bin/bash
# Interactive Q&A using semantic search

ask_question() {
  local question="$1"

  echo "Q: $question"
  echo ""

  RESULTS=$(curl -s -X POST "$MINKY_BASE_URL/embeddings/search" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $MINKY_TOKEN" \
    -d "{\"query\": \"$question\", \"limit\": 3, \"threshold\": 0.6}")

  TOP_RESULT=$(echo $RESULTS | jq -r '.data[0]')

  if [ "$TOP_RESULT" != "null" ]; then
    TITLE=$(echo $TOP_RESULT | jq -r '.document_title')
    SIMILARITY=$(echo $TOP_RESULT | jq -r '.similarity')
    CHUNK=$(echo $TOP_RESULT | jq -r '.chunk_text // "No chunk available"')

    echo "A: Found in \"$TITLE\" (${SIMILARITY}% match)"
    echo ""
    echo "   $CHUNK"
  else
    echo "A: No relevant documents found."
  fi

  echo ""
  echo "---"
  echo ""
}

# Example questions
ask_question "How do I deploy to Kubernetes?"
ask_question "What are the coding standards for our team?"
ask_question "How do I handle errors in async code?"
```

---

## JavaScript/TypeScript Examples

### API Client Class

```typescript
// minky-client.ts
interface MinkyConfig {
  baseUrl: string;
  token?: string;
}

interface Document {
  id: string;
  title: string;
  content: string;
  category_id?: number;
  created_at: string;
  updated_at: string;
}

interface SearchResult {
  document_id: string;
  document_title: string;
  chunk_text?: string;
  similarity: number;
}

class MinkyClient {
  private baseUrl: string;
  private token: string | null = null;

  constructor(config: MinkyConfig) {
    this.baseUrl = config.baseUrl;
    this.token = config.token || null;
  }

  private async request<T>(
    method: string,
    endpoint: string,
    body?: object
  ): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'API request failed');
    }

    return response.json();
  }

  // Auth
  async login(email: string, password: string): Promise<void> {
    const response = await this.request<{
      access_token: string;
      refresh_token: string;
    }>('POST', '/auth/login', { email, password });

    this.token = response.access_token;
  }

  // Documents
  async createDocument(
    title: string,
    content: string,
    categoryId?: number
  ): Promise<Document> {
    const response = await this.request<{ data: Document }>(
      'POST',
      '/documents',
      { title, content, category_id: categoryId }
    );
    return response.data;
  }

  async getDocument(id: string): Promise<Document> {
    const response = await this.request<{ data: Document }>(
      'GET',
      `/documents/${id}`
    );
    return response.data;
  }

  async listDocuments(params?: {
    page?: number;
    limit?: number;
    search?: string;
  }): Promise<{ data: Document[]; meta: any }> {
    const query = new URLSearchParams();
    if (params?.page) query.set('page', params.page.toString());
    if (params?.limit) query.set('limit', params.limit.toString());
    if (params?.search) query.set('search', params.search);

    return this.request('GET', `/documents?${query}`);
  }

  // AI Features
  async analyzeDocument(documentId: string): Promise<any> {
    return this.request('POST', `/documents/${documentId}/understand`);
  }

  async getUnderstanding(documentId: string): Promise<any> {
    return this.request('GET', `/documents/${documentId}/understanding`);
  }

  async generateEmbedding(documentId: string): Promise<void> {
    await this.request('POST', `/embeddings/document/${documentId}`);
  }

  // Search
  async semanticSearch(
    query: string,
    limit = 10,
    threshold = 0.7
  ): Promise<SearchResult[]> {
    const response = await this.request<{ data: SearchResult[] }>(
      'POST',
      '/embeddings/search',
      { query, limit, threshold }
    );
    return response.data;
  }

  async findSimilar(documentId: string, limit = 10): Promise<SearchResult[]> {
    const response = await this.request<{ data: SearchResult[] }>(
      'GET',
      `/embeddings/similar/${documentId}?limit=${limit}`
    );
    return response.data;
  }

  // AI Suggestions
  async suggestTitle(content: string): Promise<string> {
    const response = await this.request<{ data: { suggestion: string } }>(
      'POST',
      '/ai/suggest/title',
      { content }
    );
    return response.data.suggestion;
  }

  async suggestSummary(content: string): Promise<string> {
    const response = await this.request<{ data: { suggestion: string } }>(
      'POST',
      '/ai/suggest/summary',
      { content }
    );
    return response.data.suggestion;
  }
}

// Usage
async function main() {
  const client = new MinkyClient({
    baseUrl: 'http://localhost:3000/api',
  });

  // Login
  await client.login('user@example.com', 'password123');

  // Create and process document
  const doc = await client.createDocument(
    'API Design Guide',
    '# REST API Design\n\nBest practices for designing RESTful APIs...'
  );

  // Generate AI insights
  await client.analyzeDocument(doc.id);
  await client.generateEmbedding(doc.id);

  // Search
  const results = await client.semanticSearch('How to design REST APIs?');
  console.log('Search results:', results);
}
```

---

## Python Examples

### API Client

```python
# minky_client.py
import requests
from typing import Optional, List, Dict, Any

class MinkyClient:
    def __init__(self, base_url: str, token: Optional[str] = None):
        self.base_url = base_url.rstrip('/')
        self.token = token

    def _headers(self) -> Dict[str, str]:
        headers = {'Content-Type': 'application/json'}
        if self.token:
            headers['Authorization'] = f'Bearer {self.token}'
        return headers

    def _request(self, method: str, endpoint: str, **kwargs) -> Any:
        url = f"{self.base_url}{endpoint}"
        response = requests.request(
            method, url, headers=self._headers(), **kwargs
        )
        response.raise_for_status()
        return response.json()

    # Auth
    def login(self, email: str, password: str) -> None:
        data = self._request('POST', '/auth/login', json={
            'email': email,
            'password': password
        })
        self.token = data['access_token']

    # Documents
    def create_document(
        self,
        title: str,
        content: str,
        category_id: Optional[int] = None
    ) -> Dict:
        return self._request('POST', '/documents', json={
            'title': title,
            'content': content,
            'category_id': category_id
        })['data']

    def list_documents(
        self,
        page: int = 1,
        limit: int = 20,
        search: Optional[str] = None
    ) -> Dict:
        params = {'page': page, 'limit': limit}
        if search:
            params['search'] = search
        return self._request('GET', '/documents', params=params)

    # AI
    def analyze_document(self, document_id: str) -> Dict:
        return self._request('POST', f'/documents/{document_id}/understand')

    def get_understanding(self, document_id: str) -> Dict:
        return self._request('GET', f'/documents/{document_id}/understanding')

    def generate_embedding(self, document_id: str) -> None:
        self._request('POST', f'/embeddings/document/{document_id}')

    # Search
    def semantic_search(
        self,
        query: str,
        limit: int = 10,
        threshold: float = 0.7
    ) -> List[Dict]:
        return self._request('POST', '/embeddings/search', json={
            'query': query,
            'limit': limit,
            'threshold': threshold
        })['data']

    def find_similar(self, document_id: str, limit: int = 10) -> List[Dict]:
        return self._request(
            'GET',
            f'/embeddings/similar/{document_id}',
            params={'limit': limit}
        )['data']


# Usage
if __name__ == '__main__':
    client = MinkyClient('http://localhost:3000/api')
    client.login('user@example.com', 'password123')

    # Create document
    doc = client.create_document(
        title='Python Best Practices',
        content='# Python Best Practices\n\n## Code Style\n\nFollow PEP 8...'
    )
    print(f"Created document: {doc['id']}")

    # Analyze
    client.analyze_document(doc['id'])
    client.generate_embedding(doc['id'])

    # Search
    results = client.semantic_search('How to write clean Python code?')
    for r in results:
        print(f"- {r['document_title']} ({r['similarity']:.2%})")
```

### Batch Processing Script

```python
#!/usr/bin/env python3
# batch_import.py
import glob
import os
from pathlib import Path
from minky_client import MinkyClient

def import_markdown_files(client: MinkyClient, directory: str) -> None:
    """Import all markdown files from a directory."""
    md_files = glob.glob(os.path.join(directory, '**/*.md'), recursive=True)

    for filepath in md_files:
        path = Path(filepath)
        title = path.stem.replace('-', ' ').replace('_', ' ').title()

        with open(filepath, 'r') as f:
            content = f.read()

        try:
            # Create document
            doc = client.create_document(title=title, content=content)
            print(f"Imported: {title}")

            # Process with AI
            client.analyze_document(doc['id'])
            client.generate_embedding(doc['id'])
            print(f"  - Analyzed and embedded")

        except Exception as e:
            print(f"Failed to import {filepath}: {e}")


if __name__ == '__main__':
    import sys

    if len(sys.argv) < 2:
        print("Usage: python batch_import.py <directory>")
        sys.exit(1)

    client = MinkyClient('http://localhost:3000/api')
    client.login('admin@example.com', 'admin_password')

    import_markdown_files(client, sys.argv[1])
```
