# MinKy RAG E2E Setup Guide

Operational procedure for running the MinKy RAG (Retrieval-Augmented Generation) end-to-end flow locally with embedded vector search.

## Prerequisites

Ensure you have the following installed:

- Docker and Docker Compose (for PostgreSQL + pgvector)
- Rust toolchain (1.70+): `rustc --version`
- sqlx-cli for database migrations: `cargo install sqlx-cli --no-default-features --features postgres`
- curl for testing API endpoints
- GNU Make (optional, for convenience)

Verify installations:

```bash
docker --version
rustc --version
sqlx --version
curl --version
```

## Architecture Overview

The RAG pipeline operates as follows:

1. **Document Ingestion**: Markdown files from a vault directory are uploaded via `/api/vault/ingest`
2. **Embedding Generation**: Local fastembed-rs (NomicEmbedTextV1.5, 768-dim) embeds document chunks
3. **Vector Storage**: Embeddings stored in PostgreSQL pgvector table
4. **Semantic Search**: User queries are embedded and matched against stored vectors
5. **RAG Response**: Top K relevant chunks are assembled into context for Claude API

## Step 1: Start PostgreSQL with pgvector

Create a `docker-compose.yml` in the project root:

```yaml
version: '3.8'

services:
  postgres:
    image: pgvector/pgvector:pg16-v0.7.0
    container_name: minky-postgres
    environment:
      POSTGRES_USER: minky_user
      POSTGRES_PASSWORD: minky_password
      POSTGRES_DB: minky_db
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U minky_user"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
```

Start the container:

```bash
docker-compose up -d postgres
```

Verify the container is running:

```bash
docker ps | grep minky-postgres
docker-compose exec postgres pg_isready -U minky_user
```

## Step 2: Configure Database Connection

In the project root, create or update `.env`:

```bash
# Database
DATABASE_URL=postgresql://minky_user:minky_password@localhost:5432/minky_db

# Local Embedding (activate fastembed-rs)
LOCAL_EMBEDDING_ENABLED=true

# JWT Secret (any random string for auth)
JWT_SECRET=your-secret-key-change-this-in-production

# Optional: Logging level
RUST_LOG=info
```

Verify the connection string works:

```bash
sqlx database create
```

Expected output: `Database successfully created`

## Step 3: Apply Database Migrations

Migrations are located in `minky-rust/migrations/`. Apply all migrations (001–010):

```bash
cd minky-rust
sqlx migrate run
```

Verify migrations applied:

```bash
sqlx migrate list
```

Expected: All migrations listed with status "success".

### Migration Summary

| File | Purpose |
|------|---------|
| 001_initial_schema.sql | Core tables (users, documents, sessions) |
| 002_documents.sql | Document metadata table |
| 003_embeddings.sql | Embedding storage with pgvector |
| 004_chunks.sql | Document chunking table |
| 005_sessions.sql | Chat/RAG session tracking |
| 006_chat_messages.sql | Message history |
| 007_search_history.sql | Search query tracking |
| 008_performance_indexes.sql | Query optimization indexes |
| 009_embedding_model.sql | Enum for embedding model selection (includes `nomic_embed_text_v1_5`) |
| 010_source_path.sql | Document source tracking with partial unique index |

## Step 4: Start the Rust Server

From the project root:

```bash
cd minky-rust
cargo build --release
cargo run --release
```

Or for development (with faster iteration):

```bash
cargo run
```

Expected output:

```
[INFO] Server running on 0.0.0.0:3000
[INFO] Connected to PostgreSQL
[INFO] Local embedding enabled: NomicEmbedTextV1.5 (768-dim)
```

The server listens on `http://localhost:3000`.

## Step 5: Ingest Documents from a Vault

Create a test vault with markdown files:

