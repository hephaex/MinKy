# Document Understanding API

## Overview

The Document Understanding API provides AI-powered document analysis using Claude. It extracts key insights, topics, technologies, and potential audience from documents, transforming raw content into structured knowledge metadata.

## Table of Contents

- [Analyze Document](#analyze-document)
- [Get Understanding](#get-understanding)
- [Data Models](#data-models)
- [Analysis Fields](#analysis-fields)
- [Integration Guide](#integration-guide)

---

## Analyze Document

Trigger AI analysis for a document. This calls the Claude API to extract structured insights and persists the results for future retrieval.

### Endpoint

```
POST /api/documents/{id}/understand
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID to analyze |

### Example Request

```bash
curl -X POST http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440001/understand \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "document_id": "550e8400-e29b-41d4-a716-446655440001",
  "data": {
    "topics": [
      "Kubernetes deployment",
      "Container orchestration",
      "CI/CD pipelines",
      "Infrastructure as Code"
    ],
    "summary": "A comprehensive guide to deploying applications on Kubernetes clusters, covering deployment manifests, service configuration, and best practices for production environments.",
    "problem_solved": "How to reliably deploy and scale containerized applications in production",
    "insights": [
      "Use rolling deployments for zero-downtime updates",
      "Configure resource limits to prevent node exhaustion",
      "Implement health checks for automatic pod recovery",
      "Use ConfigMaps for environment-specific configuration"
    ],
    "technologies": [
      "Kubernetes",
      "Docker",
      "kubectl",
      "Helm",
      "YAML"
    ],
    "relevant_for": [
      "DevOps Engineers",
      "Backend Developers",
      "Platform Engineers",
      "SRE Teams"
    ]
  }
}
```

### How It Works

1. **Content Retrieval** - Fetches the document content from the database
2. **AI Analysis** - Sends content to Claude with a structured prompt
3. **Insight Extraction** - Claude extracts topics, summary, insights, etc.
4. **Persistence** - Results are saved to the `document_understanding` table
5. **Response** - Returns the analysis results

### Processing Time

Analysis typically takes 2-5 seconds depending on document length. For batch processing, consider using asynchronous patterns.

### Error Responses

| Status | Description |
|--------|-------------|
| 404 | Document not found |
| 500 | AI analysis failed |
| 502 | Claude API unavailable |

---

## Get Understanding

Retrieve previously computed document understanding from the database.

### Endpoint

```
GET /api/documents/{id}/understanding
```

### Path Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| id | UUID | Document ID |

### Example Request

```bash
curl -X GET http://localhost:3000/api/documents/550e8400-e29b-41d4-a716-446655440001/understanding \
  -H "Authorization: Bearer $TOKEN"
```

### Success Response (200 OK)

```json
{
  "success": true,
  "document_id": "550e8400-e29b-41d4-a716-446655440001",
  "data": {
    "id": "990e8400-e29b-41d4-a716-446655440001",
    "document_id": "550e8400-e29b-41d4-a716-446655440001",
    "topics": [
      "Kubernetes deployment",
      "Container orchestration",
      "CI/CD pipelines",
      "Infrastructure as Code"
    ],
    "summary": "A comprehensive guide to deploying applications on Kubernetes clusters...",
    "problem_solved": "How to reliably deploy and scale containerized applications in production",
    "insights": [
      "Use rolling deployments for zero-downtime updates",
      "Configure resource limits to prevent node exhaustion",
      "Implement health checks for automatic pod recovery",
      "Use ConfigMaps for environment-specific configuration"
    ],
    "technologies": [
      "Kubernetes",
      "Docker",
      "kubectl",
      "Helm",
      "YAML"
    ],
    "relevant_for": [
      "DevOps Engineers",
      "Backend Developers",
      "Platform Engineers",
      "SRE Teams"
    ],
    "related_document_ids": [
      "550e8400-e29b-41d4-a716-446655440005",
      "550e8400-e29b-41d4-a716-446655440008"
    ],
    "analyzed_at": "2024-01-20T10:30:00Z",
    "analyzer_model": "claude-sonnet-4-20250514"
  }
}
```

### Error Responses

| Status | Description |
|--------|-------------|
| 404 | Document has not been analyzed yet. POST to /understand first. |
| 500 | Database error |

---

## Data Models

### DocumentUnderstanding (Database Record)

```typescript
interface DocumentUnderstanding {
  id: string;                       // UUID
  document_id: string;              // UUID
  topics: string[];                 // 3-5 key topics
  summary: string | null;           // One-line summary
  problem_solved: string | null;    // Problem this solves
  insights: string[];               // Key insights/takeaways
  technologies: string[];           // Related technologies
  relevant_for: string[];           // Target roles/audiences
  related_document_ids: string[];   // UUIDs of similar documents
  analyzed_at: string;              // ISO 8601 timestamp
  analyzer_model: string | null;    // Claude model used
}
```

### DocumentUnderstandingResponse (API Response)

```typescript
interface DocumentUnderstandingResponse {
  topics: string[];
  summary: string;
  problem_solved: string | null;
  insights: string[];
  technologies: string[];
  relevant_for: string[];
}
```

### DocumentUnderstandingRequest (Internal)

```typescript
interface DocumentUnderstandingRequest {
  document_id: string;    // UUID
  content: string;        // Document content
  title: string | null;   // Document title
}
```

---

## Analysis Fields

### Topics (3-5 items)

High-level subjects covered in the document.

**Examples:**
- "Authentication flow design"
- "RESTful API best practices"
- "Database optimization techniques"

### Summary

A concise one-sentence description of the document's content and purpose.

**Example:**
> "A comprehensive guide to implementing JWT-based authentication in Node.js applications with refresh token rotation."

### Problem Solved

The specific problem or challenge that the document addresses.

**Example:**
> "How to securely manage user sessions across multiple microservices without sharing database state."

### Insights (Key Takeaways)

Actionable learnings and best practices extracted from the content.

**Examples:**
- "Store refresh tokens in HttpOnly cookies to prevent XSS attacks"
- "Implement token blacklisting for immediate session invalidation"
- "Use short-lived access tokens (15 min) with automatic refresh"

### Technologies

Tools, frameworks, languages, and platforms mentioned or relevant to the document.

**Examples:**
- "Node.js", "Express", "jsonwebtoken", "Redis", "PostgreSQL"

### Relevant For

Roles and team members who would benefit from this document.

**Examples:**
- "Backend Developers"
- "Security Engineers"
- "Full-stack Developers"
- "DevOps Engineers"

---

## Integration Guide

### 1. Automatic Analysis on Document Creation

```javascript
// After creating a document, trigger analysis
async function createDocumentWithAnalysis(documentData) {
  // Create document
  const createRes = await fetch('/api/documents', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    },
    body: JSON.stringify(documentData)
  });
  const { data: document } = await createRes.json();

  // Trigger analysis (async - don't await in production)
  fetch(`/api/documents/${document.id}/understand`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}` }
  });

  // Trigger embedding generation
  fetch(`/api/embeddings/document/${document.id}`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${token}` }
  });

  return document;
}
```

### 2. Display Understanding in UI

```jsx
function DocumentInsights({ documentId }) {
  const [understanding, setUnderstanding] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchUnderstanding() {
      try {
        const res = await fetch(`/api/documents/${documentId}/understanding`, {
          headers: { 'Authorization': `Bearer ${token}` }
        });
        if (res.ok) {
          const { data } = await res.json();
          setUnderstanding(data);
        }
      } catch (error) {
        console.error('Failed to fetch understanding:', error);
      } finally {
        setLoading(false);
      }
    }
    fetchUnderstanding();
  }, [documentId]);

  if (loading) return <Spinner />;
  if (!understanding) return <AnalyzeButton documentId={documentId} />;

  return (
    <div className="insights-panel">
      <section>
        <h3>Summary</h3>
        <p>{understanding.summary}</p>
      </section>

      <section>
        <h3>Key Topics</h3>
        <div className="tags">
          {understanding.topics.map(topic => (
            <span key={topic} className="tag">{topic}</span>
          ))}
        </div>
      </section>

      <section>
        <h3>Technologies</h3>
        <div className="tags">
          {understanding.technologies.map(tech => (
            <span key={tech} className="tag tech">{tech}</span>
          ))}
        </div>
      </section>

      <section>
        <h3>Key Insights</h3>
        <ul>
          {understanding.insights.map((insight, i) => (
            <li key={i}>{insight}</li>
          ))}
        </ul>
      </section>

      <section>
        <h3>Relevant For</h3>
        <div className="roles">
          {understanding.relevant_for.map(role => (
            <span key={role} className="role">{role}</span>
          ))}
        </div>
      </section>
    </div>
  );
}
```

### 3. Batch Analysis for Existing Documents

```bash
#!/bin/bash
# Analyze all documents without understanding

