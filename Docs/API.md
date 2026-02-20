# MinKy API Documentation

## Overview

MinKy is a Team Knowledge Intelligence Platform that provides AI-powered document understanding, semantic search, and RAG (Retrieval-Augmented Generation) capabilities for small teams. The API is built with Rust (Axum framework) and provides comprehensive endpoints for document management, AI analysis, and knowledge retrieval.

## Table of Contents

1. [Base URL](#base-url)
2. [Authentication](#authentication)
3. [Response Format](#response-format)
4. [Error Handling](#error-handling)
5. [Rate Limiting](#rate-limiting)
6. [API Endpoints](#api-endpoints)
   - [Health Check](#health-check)
   - [Authentication](#authentication-endpoints)
   - [Documents](#documents)
   - [Embeddings](#embeddings)
   - [Search](#search)
   - [Document Understanding](#document-understanding)
   - [AI Services](#ai-services)
   - [Tags](#tags)
   - [Categories](#categories)
   - [Notifications](#notifications)
   - [Analytics](#analytics)
   - [Admin](#admin)
   - [Export/Import](#exportimport)
   - [Agents](#agents)
   - [Skills](#skills)

## Base URL

| Environment | URL |
|-------------|-----|
| Development | `http://localhost:3000/api` |
| Production | `https://your-domain.com/api` |

## Authentication

The API uses JWT (JSON Web Tokens) for authentication. Include the token in the Authorization header:

```
Authorization: Bearer <your-jwt-token>
```

## Response Format

All API responses follow a consistent JSON structure:

### Success Response
```json
{
  "success": true,
  "data": { ... }
}
```

### List Response with Pagination
```json
{
  "success": true,
  "data": [ ... ],
  "meta": {
    "total": 100,
    "page": 1,
    "limit": 20,
    "total_pages": 5
  }
}
```

### Error Response
```json
{
  "success": false,
  "error": "Error message description"
}
```

## Error Handling

The API uses standard HTTP status codes:

| Status Code | Description |
|-------------|-------------|
| 200 | Success |
| 201 | Created |
| 204 | No Content (successful delete) |
| 400 | Bad Request - Validation error |
| 401 | Unauthorized - Authentication required |
| 403 | Forbidden - Access denied |
| 404 | Not Found - Resource does not exist |
| 409 | Conflict - Resource already exists |
| 429 | Too Many Requests - Rate limit exceeded |
| 500 | Internal Server Error |
| 502 | Bad Gateway - External service error |

## Rate Limiting

API endpoints may be rate limited. When rate limits are exceeded, you will receive a `429 Too Many Requests` response with headers indicating the limit and reset time.

---

## API Endpoints

### Health Check

#### GET /health

Basic health check endpoint for monitoring.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "database": "healthy"
}
```

---

### Authentication Endpoints

See [api/authentication.md](./api/authentication.md) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/auth/login` | User login |
| POST | `/auth/register` | User registration |
| POST | `/auth/refresh` | Refresh access token |

---

### Documents

See [api/documents.md](./api/documents.md) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/documents` | List documents |
| POST | `/documents` | Create document |
| GET | `/documents/{id}` | Get document |
| PUT | `/documents/{id}` | Update document |
| DELETE | `/documents/{id}` | Delete document |

---

### Embeddings

See [api/embeddings.md](./api/embeddings.md) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/embeddings/document/{id}` | Generate document embedding |
| GET | `/embeddings/document/{id}` | Get document embedding |
| POST | `/embeddings/chunks/{id}` | Generate chunk embeddings |
| POST | `/embeddings/search` | Semantic search |
| GET | `/embeddings/similar/{id}` | Find similar documents |
| GET | `/embeddings/stats` | Get embedding statistics |
| POST | `/embeddings/queue/{id}` | Queue document for embedding |

---

### Search

See [api/search.md](./api/search.md) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/search` | Full-text search |
| POST | `/search/semantic` | AI-powered semantic search |
| GET | `/search/autocomplete` | Search suggestions |
| POST | `/search/reindex` | Rebuild search index (admin) |

---

### Document Understanding

See [api/understanding.md](./api/understanding.md) for detailed documentation.

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/documents/{id}/understand` | Trigger AI analysis |
| GET | `/documents/{id}/understanding` | Get cached analysis |

---

### AI Services

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/ai/suggest` | Generate AI suggestion |
| POST | `/ai/suggest/title` | Suggest document title |
| POST | `/ai/suggest/summary` | Generate summary |
| POST | `/ai/suggest/tags` | Suggest tags |
| POST | `/ai/improve` | Improve text |
| POST | `/ai/embedding` | Generate text embedding |

---

### Tags

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/tags` | List all tags |
| POST | `/tags` | Create tag |
| GET | `/tags/{id}` | Get tag |
| PUT | `/tags/{id}` | Update tag |
| DELETE | `/tags/{id}` | Delete tag |

---

### Categories

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/categories` | List categories |
| POST | `/categories` | Create category |
| GET | `/categories/tree` | Get category tree |
| GET | `/categories/{id}` | Get category |
| PUT | `/categories/{id}` | Update category |
| DELETE | `/categories/{id}` | Delete category |

---

### Notifications

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/notifications` | List notifications |
| GET | `/notifications/count` | Get unread count |
| PUT | `/notifications/{id}/read` | Mark as read |
| POST | `/notifications/read-all` | Mark all as read |
| DELETE | `/notifications/{id}` | Delete notification |

---

### Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/analytics/overview` | Get analytics overview |
| GET | `/analytics/dashboard` | Get dashboard data |
| GET | `/analytics/documents/top` | Get top documents |
| GET | `/analytics/documents/{id}/content` | Analyze document content |
| GET | `/analytics/users/active` | Get active users |
| GET | `/analytics/categories` | Get category stats |
| GET | `/analytics/tags` | Get tag stats |
| GET | `/analytics/timeline` | Get activity timeline |
| GET | `/analytics/workflows` | Get workflow analytics |

---

### Admin

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/admin/stats` | Get system stats |
| GET | `/admin/users` | List all users |
| GET | `/admin/users/{id}` | Get user details |
| PUT | `/admin/users/{id}` | Update user |
| DELETE | `/admin/users/{id}` | Delete user |
| GET | `/admin/audit-logs` | Get audit logs |
| GET | `/admin/backups` | List backups |
| POST | `/admin/backups` | Create backup |
| GET | `/admin/config` | Get system config |
| PUT | `/admin/config` | Update system config |
| GET | `/admin/maintenance` | Get maintenance mode |
| PUT | `/admin/maintenance` | Set maintenance mode |

---

### Export/Import

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/export` | Start export job |
| GET | `/export/download` | Download export |
| GET | `/export/status/{id}` | Get export status |
| POST | `/export/import` | Import documents |

---

### Agents

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/agents` | List agents |
| POST | `/agents` | Create agent |
| GET | `/agents/{id}` | Get agent |
| PUT | `/agents/{id}` | Update agent |
| DELETE | `/agents/{id}` | Delete agent |
| POST | `/agents/{id}/execute` | Execute agent |
| GET | `/agents/tasks` | List agent tasks |

---

### Skills

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/skills` | Get skill registry |
| POST | `/skills` | Create custom skill |
| GET | `/skills/{id}` | Get skill |
| PUT | `/skills/{id}` | Update skill |
| DELETE | `/skills/{id}` | Delete skill |
| GET | `/skills/type/{type}` | Get skill by type |
| POST | `/skills/execute` | Execute skill |
| POST | `/skills/execute/{type}` | Execute skill by type |
| POST | `/skills/review` | Quick code review |
| POST | `/skills/debug` | Quick debug |
| POST | `/skills/refactor` | Quick refactor |
| POST | `/skills/test` | Generate tests |
| POST | `/skills/security` | Security review |
| POST | `/skills/plan` | Plan feature |
| GET | `/skills/stats` | Get skill stats |
| GET | `/skills/history` | Get execution history |

---

## Quick Start Examples

### 1. Create and Analyze a Document

```bash
# 1. Create a document
curl -X POST http://localhost:3000/api/documents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "title": "My Knowledge Document",
    "content": "This document contains important information about..."
  }'

# 2. Trigger AI understanding
curl -X POST http://localhost:3000/api/documents/{document_id}/understand \
  -H "Authorization: Bearer $TOKEN"

# 3. Generate embedding for semantic search
curl -X POST http://localhost:3000/api/embeddings/document/{document_id} \
  -H "Authorization: Bearer $TOKEN"
```

### 2. Semantic Search

```bash
curl -X POST http://localhost:3000/api/embeddings/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "query": "How do we handle authentication?",
    "limit": 10,
    "threshold": 0.7
  }'
```

### 3. AI-Powered Suggestions

```bash
# Generate title suggestion
curl -X POST http://localhost:3000/api/ai/suggest/title \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "content": "Your document content here..."
  }'

# Generate summary
curl -X POST http://localhost:3000/api/ai/suggest/summary \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "content": "Your document content here..."
  }'
```

---

## Additional Documentation

- [Authentication API](./api/authentication.md)
- [Documents API](./api/documents.md)
- [Embeddings API](./api/embeddings.md)
- [Search API](./api/search.md)
- [Understanding API](./api/understanding.md)
- [API Examples](./examples/api-examples.md)

---

## Changelog

### v0.1.0 (Current)
- Initial API implementation
- Document CRUD operations
- AI-powered document understanding
- Vector embeddings and semantic search
- Agents and Skills system