```bash
mkdir -p test_vault
cat > test_vault/sample_doc_1.md << 'EOF'
# Understanding Rust Ownership

Rust's ownership system is one of its most distinctive features. It ensures memory safety
without a garbage collector. Key concepts include move semantics, borrowing, and lifetimes.

## Ownership Rules

1. Each value has one owner
2. When owner is dropped, value is freed
3. You can borrow references to owned values

This prevents double-free errors and data races.
EOF

cat > test_vault/sample_doc_2.md << 'EOF'
# PostgreSQL Vector Search with pgvector

pgvector is a PostgreSQL extension that enables vector storage and similarity search.
It supports HNSW and IVFFlat indexing for efficient nearest-neighbor queries.

## Setup Steps

1. Install pgvector extension
2. Create vector columns with dimension
3. Index vectors for performance
4. Query using <-> or <#> operators

Vector search is foundational for semantic retrieval.
EOF
```

Ingest the vault via API:

```bash
curl -X POST http://localhost:3000/api/vault/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "root": "'$(pwd)'/test_vault",
    "recursive": true
  }'
```

Expected response:

```json
{
  "success": true,
  "documents_ingested": 2,
  "chunks_created": 6,
  "embeddings_generated": 6,
  "message": "Vault ingestion completed"
}
```

Verify documents in the database:

```bash
docker-compose exec postgres psql -U minky_user -d minky_db -c \
  "SELECT id, title, source_path, embedding_model FROM documents LIMIT 5;"
```

Expected columns:
- `id`: UUID
- `title`: Extracted from markdown filename
- `source_path`: Absolute path to source file
- `embedding_model`: `nomic_embed_text_v1_5`

## Step 6: Test RAG Search

Query the RAG endpoint:

```bash
curl -X POST http://localhost:3000/api/search/ask \
  -H "Content-Type: application/json" \
  -d '{
    "query": "How does Rust ownership prevent memory errors?",
    "top_k": 3
  }'
```

Expected response:

```json
{
  "success": true,
  "query": "How does Rust ownership prevent memory errors?",
  "answer": "Rust's ownership system prevents memory errors by...",
  "sources": [
    {
      "document_id": "...",
      "title": "Understanding Rust Ownership",
      "chunk_index": 0,
      "relevance_score": 0.87
    }
  ],
  "processing_time_ms": 145
}
```

Test another query:

```bash
curl -X POST http://localhost:3000/api/search/ask \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is pgvector and how is it used?",
    "top_k": 2
  }'
```

## Step 7: Verify Embedding Pipeline

Check that embeddings were generated locally (no external API calls):

```bash
docker-compose exec postgres psql -U minky_user -d minky_db -c \
  "SELECT COUNT(*) as total_embeddings FROM embeddings;"
```

Inspect a specific embedding vector:

```bash
docker-compose exec postgres psql -U minky_user -d minky_db -c \
  "SELECT id, embedding_model, vector_dimension FROM embeddings LIMIT 1;"
```

Expected:
- `embedding_model`: `nomic_embed_text_v1_5`
- `vector_dimension`: `768`

Monitor memory and CPU usage during embedding generation:

```bash
# In a separate terminal, watch Docker stats
docker stats minky-postgres
```

## Operational Workflows

### Workflow A: Add New Documents

To add documents without restarting the server:

```bash
# 1. Place new markdown files in test_vault/
cp your_docs/*.md test_vault/

# 2. Trigger re-ingestion
curl -X POST http://localhost:3000/api/vault/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "root": "'$(pwd)'/test_vault",
    "recursive": true
  }'

# 3. Query immediately
curl -X POST http://localhost:3000/api/search/ask \
  -H "Content-Type: application/json" \
  -d '{"query": "your new question"}'
```

### Workflow B: Reset and Clean State

To start fresh (clears all ingested documents):

```bash
# 1. Stop the server (Ctrl+C)

# 2. Drop and recreate the database
docker-compose exec postgres psql -U minky_user -c "DROP DATABASE minky_db;"
docker-compose exec postgres psql -U minky_user -c "CREATE DATABASE minky_db;"

# 3. Re-apply migrations
cd minky-rust && sqlx migrate run

# 4. Restart server and ingest fresh vault
cargo run
```

### Workflow C: Monitor RAG Performance

Track query performance and hit rates:

```bash
# Query the search history table
docker-compose exec postgres psql -U minky_user -d minky_db -c \
  "SELECT query, top_k, avg_relevance_score, processing_time_ms
   FROM search_history
   ORDER BY created_at DESC LIMIT 10;"
```

## Troubleshooting

### Error: `Connection refused` on port 5432