TOKEN="your-auth-token"
BASE_URL="http://localhost:3000/api"

# Get all document IDs
DOCS=$(curl -s "$BASE_URL/documents?limit=1000" \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[].id')

# Analyze each document
for DOC_ID in $DOCS; do
  # Check if already analyzed
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
    "$BASE_URL/documents/$DOC_ID/understanding" \
    -H "Authorization: Bearer $TOKEN")

  if [ "$STATUS" = "404" ]; then
    echo "Analyzing document: $DOC_ID"
    curl -X POST "$BASE_URL/documents/$DOC_ID/understand" \
      -H "Authorization: Bearer $TOKEN"
    # Rate limit to avoid API throttling
    sleep 2
  else
    echo "Already analyzed: $DOC_ID"
  fi
done
```

### 4. Using Understanding for Search Enhancement

Document understanding can enhance search relevance:

```javascript
// Search with understanding-based filtering
async function enhancedSearch(query, targetRole) {
  // Get semantic search results
  const searchRes = await fetch('/api/embeddings/search', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${token}`
    },
    body: JSON.stringify({ query, limit: 50 })
  });
  const { data: results } = await searchRes.json();

  // Fetch understanding for top results
  const enrichedResults = await Promise.all(
    results.slice(0, 20).map(async (result) => {
      const understandingRes = await fetch(
        `/api/documents/${result.document_id}/understanding`,
        { headers: { 'Authorization': `Bearer ${token}` } }
      );
      if (understandingRes.ok) {
        const { data: understanding } = await understandingRes.json();
        return { ...result, understanding };
      }
      return result;
    })
  );

  // Filter by target role if specified
  if (targetRole) {
    return enrichedResults.filter(r =>
      r.understanding?.relevant_for?.includes(targetRole)
    );
  }

  return enrichedResults;
}
```

---

## Best Practices

### 1. When to Re-analyze

Re-trigger analysis when:
- Document content is significantly updated
- You want to use a newer AI model
- Understanding data seems outdated

```bash
# Force re-analysis
curl -X POST http://localhost:3000/api/documents/{id}/understand \
  -H "Authorization: Bearer $TOKEN"
```

### 2. Handling Analysis Failures

```javascript
async function analyzeWithRetry(documentId, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const res = await fetch(`/api/documents/${documentId}/understand`, {
        method: 'POST',
        headers: { 'Authorization': `Bearer ${token}` }
      });
      if (res.ok) return await res.json();
      if (res.status !== 502) throw new Error('Non-retryable error');
    } catch (error) {
      console.error(`Attempt ${i + 1} failed:`, error);
      await new Promise(r => setTimeout(r, 1000 * (i + 1)));
    }
  }
  throw new Error('Analysis failed after retries');
}
```

### 3. Caching Strategy

Understanding results are cached in the database. The GET endpoint returns cached results instantly. Only call POST when:
- First-time analysis needed
- Content has changed
- Manual refresh requested

### 4. Cost Considerations

Each analysis makes an API call to Claude. For cost optimization:
- Analyze documents selectively (important/high-value content)
- Use batch processing during off-peak hours
- Consider implementing analysis quotas per user/team
