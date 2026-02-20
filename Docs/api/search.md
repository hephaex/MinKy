# Search API

## Overview

The Search API provides multiple search capabilities for MinKy documents:

1. **Full-text Search** - Traditional keyword-based search with filtering and facets
2. **Semantic Search** - AI-powered search using vector embeddings
3. **Autocomplete** - Real-time search suggestions

## Table of Contents

- [Full-text Search](#full-text-search)
- [Semantic Search](#semantic-search)
- [Autocomplete](#autocomplete)
- [Reindex All](#reindex-all)
- [Data Models](#data-models)
- [Search Strategies](#search-strategies)

---

## Full-text Search

Perform keyword-based search with filtering, sorting, and facets.

### Endpoint

```
GET /api/search
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| q | string | - | Search query (required) |
| page | integer | 1 | Page number |
| limit | integer | 10 | Results per page |
| category_id | integer | - | Filter by category |
| tags | array | - | Filter by tags |
| date_from | datetime | - | Filter by start date |
| date_to | datetime | - | Filter by end date |
| sort_by | string | relevance | Sort field (relevance, created_at, updated_at, title, view_count) |
| sort_order | string | desc | Sort order (asc, desc) |

### Example Request

```bash
curl -X GET "http://localhost:3000/api/search?q=kubernetes+deployment&limit=10&sort_by=relevance" \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": {
    "hits": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "title": "Kubernetes Deployment Guide",
        "content_snippet": "...how to deploy applications to <em>Kubernetes</em> clusters using <em>deployment</em> manifests...",
        "highlights": [
          "how to deploy applications to <em>Kubernetes</em> clusters",
          "using <em>deployment</em> manifests"
        ],
        "score": 0.95,
        "category_name": "DevOps",
        "tags": ["kubernetes", "deployment", "containers"],
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-16T14:20:00Z"
      },
      {
        "id": "550e8400-e29b-41d4-a716-446655440002",
        "title": "Container Orchestration Basics",
        "content_snippet": "...<em>Kubernetes</em> is the most popular container orchestration platform...",
        "highlights": [
          "<em>Kubernetes</em> is the most popular"
        ],
        "score": 0.78,
        "category_name": "DevOps",
        "tags": ["kubernetes", "docker", "containers"],
        "created_at": "2024-01-10T08:00:00Z",
        "updated_at": "2024-01-10T08:00:00Z"
      }
    ],
    "total": 25,
    "page": 1,
    "limit": 10,
    "took_ms": 45,
    "facets": {
      "categories": [
        {"value": "DevOps", "count": 15},
        {"value": "Backend", "count": 8},
        {"value": "Infrastructure", "count": 2}
      ],
      "tags": [
        {"value": "kubernetes", "count": 20},
        {"value": "deployment", "count": 12},
        {"value": "docker", "count": 10}
      ],
      "date_ranges": [
        {"label": "Last 7 days", "from": "2024-01-13T00:00:00Z", "to": "2024-01-20T00:00:00Z", "count": 5},
        {"label": "Last 30 days", "from": "2023-12-21T00:00:00Z", "to": "2024-01-20T00:00:00Z", "count": 18}
      ]
    }
  }
}
```

### Sort Fields

| Field | Description |
|-------|-------------|
| relevance | Search relevance score (default) |
| created_at | Document creation date |
| updated_at | Last update date |
| title | Alphabetical by title |
| view_count | Most viewed |

---

## Semantic Search

Perform AI-powered semantic search using vector embeddings. This finds conceptually similar content even without exact keyword matches.

### Endpoint

```
POST /api/search/semantic
```

### Request Body

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| query | string | - | Natural language query (required) |
| limit | integer | 10 | Maximum results (max: 50) |

### Example Request

```bash
curl -X POST http://localhost:3000/api/search/semantic \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "query": "How do I handle errors in asynchronous JavaScript code?",
    "limit": 5
  }'
```

### Success Response (200 OK)

```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440010",
      "title": "Promise Error Handling Patterns",
      "content_snippet": "Using try-catch with async/await provides clean error handling...",
      "highlights": [],
      "score": 0.91,
      "category_name": "JavaScript",
      "tags": ["javascript", "async", "promises"],
      "created_at": "2024-01-12T09:00:00Z",
      "updated_at": "2024-01-12T09:00:00Z"
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655440011",
      "title": "Error Boundaries in React",
      "content_snippet": "Error boundaries catch errors in the component tree...",
      "highlights": [],
      "score": 0.84,
      "category_name": "React",
      "tags": ["react", "error-handling"],
      "created_at": "2024-01-08T11:30:00Z",
      "updated_at": "2024-01-08T11:30:00Z"
    }
  ]
}
```

### How It Works

1. The query text is converted to a vector embedding using OpenAI's text-embedding-3-small
2. The embedding is compared against document embeddings using cosine similarity
3. Results are ranked by similarity score (0.0 to 1.0)

---

## Autocomplete

Get real-time search suggestions as users type.

### Endpoint

```
GET /api/search/autocomplete
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| q | string | - | Partial search query (required) |
| limit | integer | 10 | Maximum suggestions (max: 20) |

### Example Request

```bash
curl -X GET "http://localhost:3000/api/search/autocomplete?q=kube&limit=5" \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "suggestions": [
    {
      "text": "kubernetes",
      "score": 0.95,
      "document_count": 20
    },
    {
      "text": "kubectl commands",
      "score": 0.87,
      "document_count": 8
    },
    {
      "text": "kubernetes deployment",
      "score": 0.82,
      "document_count": 15
    },
    {
      "text": "kubernetes services",
      "score": 0.78,
      "document_count": 12
    },
    {
      "text": "kubernetes networking",
      "score": 0.72,
      "document_count": 6
    }
  ]
}
```

