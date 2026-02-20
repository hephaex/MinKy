# Embeddings API

## Overview

The Embeddings API provides vector embedding capabilities for semantic search and document similarity. It uses OpenAI's text-embedding-3-small model (or Voyage AI as an alternative) to convert text into high-dimensional vectors stored in PostgreSQL with pgvector.

## Table of Contents

- [Generate Document Embedding](#generate-document-embedding)
- [Get Document Embedding](#get-document-embedding)
- [Generate Chunk Embeddings](#generate-chunk-embeddings)
- [Semantic Search](#semantic-search)
- [Find Similar Documents](#find-similar-documents)
- [Get Statistics](#get-statistics)
- [Queue Document](#queue-document)
- [Data Models](#data-models)
- [Embedding Models](#embedding-models)

---

## Generate Document Embedding

Generate or regenerate the document-level embedding for a specific document.

### Endpoint

```
POST /api/embeddings/document/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Example Request

```bash
curl -X POST http://localhost:3000/api/embeddings/document/550e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440099",
    "document_id": "550e8400-e29b-41d4-a716-446655440001",
    "embedding": [0.0123, -0.0456, 0.0789, ...],
    "model": "openai_text_embedding_3_small",
    "token_count": 256,
    "created_at": "2024-01-20T10:00:00Z",
    "updated_at": "2024-01-20T10:00:00Z"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 404 | Document not found |
| 502 | OpenAI API error |

---

## Get Document Embedding

Retrieve the stored document-level embedding for a document.

### Endpoint

```
GET /api/embeddings/document/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Example Request

```bash
curl -X GET http://localhost:3000/api/embeddings/document/550e8400-e29b-41d4-a716-446655440001 \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "660e8400-e29b-41d4-a716-446655440099",
    "document_id": "550e8400-e29b-41d4-a716-446655440001",
    "embedding": [0.0123, -0.0456, 0.0789, ...],
    "model": "openai_text_embedding_3_small",
    "token_count": 256,
    "created_at": "2024-01-20T10:00:00Z",
    "updated_at": "2024-01-20T10:00:00Z"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 404 | Embedding not found for document |

---

## Generate Chunk Embeddings

Generate chunk-level embeddings for a document. Chunks are smaller segments of the document content that enable more precise semantic search.

### Endpoint

```
POST /api/embeddings/chunks/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Request Body

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| chunks | array | Yes | Array of chunk data |
| model | string | No | Embedding model to use |

### Chunk Data Structure

```json
{
  "text": "Chunk text content...",
  "start_offset": 0,
  "end_offset": 512,
  "metadata": {"section": "introduction"}
}
```

### Example Request

```bash
curl -X POST http://localhost:3000/api/embeddings/chunks/550e8400-e29b-41d4-a716-446655440001 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "document_id": "550e8400-e29b-41d4-a716-446655440001",
    "chunks": [
      {
        "text": "Kubernetes is a container orchestration platform...",
        "start_offset": 0,
        "end_offset": 512,
        "metadata": {"section": "introduction"}
      },
      {
        "text": "To deploy an application, first create a deployment YAML...",
        "start_offset": 513,
        "end_offset": 1024,
        "metadata": {"section": "deployment"}
      }
    ]
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440001",
      "document_id": "550e8400-e29b-41d4-a716-446655440001",
      "chunk_index": 0,
      "chunk_text": "Kubernetes is a container orchestration platform...",
      "chunk_start_offset": 0,
      "chunk_end_offset": 512,
      "embedding": [0.0111, -0.0222, ...],
      "model": "openai_text_embedding_3_small",
      "token_count": 87,
      "metadata": {"section": "introduction"},
      "created_at": "2024-01-20T10:05:00Z"
    },
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "document_id": "550e8400-e29b-41d4-a716-446655440001",
      "chunk_index": 1,
      "chunk_text": "To deploy an application, first create a deployment YAML...",
      "chunk_start_offset": 513,
      "chunk_end_offset": 1024,
      "embedding": [0.0333, -0.0444, ...],
      "model": "openai_text_embedding_3_small",
      "token_count": 92,
      "metadata": {"section": "deployment"},
      "created_at": "2024-01-20T10:05:00Z"
    }
  ]
}
```

---

## Semantic Search

Perform a semantic search against the chunk embedding store.

### Endpoint

```
POST /api/embeddings/search
```

### Request Body

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| query | string | - | Search query (required) |
| limit | integer | 10 | Maximum results (max: 50) |
| threshold | float | 0.7 | Minimum similarity (0.0-1.0) |
| model | string | - | Embedding model for query |
| user_id | integer | - | Filter by user's documents |

### Example Request

```bash
curl -X POST http://localhost:3000/api/embeddings/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "query": "How do I deploy to Kubernetes?",
    "limit": 10,
    "threshold": 0.7
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "document_id": "550e8400-e29b-41d4-a716-446655440001",
      "chunk_id": "770e8400-e29b-41d4-a716-446655440002",
      "chunk_text": "To deploy an application, first create a deployment YAML...",
      "similarity": 0.92,
      "document_title": "Kubernetes Deployment Guide"
    },
    {
      "document_id": "550e8400-e29b-41d4-a716-446655440005",
      "chunk_id": "770e8400-e29b-41d4-a716-446655440010",
      "chunk_text": "Kubernetes deployments can be managed using kubectl...",
      "similarity": 0.87,
      "document_title": "kubectl Commands Reference"
    }
  ]
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 400 | Query must not be empty |

---

## Find Similar Documents

Find documents similar to a given document using vector similarity.

### Endpoint

```
GET /api/embeddings/similar/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID to find similar documents for |

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| limit | integer | 10 | Maximum results (max: 50) |

### Example Request

```bash
curl -X GET "http://localhost:3000/api/embeddings/similar/550e8400-e29b-41d4-a716-446655440001?limit=5" \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "document_id": "550e8400-e29b-41d4-a716-446655440005",
      "chunk_id": null,
      "chunk_text": null,
      "similarity": 0.89,
      "document_title": "Docker Container Guide"
    },
    {
      "document_id": "550e8400-e29b-41d4-a716-446655440008",
      "chunk_id": null,
      "chunk_text": null,
      "similarity": 0.82,
      "document_title": "CI/CD Pipeline Setup"
    }
  ]
}
```

---

## Get Statistics

Retrieve overall embedding statistics.

### Endpoint

```
GET /api/embeddings/stats
```

### Example Request

```bash
curl -X GET http://localhost:3000/api/embeddings/stats \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "total_documents": 150,
    "documents_with_embeddings": 142,
    "total_chunks": 1250,
    "pending_queue": 5,
    "failed_queue": 2
  }
}
```

---

## Queue Document

Add a document to the asynchronous embedding generation queue. Useful for batch processing.

### Endpoint

```
POST /api/embeddings/queue/{id}
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Request Body

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| priority | integer | 0 | Processing priority (higher = first) |

