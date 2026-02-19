# Slack / Teams Integration API

Knowledge extraction from Slack and Teams messaging platforms.

**Base URL:** `/api/slack`

---

## Authentication

All endpoints (except webhook and OAuth callback) require a valid JWT Bearer token:

```
Authorization: Bearer <token>
```

---

## Endpoints

### POST /api/slack/extract

Extract knowledge from a conversation thread using Claude AI.

**Request Body:**

```json
{
  "conversation_id": "C01ABC123/1700000000.000",
  "messages": [
    {
      "id": "1700000000.000",
      "platform": "slack",
      "workspace_id": "T01ABC",
      "channel_id": "C01ABC123",
      "channel_name": "engineering",
      "user_id": "U01XYZ",
      "username": "alice",
      "text": "We solved the pgvector performance issue by adding an HNSW index.",
      "thread_ts": "1700000000.000",
      "reply_count": 3,
      "reactions": [],
      "attachments": [],
      "posted_at": "2026-02-19T10:00:00Z",
      "captured_at": "2026-02-19T10:01:00Z"
    }
  ],
  "filter": {
    "platform": "slack",
    "channel_id": "C01ABC123",
    "since": "2026-02-01T00:00:00Z",
    "limit": 100
  }
}
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `conversation_id` | string | yes | Format: `<channel_id>/<thread_ts>` |
| `messages` | PlatformMessage[] | yes | Messages in the thread (root + replies) |
| `filter` | MessageFilter | no | Optional filter (default: no filter) |

**MessageFilter Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `platform` | string | Filter by platform (`slack`, `teams`, `discord`) |
| `channel_id` | string | Filter by channel |
| `user_id` | string | Filter by user |
| `since` | datetime | Only include messages after this time |
| `limit` | integer | Max messages to process (default: 50) |

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "conversation_id": "C01ABC123/1700000000.000",
    "status": "completed",
    "knowledge": {
      "title": "pgvector HNSW Index Performance Fix",
      "summary": "Adding an HNSW index to pgvector resolves performance issues for large datasets.",
      "topics": ["pgvector", "PostgreSQL", "performance"],
      "technologies": ["pgvector", "PostgreSQL"],
      "insights": ["HNSW index outperforms IVFFlat for recall at scale"],
      "confidence": 0.92,
      "is_high_quality": true,
      "source_messages": 4,
      "extracted_at": "2026-02-19T10:01:30Z"
    },
    "message": "Extraction completed. 4 messages analysed across 1 threads."
  }
}
```

**ExtractionStatus values:**

| Status | Description |
|--------|-------------|
| `pending` | Queued, not yet processed |
| `processing` | Currently being analysed |
| `completed` | Successfully extracted |
| `skipped` | Thread did not meet quality criteria (too short, low confidence) |
| `failed` | Extraction failed (API error, parse error) |
| `confirmed` | Human confirmed the extraction |
| `rejected` | Human rejected the extraction |

**Error Responses:**

| Status | Condition |
|--------|-----------|
| 400 Bad Request | Invalid request body or missing required fields |
| 502 Bad Gateway | Anthropic API unavailable |

---

### GET /api/slack/extract/{conversation_id}

Retrieve a previously stored extraction result by conversation ID.

**Path Parameter:** `conversation_id` (URL-encoded `<channel_id>/<thread_ts>`)

**Response (200 OK):** Same as POST `/extract` response.

**Response (404 Not Found):**
```json
{
  "success": false,
  "error": "No extraction found for conversation 'C01ABC123/1700000000.000'"
}
```

---

### POST /api/slack/confirm

Human-in-the-loop confirmation or rejection of extracted knowledge.

**Request Body:**

```json
{
  "extraction_id": "ext-001",
  "confirmed": true,
  "override_title": "Optional corrected title",
  "override_summary": "Optional corrected summary"
}
```

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "extraction_id": "ext-001",
    "action": "confirmed"
  }
}
```

---

### GET /api/slack/summary

Return aggregated extraction activity statistics.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `since` | datetime | Only include activity after this time (ISO 8601) |

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "total_conversations": 152,
    "total_messages": 3840,
    "knowledge_items_extracted": 89,
    "high_quality_items": 61,
    "pending_confirmation": 12,
    "last_extraction_at": "2026-02-19T09:55:00Z"
  }
}
```

---

### GET /api/slack/oauth/callback

Slack OAuth 2.0 redirect handler. Called by Slack after user approves the app.

**Query Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `code` | string | Temporary OAuth code from Slack |
| `state` | string | CSRF protection state |
| `error` | string | Set if user denied access |

**Response (200 OK):**

```json
{
  "success": true,
  "data": {
    "message": "Slack workspace connected successfully.",
    "workspace_id": "T01ABC",
    "workspace_name": "ACME Engineering",
    "platform_config_id": 1
  }
}
```

**Response (400 Bad Request):** Missing `code` parameter.

**Response (502 Bad Gateway):** Slack OAuth API error.

