# Documents API

## Overview

The Documents API provides CRUD operations for managing knowledge documents in MinKy. Documents are the core content units that can be analyzed by AI, embedded for semantic search, and organized with tags and categories.

## Table of Contents

- [List Documents](#list-documents)
- [Create Document](#create-document)
- [Get Document](#get-document)
- [Update Document](#update-document)
- [Delete Document](#delete-document)
- [Data Models](#data-models)

---

## List Documents

Retrieve a paginated list of documents with optional filtering.

### Endpoint

```
GET /api/documents
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| page | integer | 1 | Page number |
| limit | integer | 20 | Items per page (max: 100) |
| category_id | integer | - | Filter by category |
| search | string | - | Search in title and content |

### Example Request

```bash
curl -X GET "http://localhost:3000/api/documents?page=1&limit=10&search=kubernetes" \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Kubernetes Deployment Guide",
      "content": "# Introduction\n\nThis guide covers...",
      "category_id": 1,
      "user_id": 1,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-16T14:20:00Z"
    }
  ],
  "meta": {
    "total": 25,
    "page": 1,
    "limit": 10,
    "total_pages": 3
  }
}
```

---

## Create Document

Create a new document.

### Endpoint

```
POST /api/documents
```

### Request Body

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| title | string | Yes | 1-500 characters |
| content | string | Yes | Markdown content |
| category_id | integer | No | Valid category ID |

### Example Request

```bash
curl -X POST http://localhost:3000/api/documents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "How to Deploy to Kubernetes",
    "content": "# Kubernetes Deployment\n\n## Prerequisites\n\n- kubectl installed\n- Access to cluster\n\n## Steps\n\n1. Create deployment YAML\n2. Apply configuration\n3. Verify pods",
    "category_id": 1
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "title": "How to Deploy to Kubernetes",
    "content": "# Kubernetes Deployment\n\n## Prerequisites...",
    "category_id": 1,
    "user_id": 1,
    "created_at": "2024-01-20T09:00:00Z",
    "updated_at": "2024-01-20T09:00:00Z"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 400 | Validation error (title length, missing required fields) |
| 401 | Authentication required |

---

## Get Document

Retrieve a single document by ID.

### Endpoint

```
GET /api/documents/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Example Request

```bash
curl -X GET http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "title": "How to Deploy to Kubernetes",
    "content": "# Kubernetes Deployment\n\n## Prerequisites...",
    "category_id": 1,
    "user_id": 1,
    "created_at": "2024-01-20T09:00:00Z",
    "updated_at": "2024-01-20T09:00:00Z"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 404 | Document not found |

---

## Update Document

Update an existing document.

### Endpoint

```
PUT /api/documents/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Request Body

| Field | Type | Required | Validation |
|-------|------|----------|------------|
| title | string | No | 1-500 characters |
| content | string | No | Markdown content |
| category_id | integer | No | Valid category ID |

All fields are optional. Only provided fields will be updated.

### Example Request

```bash
curl -X PUT http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440001 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "Updated: How to Deploy to Kubernetes",
    "content": "# Kubernetes Deployment\n\nUpdated content with more details..."
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "title": "Updated: How to Deploy to Kubernetes",
    "content": "# Kubernetes Deployment\n\nUpdated content with more details...",
    "category_id": 1,
    "user_id": 1,
    "created_at": "2024-01-20T09:00:00Z",
    "updated_at": "2024-01-20T15:30:00Z"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 400 | Validation error |
| 403 | Not authorized to update this document |
| 404 | Document not found |

---

## Delete Document

Delete a document.

### Endpoint

```
DELETE /api/documents/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Example Request

```bash
curl -X DELETE http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "message": "Document deleted successfully"
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 403 | Not authorized to delete this document |
| 404 | Document not found |

---

## Data Models

### Document

```typescript
interface Document {
  id: string;          // UUID
  title: string;       // Document title (1-500 chars)
  content: string;     // Markdown content
  category_id: number | null;  // Optional category reference
  user_id: number;     // Owner user ID
  created_at: string;  // ISO 8601 timestamp
  updated_at: string;  // ISO 8601 timestamp
}
```

### CreateDocumentRequest

```typescript
interface CreateDocumentRequest {
  title: string;           // Required, 1-500 chars
  content: string;         // Required, Markdown
  category_id?: number;    // Optional
}
```

### UpdateDocumentRequest

```typescript
interface UpdateDocumentRequest {
  title?: string;          // Optional, 1-500 chars
  content?: string;        // Optional, Markdown
  category_id?: number;    // Optional
}
```

### ListResponse

```typescript
interface ListResponse<T> {
  success: boolean;
  data: T[];
  meta: {
    total: number;
    page: number;
    limit: number;
    total_pages: number;
  };
}
```

---

## Best Practices

### Content Format

Documents support full Markdown syntax:

```markdown
# Main Heading

## Section

Regular paragraph text.

- Bullet points
- More items

1. Numbered list
2. Second item

```code
Code blocks
```

> Blockquotes

**Bold** and *italic* text
```

### Organizing Documents

1. **Use Categories** - Organize related documents into categories
2. **Add Tags** - Use the Tags API to add searchable labels
3. **Meaningful Titles** - Use descriptive titles for better search results
4. **Structure Content** - Use headings to structure long documents

### Integration with AI Features

After creating a document, you can:

1. **Generate Embeddings** - `POST /api/embeddings/document/{id}`
2. **Trigger AI Understanding** - `POST /api/documents/{id}/understand`
3. **Enable Semantic Search** - Documents with embeddings appear in semantic search results

---

## Example: Complete Document Workflow

```bash
# 1. Create document
DOC_ID=$(curl -s -X POST http://localhost:3000/api/documents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "Team Coding Standards",
    "content": "# Coding Standards\n\n## Naming Conventions\n\n- Use camelCase for variables\n- Use PascalCase for classes",
    "category_id": 1
  }' | jq -r '.data.id')

echo "Created document: $DOC_ID"

# 2. Generate embedding for semantic search
curl -X POST http://localhost:3000/api/embeddings/document/$DOC_ID \
  -H "Authorization: Bearer $TOKEN"

# 3. Trigger AI understanding
curl -X POST http://localhost:3000/api/documents/$DOC_ID/understand \
  -H "Authorization: Bearer $TOKEN"

# 4. Document is now searchable semantically and has AI-generated insights
curl -X POST http://localhost:3000/api/embeddings/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"query": "What are our naming conventions?", "limit": 5}'
```