---

## Reindex All

Rebuild the search index for all documents. Admin-only operation.

### Endpoint

```
POST /api/search/reindex
```

### Example Request

```bash
curl -X POST http://localhost:3000/api/search/reindex \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "message": "Reindexing completed",
  "documents_indexed": 150
}
```

### Note

This operation may take significant time for large document collections. Consider running during off-peak hours.

---

## Data Models

### SearchQuery

```typescript
interface SearchQuery {
  q: string;                    // Search query
  page?: number;                // Page number (default: 1)
  limit?: number;               // Results per page (default: 10)
  category_id?: number;         // Filter by category
  tags?: string[];              // Filter by tags
  date_from?: string;           // ISO 8601 date
  date_to?: string;             // ISO 8601 date
  sort_by?: SortField;          // Sort field
  sort_order?: SortOrder;       // asc or desc
}
```

### SearchHit

```typescript
interface SearchHit {
  id: string;                   // UUID
  title: string;
  content_snippet: string;      // Relevant excerpt
  highlights: string[];         // Highlighted matches
  score: number;                // Relevance score
  category_name: string | null;
  tags: string[];
  created_at: string;           // ISO 8601
  updated_at: string;           // ISO 8601
}
```

### SearchResponse

```typescript
interface SearchResponse {
  hits: SearchHit[];
  total: number;
  page: number;
  limit: number;
  took_ms: number;              // Query execution time
  facets: SearchFacets;
}
```

### SearchFacets

```typescript
interface SearchFacets {
  categories: FacetCount[];
  tags: FacetCount[];
  date_ranges: DateRangeFacet[];
}

interface FacetCount {
  value: string;
  count: number;
}

interface DateRangeFacet {
  label: string;
  from: string;                 // ISO 8601
  to: string;                   // ISO 8601
  count: number;
}
```

### AutocompleteSuggestion

```typescript
interface AutocompleteSuggestion {
  text: string;                 // Suggested text
  score: number;                // Relevance score
  document_count: number;       // Number of matching docs
}
```

---

## Search Strategies

### When to Use Full-text vs Semantic Search

| Use Case | Recommended | Reason |
|----------|-------------|--------|
| Exact term lookup | Full-text | Precise matches |
| Natural language questions | Semantic | Conceptual understanding |
| Known keywords | Full-text | Faster, more predictable |
| Exploratory search | Semantic | Finds related content |
| Code snippets | Full-text | Exact syntax matching |
| Troubleshooting | Semantic | Problem-solution matching |

### Combining Both Searches

For best results, consider implementing hybrid search:

```javascript
// Pseudo-code for hybrid search
async function hybridSearch(query) {
  const [fulltextResults, semanticResults] = await Promise.all([
    fetch(`/api/search?q=${query}`),
    fetch('/api/search/semantic', {
      method: 'POST',
      body: JSON.stringify({ query })
    })
  ]);

  // Merge and rank results
  return mergeResults(fulltextResults, semanticResults);
}
```

### Search Tips

1. **Use Filters** - Narrow results with category and tag filters
2. **Leverage Facets** - Use returned facets to refine searches
3. **Autocomplete First** - Guide users with autocomplete suggestions
4. **Semantic for Q&A** - Use semantic search for question-answering
5. **Full-text for Browsing** - Use full-text for category exploration

---

## Example: Building a Search UI

```javascript
// React example for search with autocomplete
import { useState, useEffect } from 'react';
import debounce from 'lodash/debounce';

function SearchComponent() {
  const [query, setQuery] = useState('');
  const [suggestions, setSuggestions] = useState([]);
  const [results, setResults] = useState([]);

  // Autocomplete on keystroke (debounced)
  const fetchSuggestions = debounce(async (q) => {
    if (q.length < 2) return;
    const res = await fetch(`/api/search/autocomplete?q=${q}&limit=5`);
    const data = await res.json();
    setSuggestions(data.suggestions);
  }, 200);

  useEffect(() => {
    fetchSuggestions(query);
  }, [query]);

  // Full search on submit
  const handleSearch = async (e) => {
    e.preventDefault();
    const res = await fetch(`/api/search?q=${query}&limit=20`);
    const data = await res.json();
    setResults(data.data.hits);
  };

  // Semantic search button
  const handleSemanticSearch = async () => {
    const res = await fetch('/api/search/semantic', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query, limit: 20 })
    });
    const data = await res.json();
    setResults(data.data);
  };

  return (
    <form onSubmit={handleSearch}>
      <input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search documents..."
      />
      <button type="submit">Search</button>
      <button type="button" onClick={handleSemanticSearch}>
        AI Search
      </button>

      {/* Autocomplete dropdown */}
      {suggestions.length > 0 && (
        <ul className="suggestions">
          {suggestions.map(s => (
            <li key={s.text} onClick={() => setQuery(s.text)}>
              {s.text} ({s.document_count})
            </li>
          ))}
        </ul>
      )}

      {/* Results */}
      <div className="results">
        {results.map(hit => (
          <div key={hit.id} className="result">
            <h3>{hit.title}</h3>
            <p dangerouslySetInnerHTML={{ __html: hit.content_snippet }} />
            <span className="score">Score: {hit.score.toFixed(2)}</span>
          </div>
        ))}
      </div>
    </form>
  );
}
```
