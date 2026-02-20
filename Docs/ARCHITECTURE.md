# MinKy Architecture

This document provides a comprehensive overview of MinKy's system architecture, design decisions, and technical implementation details.

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Architecture Principles](#architecture-principles)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Technology Decisions](#technology-decisions)
6. [Database Schema](#database-schema)
7. [API Design](#api-design)
8. [Security Architecture](#security-architecture)
9. [Performance Considerations](#performance-considerations)
10. [Deployment Architecture](#deployment-architecture)

---

## System Overview

MinKy is a knowledge intelligence platform built on a modern Rust backend with a React frontend. The system transforms tacit knowledge into searchable, connected intelligence through AI-powered document understanding and RAG-based search.

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Client Layer                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                 │
│   │   React SPA  │    │Safari Clipper│    │  Slack Bot   │                 │
│   │   (Web UI)   │    │  (Browser)   │    │  (Future)    │                 │
│   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘                 │
│          │                   │                   │                          │
└──────────┼───────────────────┼───────────────────┼──────────────────────────┘
           │                   │                   │
           └───────────────────┼───────────────────┘
                               │ HTTPS / WebSocket
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              API Layer (Rust/Axum)                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐    │
│   │    Auth     │   │  Documents  │   │   Search    │   │  WebSocket  │    │
│   │   Routes    │   │   Routes    │   │   Routes    │   │   Handler   │    │
│   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘    │
│          │                 │                 │                 │            │
│   ┌──────┴─────────────────┴─────────────────┴─────────────────┴──────┐    │
│   │                         Middleware Layer                           │    │
│   │   (Auth, Rate Limiting, CORS, Logging, Tracing)                   │    │
│   └──────┬─────────────────┬─────────────────┬─────────────────┬──────┘    │
│          │                 │                 │                 │            │
│   ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐    │
│   │    Auth     │   │  Document   │   │   Search    │   │     AI      │    │
│   │   Service   │   │   Service   │   │   Service   │   │   Service   │    │
│   └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘    │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
                               │
           ┌───────────────────┼───────────────────┐
           │                   │                   │
           ▼                   ▼                   ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│   PostgreSQL     │  │   OpenSearch     │  │   External APIs  │
│   + pgvector     │  │   (Optional)     │  │                  │
│                  │  │                  │  │  ┌────────────┐  │
│  - Documents     │  │  - Full-text     │  │  │  Claude    │  │
│  - Users         │  │    Search        │  │  │  API       │  │
│  - Embeddings    │  │  - Aggregations  │  │  └────────────┘  │
│  - Audit Logs    │  │                  │  │  ┌────────────┐  │
│                  │  │                  │  │  │  OpenAI    │  │
│                  │  │                  │  │  │  Embedding │  │
└──────────────────┘  └──────────────────┘  │  └────────────┘  │
                                            └──────────────────┘
```

---

## Architecture Principles

### 1. AI-First Knowledge Management

Traditional systems rely on manual tagging and folder hierarchies. MinKy takes an AI-first approach:

```
Traditional:           MinKy:
User -> Tag -> Store   User -> Store -> AI Understand -> Auto-Connect
User -> Folder -> Find User -> Ask Question -> AI Search -> Answer
```

### 2. Local-First, Cloud-Optional

- Primary data stored in PostgreSQL (local or self-hosted)
- Vector embeddings in pgvector (no external vector DB required)
- Cloud APIs only for AI inference (Claude, OpenAI)

### 3. Progressive Enhancement

Each phase adds independent value:

```
Phase 1: Store + AI Analysis    → Already useful
Phase 2: + RAG Search           → Much more useful
Phase 3: + Knowledge Graph      → Maximum value
```

### 4. Type Safety Throughout

- Rust's type system prevents runtime errors
- Compile-time SQL validation with sqlx
- Type-safe API contracts

---

## Component Architecture

### Backend Components (Rust)

```
minky-rust/src/
├── main.rs              # Application entry point
├── lib.rs               # Library exports
├── config.rs            # Configuration management
├── error.rs             # Error types and handling
│
├── models/              # Data types and schemas
│   ├── mod.rs
│   ├── user.rs          # User, credentials
│   ├── document.rs      # Document, metadata
│   ├── embedding.rs     # Vector embeddings
│   ├── rag.rs           # RAG search types
│   ├── ai.rs            # AI analysis results
│   ├── tag.rs           # Tags and categories
│   ├── workflow.rs      # Document workflows
│   ├── agent.rs         # AI agent definitions
│   ├── skill.rs         # Agent skills
│   └── ...              # (40+ model files)
│
├── routes/              # API endpoint handlers
│   ├── mod.rs
│   ├── auth.rs          # /api/auth/*
│   ├── documents.rs     # /api/documents/*
│   ├── search.rs        # /api/search/*
│   ├── embeddings.rs    # /api/embeddings/*
│   ├── ai.rs            # /api/ai/*
│   └── ...              # (25+ route files)
│
├── services/            # Business logic
│   ├── mod.rs
│   ├── auth_service.rs
│   ├── document_service.rs
│   ├── embedding_service.rs
│   ├── understanding_service.rs  # AI document analysis
│   ├── rag_service.rs           # RAG search
│   ├── ai_service.rs            # Claude integration
│   └── ...              # (20+ service files)
│
├── middleware/          # Request processing
│   ├── mod.rs
│   ├── auth.rs          # JWT validation
│   ├── rate_limit.rs    # Rate limiting
│   └── extractor.rs     # Custom extractors
│
└── utils/               # Helper functions
    ├── mod.rs
    └── validation.rs
```

### Frontend Components (React)

```
frontend/src/
├── index.js             # Application entry
├── App.js               # Root component, routing
│
├── pages/               # Page-level components
│   ├── DocumentList.js
│   ├── DocumentView.js
│   ├── DocumentCreate.js
│   ├── DocumentEdit.js
│   ├── AnalyticsDashboard.js
│   ├── AdminPanel.js
│   └── ...
│
├── components/          # Reusable UI components
│   ├── Header.js
│   ├── SearchBar.js
│   ├── TagInput.js
│   ├── MarkdownEditor.js
│   ├── CollaborativeEditor.js
│   ├── admin/           # Admin components
│   ├── settings/        # Settings components
│   ├── clustering/      # ML visualization
│   └── ocr/             # OCR components
│
├── services/            # API communication
│   ├── api.js           # Main API client
│   └── collaborationService.js
│
├── hooks/               # Custom React hooks
│   ├── useAsync.js
│   ├── useCategories.js
│   └── useTagSuggestions.js
│
├── utils/               # Helper functions
│   ├── dateUtils.js
│   ├── highlightText.js
│   └── obsidianRenderer.js
│
└── i18n/                # Internationalization
    └── i18n.js
```

---

## Data Flow

### Document Ingestion Pipeline

```
┌──────────────┐
│   Upload     │
│  (Markdown,  │
│   HTML, PDF) │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Parse &    │
│   Extract    │
│   Content    │
└──────┬───────┘
       │
       ├─────────────────────────────────┐
       │                                 │
       ▼                                 ▼
┌──────────────┐                 ┌──────────────┐
│   Store in   │                 │   AI        │
│  PostgreSQL  │                 │  Analysis   │
│              │                 │  (Claude)   │
└──────┬───────┘                 └──────┬──────┘
       │                                │
       │                                ▼
       │                         ┌──────────────┐
       │                         │   Extract:   │
       │                         │   - Topics   │
       │                         │   - Summary  │
       │                         │   - Insights │
       │                         └──────┬───────┘
       │                                │
       │     ┌──────────────────────────┘
       │     │
       ▼     ▼
┌──────────────┐
│   Generate   │
│  Embedding   │
│  (OpenAI)    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Store in   │
│  pgvector    │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Index in   │
│  OpenSearch  │
│  (Optional)  │
└──────────────┘
```

### RAG Search Flow

```
┌──────────────┐
│   User       │
│   Question   │
│              │
│ "How did we  │
│  solve the   │
│  auth bug?"  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Embed      │
│   Question   │
│   (OpenAI)   │
└──────┬───────┘
       │
       ▼
┌──────────────┐     ┌─────────────────────────────┐
│   Vector     │────►│   pgvector cosine search    │
│   Search     │     │   SELECT * FROM embeddings  │
│              │     │   ORDER BY embedding <=>    │
│              │     │   $query_embedding          │
└──────┬───────┘     └─────────────────────────────┘
       │
       ▼
┌──────────────┐
│   Retrieve   │
│   Top K      │
│   Chunks     │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Assemble   │
│   Context    │
│   + Metadata │
└──────┬───────┘
       │
       ▼
┌──────────────┐     ┌─────────────────────────────┐
│   Generate   │────►│   Claude API                │
│   Answer     │     │   - Context: [documents]    │
│   (Claude)   │     │   - Question: [user query]  │
│              │     │   - Instructions: [prompt]  │
└──────┬───────┘     └─────────────────────────────┘
       │
       ▼
┌──────────────┐
│   Response   │
│   + Sources  │
│   + Metadata │
└──────────────┘
```

---

## Technology Decisions

### Why Rust for Backend?

| Factor | Python (Flask) | Rust (Axum) |
|--------|----------------|-------------|
| **Performance** | ~50ms p50 | ~5ms p50 |
| **Memory** | ~500MB | ~100MB |
| **Concurrency** | 1,000 | 10,000+ |
| **Type Safety** | Runtime errors | Compile-time |
| **Cold Start** | ~3s | ~50ms |

### Why pgvector over Dedicated Vector DB?

| Factor | pgvector | Pinecone/Qdrant |
|--------|----------|-----------------|
| **Deployment** | Same as data | Separate service |
| **Cost** | Included | Additional |
| **Transactions** | Full ACID | Limited |
| **Joins** | Native SQL | API calls |
| **Backup** | One system | Two systems |

For teams storing <1M vectors, pgvector offers simplicity without sacrificing performance.

### Why Claude for AI?

| Factor | Claude | GPT-4 |
|--------|--------|-------|
| **Context Window** | 200K tokens | 128K tokens |
| **Document Analysis** | Excellent | Excellent |
| **Reasoning** | Strong | Strong |
| **API Reliability** | High | High |

Both are excellent choices. Claude's larger context window helps with document analysis.

---

## Database Schema

### Core Tables

```sql
-- Users and Authentication
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(50) DEFAULT 'user',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Documents
CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    title VARCHAR(500) NOT NULL,
    content TEXT NOT NULL,
    source_type VARCHAR(50) NOT NULL,
    source_url VARCHAR(2048),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Vector Embeddings (pgvector)
CREATE TABLE document_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    embedding vector(1536) NOT NULL,  -- OpenAI dimension
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(document_id, chunk_index)
);

-- Create HNSW index for fast similarity search
CREATE INDEX ON document_embeddings
    USING hnsw (embedding vector_cosine_ops);

-- Document Understanding (AI Analysis)
CREATE TABLE document_understanding (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE UNIQUE,
    topics JSONB NOT NULL,
    summary TEXT NOT NULL,
    insights JSONB,
    technologies JSONB,
    problem_solved TEXT,
    relevant_for JSONB,
    analyzed_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Entity Relationship Diagram

```
┌─────────────┐       ┌──────────────────────┐
│    users    │       │      documents       │
├─────────────┤       ├──────────────────────┤
│ id (PK)     │──┐    │ id (PK)              │
│ email       │  │    │ user_id (FK)         │──┐
│ password    │  └───►│ title                │  │
│ name        │       │ content              │  │
│ role        │       │ source_type          │  │
└─────────────┘       └──────────────────────┘  │
                              │                 │
              ┌───────────────┼─────────────┐   │
              │               │             │   │
              ▼               ▼             ▼   │
┌──────────────────┐ ┌──────────────┐ ┌──────────────┐
│document_embeddings│ │document_under│ │   comments   │
├──────────────────┤ │   standing   │ ├──────────────┤
│ id (PK)          │ ├──────────────┤ │ id (PK)      │
│ document_id (FK) │ │ id (PK)      │ │ document_id  │
│ chunk_index      │ │ document_id  │ │ user_id (FK) │◄┘
│ chunk_text       │ │ topics       │ │ content      │
│ embedding        │ │ summary      │ └──────────────┘
└──────────────────┘ │ insights     │
                     └──────────────┘
```

---

## API Design

### RESTful Conventions

```
GET    /api/documents          # List documents
POST   /api/documents          # Create document
GET    /api/documents/:id      # Get document
PUT    /api/documents/:id      # Update document
DELETE /api/documents/:id      # Delete document

POST   /api/search/semantic    # Vector search
POST   /api/search/ask         # RAG Q&A

POST   /api/embeddings         # Generate embedding
```

### Request/Response Format

```rust
// Standard API Response
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub meta: Option<ResponseMeta>,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    pub total: Option<i64>,
    pub page: Option<i32>,
    pub limit: Option<i32>,
}
```

### Error Handling

```rust
// Error types
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("External API error: {0}")]
    ExternalApi(String),
}

// HTTP status mapping
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalApi(_) => StatusCode::BAD_GATEWAY,
        };
        // ...
    }
}
```

---

## Security Architecture

### Authentication Flow

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │────►│  Login   │────►│  Verify  │
│          │     │ Request  │     │ Password │
└──────────┘     └──────────┘     └────┬─────┘
                                       │
                                       ▼
                                 ┌──────────┐
                                 │  Argon2  │
                                 │  Hash    │
                                 └────┬─────┘
                                       │
                      ┌────────────────┼────────────────┐
                      │ Valid          │ Invalid        │
                      ▼                ▼                │
               ┌──────────┐     ┌──────────┐           │
               │  Create  │     │  Reject  │           │
               │   JWT    │     │ (401)    │           │
               └────┬─────┘     └──────────┘           │
                    │                                   │
                    ▼                                   │
               ┌──────────┐                            │
               │  Return  │                            │
               │  Token   │                            │
               └──────────┘                            │
```

### Security Measures

```rust
// Password hashing (Argon2)
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?
        .to_string())
}

// JWT claims
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,      // User ID
    pub email: String,
    pub role: String,
    pub exp: usize,     // Expiration
    pub iat: usize,     // Issued at
}

// Rate limiting
pub struct RateLimiter {
    requests_per_minute: u32,
    requests_per_hour: u32,
}
```

### Security Checklist

- [x] JWT authentication with refresh tokens
- [x] Argon2 password hashing
- [x] Rate limiting per endpoint
- [x] CORS configuration
- [x] SQL injection prevention (parameterized queries)
- [x] XSS prevention (content sanitization)
- [x] CSRF protection
- [x] Audit logging
- [x] Input validation (validator crate)

---

## Performance Considerations

### Database Optimization

```sql
-- Indexes for common queries
CREATE INDEX idx_documents_user_id ON documents(user_id);
CREATE INDEX idx_documents_created_at ON documents(created_at DESC);
CREATE INDEX idx_embeddings_document_id ON document_embeddings(document_id);

-- HNSW index for vector search (faster than IVFFlat for <1M vectors)
CREATE INDEX ON document_embeddings
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);
```

### Connection Pooling

```rust
// sqlx pool configuration
let pool = PgPoolOptions::new()
    .max_connections(config.database_max_connections)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&config.database_url)
    .await?;
```

### Caching Strategy

```
Request Flow with Caching:

┌────────┐    ┌─────────┐    ┌──────────┐    ┌──────────┐
│ Client │───►│  Cache  │───►│  Service │───►│ Database │
│        │    │  Check  │    │          │    │          │
└────────┘    └────┬────┘    └──────────┘    └──────────┘
                   │
            ┌──────┴──────┐
            │ Cache Hit   │ Cache Miss
            ▼             ▼
      ┌──────────┐  ┌──────────┐
      │  Return  │  │  Query   │
      │  Cached  │  │  & Cache │
      └──────────┘  └──────────┘
```

### Performance Targets

| Operation | Target p50 | Target p99 |
|-----------|------------|------------|
| Health check | <1ms | <5ms |
| Document CRUD | <10ms | <50ms |
| Vector search (10k docs) | <20ms | <100ms |
| RAG search | <2s | <5s |

---

## Deployment Architecture

### Development

```
┌────────────────┐    ┌────────────────┐
│   Frontend     │    │   Backend      │
│   (npm start)  │───►│   (cargo run)  │
│   :3000        │    │   :8000        │
└────────────────┘    └───────┬────────┘
                              │
                      ┌───────┴────────┐
                      │   PostgreSQL   │
                      │   :5432        │
                      └────────────────┘
```

### Production (Docker)

```yaml
# docker-compose.yml structure
services:
  frontend:
    build: ./frontend
    ports: ["80:80"]

  backend:
    build: ./minky-rust
    ports: ["8000:8000"]
    environment:
      - DATABASE_URL=postgres://...

  postgres:
    image: pgvector/pgvector:pg16
    volumes:
      - pgdata:/var/lib/postgresql/data

  opensearch:
    image: opensearchproject/opensearch:2.11.0
    # ...
```

### Production (Kubernetes)

```
┌─────────────────────────────────────────────────────────────┐
│                        Kubernetes Cluster                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌─────────────┐                    ┌─────────────┐        │
│   │   Ingress   │                    │   Ingress   │        │
│   │   (HTTPS)   │                    │   (HTTPS)   │        │
│   └──────┬──────┘                    └──────┬──────┘        │
│          │                                  │               │
│          ▼                                  ▼               │
│   ┌─────────────┐                    ┌─────────────┐        │
│   │  Frontend   │                    │  Backend    │        │
│   │  Service    │                    │  Service    │        │
│   │  (3 pods)   │                    │  (3 pods)   │        │
│   └─────────────┘                    └──────┬──────┘        │
│                                             │               │
│          ┌──────────────────────────────────┤               │
│          │                                  │               │
│          ▼                                  ▼               │
│   ┌─────────────┐                    ┌─────────────┐        │
│   │ PostgreSQL  │                    │ OpenSearch  │        │
│   │ StatefulSet │                    │ StatefulSet │        │
│   │ (pgvector)  │                    │             │        │
│   └─────────────┘                    └─────────────┘        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Future Architecture Considerations

### Scaling Vector Search

For >1M documents, consider:

1. **Horizontal Partitioning**: Split embeddings by date/category
2. **Dedicated Vector DB**: Migrate to Qdrant/Milvus
3. **Hybrid Search**: Combine keyword + semantic

### Real-time Collaboration

Current WebSocket implementation:

```
┌──────────┐     ┌──────────┐     ┌──────────┐
│ Client A │◄───►│ WS Server│◄───►│ Client B │
│          │     │  (Axum)  │     │          │
└──────────┘     └────┬─────┘     └──────────┘
                      │
                      ▼
               ┌──────────┐
               │  Redis   │ (for multi-instance)
               │  PubSub  │
               └──────────┘
```

### Multi-tenant Support

Future considerations for SaaS deployment:

- Database-per-tenant or row-level security
- Tenant-specific embedding spaces
- Usage metering and quotas

---

## References

- [Axum Documentation](https://docs.rs/axum)
- [pgvector GitHub](https://github.com/pgvector/pgvector)
- [RAG Best Practices (Anthropic)](https://docs.anthropic.com/claude/docs/retrieval-augmented-generation)
- [sqlx Documentation](https://docs.rs/sqlx)

---

*Last updated: 2026-02-19*