**Environment Variables Required:**

| Variable | Description |
|----------|-------------|
| `SLACK_CLIENT_ID` | Slack App client ID |
| `SLACK_CLIENT_SECRET` | Slack App client secret |
| `SLACK_REDIRECT_URI` | Registered OAuth redirect URI |
| `SLACK_SIGNING_SECRET` | For webhook signature verification |

---

### POST /api/slack/webhook

Receive Slack Events API payloads. Must be registered as the Events API endpoint in the Slack App configuration.

**Slack Endpoint Verification:**

When first registering the endpoint, Slack sends a `url_verification` challenge:

```json
{
  "type": "url_verification",
  "challenge": "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tYXAYM8P"
}
```

Response must echo the challenge:

```json
{
  "challenge": "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tYXAYM8P"
}
```

**Event Callback:**

Slack sends `event_callback` for subscribed events. For `message` and `app_mention` events, the handler automatically queues knowledge extraction:

```json
{
  "type": "event_callback",
  "team_id": "T01ABC",
  "event": {
    "type": "message",
    "channel": "C01ABC123",
    "user": "U01XYZ",
    "text": "Resolved: use HNSW index for pgvector",
    "ts": "1700000000.000",
    "thread_ts": "1699999000.000"
  }
}
```

**Response (200 OK):** Acknowledged within 3 seconds (Slack requirement).

```json
{ "ok": true, "queued": true }
```

For non-message events:

```json
{ "ok": true, "queued": false }
```

**Event Routing Logic:**

| Payload type | Inner event type | Action |
|--------------|-----------------|--------|
| `url_verification` | — | Echo challenge |
| `event_callback` | `message` | Queue extraction (background) |
| `event_callback` | `app_mention` | Queue extraction (background) |
| `event_callback` | other | Ignore |
| other | — | Acknowledge, log warning |

---

## Data Models

### PlatformMessage

```json
{
  "id": "1700000000.000",
  "platform": "slack",
  "workspace_id": "T01ABC",
  "channel_id": "C01ABC123",
  "channel_name": "engineering",
  "user_id": "U01XYZ",
  "username": "alice",
  "text": "Message content",
  "thread_ts": "1700000000.000",
  "reply_count": 0,
  "reactions": [
    { "name": "white_check_mark", "count": 3, "users": ["U001", "U002", "U003"] }
  ],
  "attachments": [],
  "posted_at": "2026-02-19T10:00:00Z",
  "captured_at": "2026-02-19T10:01:00Z"
}
```

### ExtractedKnowledge

```json
{
  "title": "Title of the extracted knowledge",
  "summary": "One paragraph summary",
  "topics": ["topic1", "topic2"],
  "technologies": ["Rust", "PostgreSQL"],
  "insights": ["Key insight 1"],
  "confidence": 0.87,
  "is_high_quality": true,
  "source_messages": 5,
  "extracted_at": "2026-02-19T10:01:30Z"
}
```

`is_high_quality` is `true` when `confidence >= 0.7` and both `title` and `summary` are non-empty.

---

## Database Schema

```sql
-- Workspace OAuth credentials
CREATE TABLE platform_configs (
    id SERIAL PRIMARY KEY,
    platform VARCHAR(50) NOT NULL,
    workspace_id VARCHAR(100) NOT NULL,
    workspace_name VARCHAR(255),
    bot_token TEXT,
    scope TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(platform, workspace_id)
);

-- Raw messages captured from platforms
CREATE TABLE platform_messages (
    id VARCHAR(100) PRIMARY KEY,
    platform VARCHAR(50) NOT NULL,
    workspace_id VARCHAR(100) NOT NULL,
    channel_id VARCHAR(100) NOT NULL,
    user_id VARCHAR(100) NOT NULL,
    text TEXT NOT NULL,
    thread_ts VARCHAR(50),
    posted_at TIMESTAMPTZ NOT NULL,
    captured_at TIMESTAMPTZ DEFAULT NOW()
);

-- Extraction job tracking
CREATE TABLE extraction_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    platform VARCHAR(50) NOT NULL,
    message_count INTEGER DEFAULT 0,
    confidence FLOAT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Extracted knowledge items
CREATE TABLE extracted_knowledge (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID REFERENCES extraction_jobs(id),
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    topics JSONB DEFAULT '[]',
    technologies JSONB DEFAULT '[]',
    insights JSONB DEFAULT '[]',
    confidence FLOAT NOT NULL DEFAULT 0,
    is_confirmed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## Error Response Format

All errors follow the standard format:

```json
{
  "success": false,
  "error": "Human-readable error message"
}
```

| HTTP Status | Meaning |
|-------------|---------|
| 400 | Validation error, missing required field |
| 401 | Missing or invalid JWT token |
| 403 | Insufficient permissions |
| 404 | Resource not found |
| 429 | Rate limit exceeded |
| 500 | Internal server error |
| 502 | External service (Slack, Anthropic) unavailable |