### Example Request

```bash
curl -X POST http://localhost:3000/api/embeddings/queue/550e8400-e29b-41d4-a716-446655440001 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "priority": 10
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "id": "880e8400-e29b-41d4-a716-446655440001",
    "document_id": "550e8400-e29b-41d4-a716-446655440001",
    "status": "pending",
    "priority": 10,
    "attempts": 0,
    "max_attempts": 3,
    "error_message": null,
    "created_at": "2024-01-20T10:30:00Z",
    "started_at": null,
    "completed_at": null
  }
}
```

---

## Data Models

### DocumentEmbedding

```typescript
interface DocumentEmbedding {
  id: string;              // UUID
  document_id: string;     // UUID
  embedding: number[];     // Vector (1536 dimensions for text-embedding-3-small)
  model: EmbeddingModel;   // Model used
  token_count: number | null;
  created_at: string;      // ISO 8601
  updated_at: string;      // ISO 8601
}
```

### ChunkEmbedding

```typescript
interface ChunkEmbedding {
  id: string;              // UUID
  document_id: string;     // UUID
  chunk_index: number;     // Position in document
  chunk_text: string;      // Original text
  chunk_start_offset: number;
  chunk_end_offset: number;
  embedding: number[];     // Vector
  model: EmbeddingModel;
  token_count: number | null;
  metadata: object | null; // Custom metadata
  created_at: string;      // ISO 8601
}
```

### SemanticSearchRequest

```typescript
interface SemanticSearchRequest {
  query: string;           // Search query
  limit?: number;          // Max results (default: 10, max: 50)
  threshold?: number;      // Min similarity (default: 0.7)
  model?: EmbeddingModel;  // Model for query embedding
  user_id?: number;        // Filter by user
}
```

### SemanticSearchResult

