# Knowledge Graph API

Build and query the team knowledge graph derived from documents and AI analysis.

**Base URL:** `/api/knowledge`

---

## Authentication

All endpoints require a valid JWT Bearer token:

```
Authorization: Bearer <token>
```

---

## Endpoints

### GET /api/knowledge/graph

Build and return the full knowledge graph as nodes and edges.

**Query Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `threshold` | float | 0.5 | Minimum cosine similarity for document–document edges (0.0–1.0) |
| `max_edges` | integer | 5 | Maximum similar-document edges per node (max: 20) |
| `include_topics` | boolean | true | Include topic nodes extracted by AI analysis |
| `include_technologies` | boolean | true | Include technology nodes |
| `include_insights` | boolean | false | Include insight nodes |
| `max_documents` | integer | 100 | Maximum document nodes returned |

**Example:**

```
GET /api/knowledge/graph?threshold=0.6&max_edges=3&include_insights=true
```

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "id": "doc-550e8400-e29b-41d4-a716-446655440000",
        "label": "Setting Up pgvector in Rust",
        "node_type": "document",
        "document_count": 1,
        "summary": "Guide to installing and configuring pgvector with Axum",
        "metadata": {}
      },
      {
        "id": "topic-pgvector",
        "label": "pgvector",
        "node_type": "topic",
        "document_count": 3,
        "summary": null,
        "metadata": {}
      },
      {
        "id": "tech-Rust",
        "label": "Rust",
        "node_type": "technology",
        "document_count": 7,
        "summary": null,
        "metadata": {}
      }
    ],
    "edges": [
      {
        "source": "doc-550e8400-e29b-41d4-a716-446655440000",
        "target": "topic-pgvector",
        "weight": 1.0,
        "edge_type": "has_topic"
      },
      {
        "source": "doc-550e8400-e29b-41d4-a716-446655440000",
        "target": "doc-6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "weight": 0.73,
        "edge_type": "similar"
      }
    ]
  }
}
```

**Node Types:**

| Type | Description |
|------|-------------|
| `document` | A document stored in the knowledge base |
| `topic` | A key topic extracted by AI from documents |
| `technology` | A technology or tool mentioned in documents |
| `person` | A team member (author/contributor) |
| `insight` | A key insight extracted by AI |

**Edge Types:**

| Type | Description | Weight |
|------|-------------|--------|
| `similar` | Cosine similarity between document embeddings | similarity score (0–1) |
| `has_topic` | Document contains this topic | 1.0 |
| `uses_technology` | Document uses this technology | 1.0 |
| `has_insight` | Document contains this insight | 0.8 |
| `authored_by` | Document authored by a person | 1.0 |

---

### GET /api/knowledge/team

Return the team expertise map derived from document authorship and AI analysis.

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "members": [
      {
        "user_id": 1,
        "username": "alice",
        "expertise_areas": ["Rust", "PostgreSQL", "pgvector"],
        "document_count": 23,
        "expertise_level": "expert",
        "topics": ["database optimization", "vector search", "RAG"]
      },
      {
        "user_id": 2,
        "username": "bob",
        "expertise_areas": ["React", "TypeScript", "UI/UX"],
        "document_count": 8,
        "expertise_level": "intermediate",
        "topics": ["frontend architecture", "component design"]
      }
    ],
    "shared_areas": ["Rust", "API design", "documentation"],
    "unique_experts": [
      {
        "area": "pgvector",
        "expert_user_id": 1,
        "expert_username": "alice"
      }
    ]
  }
}
```

**ExpertiseLevel values:**

| Level | Document Count | Description |
|-------|---------------|-------------|
| `beginner` | 0–2 | Getting started |
| `intermediate` | 3–7 | Comfortable with the area |
| `advanced` | 8–15 | Deep knowledge |
| `expert` | 16+ | Domain authority |

**`unique_experts`:** Areas where only one team member has documented knowledge. Useful for identifying knowledge silos and bus-factor risks.

**`shared_areas`:** Topics that multiple team members have documented, indicating shared team knowledge.

---

## Knowledge Graph Data Model

### GraphNode

```json
{
  "id": "string (unique node identifier)",
  "label": "string (display name)",
  "node_type": "document | topic | technology | person | insight",
  "document_count": 0,
  "summary": "string | null",
  "metadata": {}
}
```

Node ID format by type:

| Type | ID Format |
|------|-----------|
| document | `doc-<uuid>` |
| topic | `topic-<normalized_label>` |
| technology | `tech-<normalized_label>` |
| person | `person-<user_id>` |
| insight | `insight-<normalized_label>` |

### GraphEdge

```json
{
  "source": "string (source node ID)",
  "target": "string (target node ID)",
  "weight": 0.0,
  "edge_type": "similar | has_topic | uses_technology | has_insight | authored_by"
}
```

Weight range: 0.0–1.0. Higher weight = stronger relationship.

### KnowledgeGraph

```json
{
  "nodes": "GraphNode[]",
  "edges": "GraphEdge[]"
}
```

---

## How the Graph Is Built

1. **Document nodes:** One node per document in the knowledge base.
2. **Derived nodes (topics/technologies/insights):** Extracted from `document_understandings` table (AI analysis results). Nodes are deduplicated — if 5 documents share the topic "Rust", there is a single `topic-Rust` node with `document_count: 5`.
3. **Similarity edges:** Documents with pgvector cosine similarity above `threshold` are connected. At most `max_edges` edges per document node.
4. **Derived edges:** Each document–topic/technology/insight relationship creates an edge.

**Example query (pgvector cosine distance):**

```sql
SELECT d2.id, 1 - (e1.embedding <=> e2.embedding) AS similarity
FROM document_embeddings e1
CROSS JOIN LATERAL (
    SELECT de.document_id, de.embedding
    FROM document_embeddings de
    WHERE de.document_id != e1.document_id
    ORDER BY e1.embedding <=> de.embedding
    LIMIT $max_edges
) e2
JOIN documents d2 ON d2.id = e2.document_id
WHERE 1 - (e1.embedding <=> e2.embedding) >= $threshold
```

---

## Frontend Integration

The knowledge graph API is consumed by the React `KnowledgeGraphPage` at `/graph`.

The component uses the standard `{nodes, edges}` response shape to render an SVG force-directed graph using the Fruchterman-Reingold layout algorithm.

**Example fetch:**

```javascript
const response = await fetch('/api/knowledge/graph?threshold=0.5&max_edges=5', {
  headers: { Authorization: `Bearer ${token}` }
})
const { data } = await response.json()
// data.nodes, data.edges
```

---

## Team Expertise Integration

The team expertise endpoint supports identifying knowledge silos:

```javascript
const { data } = await fetch('/api/knowledge/team', {
  headers: { Authorization: `Bearer ${token}` }
}).then(r => r.json())

// Find bus-factor risks
const singleExperts = data.unique_experts
console.log('Knowledge silos:', singleExperts)

// Find cross-functional skills
console.log('Shared knowledge:', data.shared_areas)
```

---

## Error Response Format

```json
{
  "success": false,
  "error": "Human-readable error message"
}
```

| HTTP Status | Meaning |
|-------------|---------|
| 400 | Invalid query parameter |
| 401 | Missing or invalid JWT token |
| 500 | Internal server error (DB query failed) |