**Cause**: PostgreSQL container not running.

**Solution**:
```bash
docker-compose up -d postgres
docker-compose logs postgres
```

### Error: `DATABASE_URL not set`

**Cause**: `.env` file missing or not loaded.

**Solution**:
```bash
# Verify .env exists in project root
cat .env | grep DATABASE_URL

# If missing, create it (see Step 2)
echo "DATABASE_URL=postgresql://minky_user:minky_password@localhost:5432/minky_db" >> .env
```

### Error: `Relation "documents" does not exist`

**Cause**: Migrations not applied.

**Solution**:
```bash
cd minky-rust
sqlx migrate run
sqlx migrate list  # Verify all migrations passed
```

### Error: `LOCAL_EMBEDDING_ENABLED not recognized`

**Cause**: Environment variable not loaded.

**Solution**:
```bash
# In minky-rust/src/main.rs, verify:
let local_embedding = std::env::var("LOCAL_EMBEDDING_ENABLED")
    .map(|v| v.to_lowercase() == "true")
    .unwrap_or(false);

# Restart server after updating .env
cargo run
```

### Slow Embedding Generation

**Cause**: fastembed running on CPU (normal for first ingestion).

**Solution**:
- First run ingests documents serially; is I/O bound on PostgreSQL
- Subsequent queries use cached embeddings (fast)
- To parallelize: modify `services/embedding_service.rs` to use `rayon` work-stealing

**Monitor progress**:
```bash
docker logs -f minky-postgres  # Watch INSERT operations
watch -n 1 "docker-compose exec postgres psql -U minky_user -d minky_db -c \
  'SELECT COUNT(*) FROM embeddings;'"
```

### pgvector Extension Not Loaded

**Cause**: Container image version mismatch.

**Solution**:
```bash
docker-compose exec postgres psql -U minky_user -d minky_db -c "CREATE EXTENSION IF NOT EXISTS vector;"
docker-compose exec postgres psql -U minky_user -d minky_db -c "SELECT extname FROM pg_extension WHERE extname = 'vector';"
```

Expected: `vector` extension listed.

### Out of Memory During Large Ingestion

**Cause**: Processing large vaults with insufficient Docker memory allocation.

**Solution**:
```yaml
# In docker-compose.yml, add:
services:
  postgres:
    mem_limit: 4g  # Increase if needed
```

Then restart:
```bash
docker-compose down
docker-compose up -d postgres
```

## Maintenance Tasks

### Weekly: Verify Data Integrity

```bash
# Check for orphaned embeddings (embeddings without documents)
docker-compose exec postgres psql -U minky_user -d minky_db -c \
  "SELECT COUNT(*) as orphaned_embeddings
   FROM embeddings e
   LEFT JOIN chunks c ON e.chunk_id = c.id
   WHERE c.id IS NULL;"

# Should return 0
```

### Monthly: Optimize Indexes

```bash
docker-compose exec postgres psql -U minky_user -d minky_db -c "ANALYZE;"
```

### Quarterly: Backup Vector Data

```bash
docker-compose exec postgres pg_dump -U minky_user minky_db > backup_$(date +%Y%m%d).sql
```

## Performance Benchmarks

Typical performance on a 2021 MacBook Pro (16GB RAM, M1 Pro):

| Operation | Time | Notes |
|-----------|------|-------|
| Ingest 10 documents (50 chunks) | 5-8s | Local fastembed |
| Single RAG query (k=3) | 200-300ms | Vector search + Claude |
| Bulk insert 1000 embeddings | 15-20s | Batch insert with pgvector |

## Next Steps

After successful E2E verification:

1. **Scale to Production**: Move PostgreSQL to managed service (RDS, Neon, Supabase)
2. **Add Authentication**: Implement JWT token validation on API endpoints
3. **Enable Caching**: Add Redis layer for frequent queries
4. **Monitor Observability**: Set up Prometheus metrics and structured logging
5. **Document Sync**: Implement webhook-based vault sync for CI/CD integration

## References

- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [fastembed-rs](https://github.com/qdrant/fastembed-rs)
- [Axum Web Framework](https://github.com/tokio-rs/axum)
- [SQLx Documentation](https://github.com/launchbadge/sqlx)