```typescript
interface SemanticSearchResult {
  document_id: string;     // UUID
  chunk_id: string | null; // UUID if chunk match
  chunk_text: string | null;
  similarity: number;      // 0.0 - 1.0
  document_title: string | null;
}
```

### EmbeddingStats

```typescript
interface EmbeddingStats {
  total_documents: number;
  documents_with_embeddings: number;
  total_chunks: number;
  pending_queue: number;
  failed_queue: number;
}
```

### EmbeddingQueueEntry

```typescript
interface EmbeddingQueueEntry {
  id: string;              // UUID
  document_id: string;     // UUID
  status: string;          // pending | processing | completed | failed
  priority: number;
  attempts: number;
  max_attempts: number;
  error_message: string | null;
  created_at: string;      // ISO 8601
  started_at: string | null;
  completed_at: string | null;
}
```

---

## Embedding Models

| Model | Dimensions | Provider | Best For |
|-------|------------|----------|----------|
| `openai_text_embedding_3_small` | 1536 | OpenAI | General purpose, cost-effective |
| `openai_text_embedding_3_large` | 3072 | OpenAI | Higher accuracy, larger contexts |
| `voyage_large_2` | 1536 | Voyage AI | General purpose alternative |
| `voyage_code_2` | 1536 | Voyage AI | Code-focused embeddings |

### Default Configuration

```rust
EmbeddingConfig {
    openai_api_key: Some(key),
    voyage_api_key: None,
    default_model: EmbeddingModel::OpenaiTextEmbedding3Small,
    chunk_size: 512,        // tokens per chunk
    chunk_overlap: 50,      // overlap between chunks
}
```

---

## Best Practices

### 1. Chunk Size

The default chunk size is 512 tokens with 50 token overlap. This provides:
- Good semantic granularity
- Efficient token usage
- Context preservation at chunk boundaries

### 2. When to Use Document vs Chunk Embeddings

| Use Case | Recommended |
|----------|-------------|
| Document similarity | Document embedding |
| Precise Q&A | Chunk embeddings |
| Topic clustering | Document embedding |
| RAG pipelines | Chunk embeddings |

### 3. Similarity Thresholds

| Threshold | Meaning |
|-----------|---------|
| > 0.9 | Very high relevance |
| 0.8 - 0.9 | High relevance |
| 0.7 - 0.8 | Moderate relevance |
| < 0.7 | Low relevance |

### 4. Batch Processing

For large document imports, use the queue endpoint:

```bash
# Queue multiple documents with different priorities
for id in $DOC_IDS; do
  curl -X POST http://localhost:3000/api/embeddings/queue/$id \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -d '{"priority": 5}'
done
```

---

## Example: Complete Embedding Workflow

```bash
#!/bin/bash
TOKEN="your-auth-token"
BASE_URL="http://localhost:3000/api"

# 1. Create a document
DOC_RESPONSE=$(curl -s -X POST $BASE_URL/documents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "API Design Best Practices",
    "content": "# REST API Design\n\n## Naming Conventions\n\nUse plural nouns for resources...\n\n## HTTP Methods\n\nGET for retrieval, POST for creation..."
  }')

DOC_ID=$(echo $DOC_RESPONSE | jq -r '.data.id')
echo "Created document: $DOC_ID"

# 2. Generate document embedding
curl -X POST $BASE_URL/embeddings/document/$DOC_ID \
  -H "Authorization: Bearer $TOKEN"
echo "Generated document embedding"

# 3. Generate chunk embeddings (auto-chunking)
curl -X POST $BASE_URL/embeddings/chunks/$DOC_ID \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "document_id": "'$DOC_ID'",
    "chunks": [
      {"text": "REST API Design - Use plural nouns for resources. Keep URLs simple and intuitive.", "start_offset": 0, "end_offset": 100},
      {"text": "HTTP Methods - GET for retrieval, POST for creation, PUT for full updates, PATCH for partial updates.", "start_offset": 101, "end_offset": 200}
    ]
  }'
echo "Generated chunk embeddings"

# 4. Search for related content
curl -X POST $BASE_URL/embeddings/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "query": "What HTTP methods should I use for CRUD operations?",
    "limit": 5
  }'

# 5. Find similar documents
curl -X GET "$BASE_URL/embeddings/similar/$DOC_ID?limit=3" \
  -H "Authorization: Bearer $TOKEN"
```
