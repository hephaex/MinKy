# Getting Started with MinKy

This guide will walk you through setting up MinKy for local development.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Database Setup](#database-setup)
4. [Environment Configuration](#environment-configuration)
5. [Running the Application](#running-the-application)
6. [Verifying Installation](#verifying-installation)
7. [Common Issues](#common-issues)
8. [Next Steps](#next-steps)

---

## Prerequisites

### Required Software

| Software | Version | Purpose | Installation |
|----------|---------|---------|--------------|
| **Rust** | 1.75+ | Backend runtime | [rustup.rs](https://rustup.rs/) |
| **Node.js** | 18+ | Frontend runtime | [nodejs.org](https://nodejs.org/) |
| **PostgreSQL** | 15+ | Database | [postgresql.org](https://www.postgresql.org/download/) |
| **pgvector** | 0.5+ | Vector extension | See [pgvector setup](#pgvector-installation) |

### Optional Software

| Software | Version | Purpose | Installation |
|----------|---------|---------|--------------|
| **OpenSearch** | 2.x | Full-text search | [opensearch.org](https://opensearch.org/docs/latest/install-and-configure/install-opensearch/index/) |
| **Docker** | 24+ | Containerized deployment | [docker.com](https://www.docker.com/get-started) |

### Verify Prerequisites

```bash
# Check Rust
rustc --version
# Expected: rustc 1.75.0 or higher

# Check Cargo
cargo --version
# Expected: cargo 1.75.0 or higher

# Check Node.js
node --version
# Expected: v18.0.0 or higher

# Check npm
npm --version
# Expected: 9.0.0 or higher

# Check PostgreSQL
psql --version
# Expected: psql (PostgreSQL) 15.0 or higher
```

---

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/hephaex/minky.git
cd minky
```

### 2. Install Rust Dependencies

```bash
cd minky-rust
cargo build
```

This will download and compile all Rust dependencies. First build may take several minutes.

### 3. Install Frontend Dependencies

```bash
cd frontend
npm install
```

### 4. Install sqlx-cli (for database migrations)

```bash
cargo install sqlx-cli --features postgres
```

---

## Database Setup

### PostgreSQL Installation

#### macOS (Homebrew)

```bash
brew install postgresql@15
brew services start postgresql@15
```

#### Ubuntu/Debian

```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

#### Windows

Download and install from [postgresql.org](https://www.postgresql.org/download/windows/)

### pgvector Installation

pgvector is required for vector similarity search.

#### macOS (Homebrew)

```bash
brew install pgvector
```

#### Ubuntu/Debian

```bash
sudo apt install postgresql-15-pgvector
```

#### From Source

```bash
cd /tmp
git clone --branch v0.5.1 https://github.com/pgvector/pgvector.git
cd pgvector
make
make install
```

### Create Database

```bash
# Create database user (if needed)
createuser -s minky

# Create database
createdb minky

# Enable pgvector extension
psql minky -c "CREATE EXTENSION IF NOT EXISTS vector;"

# Verify extension
psql minky -c "SELECT * FROM pg_extension WHERE extname = 'vector';"
```

### Run Migrations

```bash
cd minky-rust
sqlx migrate run
```

Expected output:

```
Applied 001_initial_schema (5.234ms)
Applied 002_workflows (2.891ms)
Applied 003_pgvector_embeddings (3.456ms)
```

---

## Environment Configuration

### Backend Configuration

Create the environment file:

```bash
cp minky-rust/.env.example minky-rust/.env
```

Edit `minky-rust/.env`:

```env
# Server Settings
HOST=127.0.0.1
PORT=8000

# Database Connection
DATABASE_URL=postgres://minky:password@localhost:5432/minky
DATABASE_MAX_CONNECTIONS=10

# JWT Authentication
# IMPORTANT: Change this in production!
JWT_SECRET=your-super-secret-jwt-key-minimum-32-characters
JWT_EXPIRATION_HOURS=24

# OpenSearch (optional - comment out if not using)
OPENSEARCH_URL=http://localhost:9200

# AI API Keys (optional - required for AI features)
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...

# Git Integration (optional)
# GIT_REPO_PATH=/path/to/your/obsidian/vault

# Logging
RUST_LOG=minky=debug,tower_http=debug
```

### Environment Variables Explained

| Variable | Required | Description |
|----------|----------|-------------|
| `HOST` | No | Server bind address (default: 127.0.0.1) |
| `PORT` | No | Server port (default: 8000) |
| `DATABASE_URL` | **Yes** | PostgreSQL connection string |
| `DATABASE_MAX_CONNECTIONS` | No | Connection pool size (default: 10) |
| `JWT_SECRET` | **Yes** | Secret key for JWT signing (min 32 chars) |
| `JWT_EXPIRATION_HOURS` | No | Token expiration (default: 24) |
| `OPENSEARCH_URL` | No | OpenSearch server URL |
| `OPENAI_API_KEY` | No* | Required for embedding generation |
| `ANTHROPIC_API_KEY` | No* | Required for AI document analysis |
| `GIT_REPO_PATH` | No | Path for Git sync feature |
| `RUST_LOG` | No | Logging level configuration |

*Required for AI features to work

### Frontend Configuration

The frontend uses a proxy configuration in `package.json`:

```json
{
  "proxy": "http://localhost:5000"
}
```

For Rust backend, update the proxy or use environment variables:

```bash
# Create frontend .env (optional)
echo "REACT_APP_API_URL=http://localhost:8000" > frontend/.env.local
```

---

## Running the Application

### Development Mode

#### Terminal 1: Start Backend

```bash
cd minky-rust
cargo run
```

Expected output:

```
   Compiling minky v0.1.0
    Finished dev [optimized + debuginfo] target(s) in 5.23s
     Running `target/debug/minky`
2026-02-19T10:00:00.000Z  INFO minky: Starting MinKy server on 127.0.0.1:8000
2026-02-19T10:00:00.001Z  INFO minky: Database connected
2026-02-19T10:00:00.002Z  INFO minky: Server ready
```

#### Terminal 2: Start Frontend

```bash
cd frontend
npm start
```

The browser will automatically open to `http://localhost:3000`

### Production Mode

#### Build Backend

```bash
cd minky-rust
cargo build --release
./target/release/minky
```

#### Build Frontend

```bash
cd frontend
npm run build
# Serve the 'build' directory with nginx or similar
```

---

## Verifying Installation

### 1. Check Backend Health

```bash
curl http://localhost:8000/health
```

Expected response:

```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

### 2. Check Database Connection

```bash
curl http://localhost:8000/api/documents
```

Expected response (empty initially):

```json
{
  "documents": [],
  "total": 0
}
```

### 3. Test Authentication

```bash
# Register a user
curl -X POST http://localhost:8000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "securepassword123", "name": "Test User"}'

# Login
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "securepassword123"}'
```

### 4. Verify pgvector

```bash
psql minky -c "SELECT '[1,2,3]'::vector <-> '[3,2,1]'::vector AS distance;"
```

Expected output:

```
 distance
----------
 2.828...
```

### 5. Frontend Check

Open `http://localhost:3000` in your browser. You should see the MinKy dashboard.

---

## Common Issues

### Issue: "role 'minky' does not exist"

**Solution:**

```bash
createuser -s minky
# Or specify password:
createuser -P minky
```

### Issue: "extension 'vector' is not available"

**Solution:**

Ensure pgvector is installed:

```bash
# macOS
brew install pgvector

# Ubuntu
sudo apt install postgresql-15-pgvector
```

Then restart PostgreSQL and create extension:

```bash
sudo systemctl restart postgresql
psql minky -c "CREATE EXTENSION vector;"
```

### Issue: "FATAL: database 'minky' does not exist"

**Solution:**

```bash
createdb minky
```

### Issue: "error: linker 'cc' not found" (Rust)

**Solution (macOS):**

```bash
xcode-select --install
```

**Solution (Ubuntu):**

```bash
sudo apt install build-essential
```

### Issue: "Cannot find module 'react-scripts'"

**Solution:**

```bash
cd frontend
rm -rf node_modules package-lock.json
npm install
```

### Issue: CORS errors in browser

**Solution:**

The backend includes CORS middleware. Ensure both servers are running and the proxy is configured correctly.

### Issue: "Connection refused" on port 5432

**Solution:**

Start PostgreSQL:

```bash
# macOS
brew services start postgresql@15

# Linux
sudo systemctl start postgresql
```

### Issue: JWT secret too short

**Solution:**

Generate a secure secret:

```bash
openssl rand -hex 32
```

Update `JWT_SECRET` in `.env` with the generated value.

---

## Next Steps

### 1. Create Your First Document

```bash
curl -X POST http://localhost:8000/api/documents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "title": "My First Document",
    "content": "# Hello MinKy\n\nThis is my first document.",
    "source_type": "manual"
  }'
```

### 2. Explore the API

See [API_DOCUMENTATION.md](API_DOCUMENTATION.md) for full API reference.

### 3. Configure AI Features

Add your API keys to `.env`:

```env
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
```

### 4. Try RAG Search

```bash
curl -X POST http://localhost:8000/api/search/ask \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{"question": "What documents do I have?"}'
```

### 5. Set Up OpenSearch (Optional)

For better full-text search, install OpenSearch:

```bash
# Docker (easiest)
docker run -d -p 9200:9200 -p 9600:9600 \
  -e "discovery.type=single-node" \
  -e "OPENSEARCH_INITIAL_ADMIN_PASSWORD=Admin123!" \
  opensearchproject/opensearch:2.11.0
```

### 6. Read the Architecture Guide

For deeper understanding, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Development Tips

### Watch Mode (Rust)

Install cargo-watch for auto-reload:

```bash
cargo install cargo-watch
cd minky-rust
cargo watch -x run
```

### Database Reset

```bash
cd minky-rust
sqlx database drop
sqlx database create
sqlx migrate run
```

### View Logs

Set detailed logging:

```env
RUST_LOG=minky=trace,tower_http=trace,sqlx=debug
```

### API Testing with HTTPie

```bash
# Install httpie
brew install httpie  # macOS
# or: pip install httpie

# Use it
http POST :8000/api/auth/login email=test@example.com password=secret
```

---

## Getting Help

- **Documentation**: Check the `Docs/` directory
- **Issues**: Open a GitHub issue
- **CLAUDE.md**: Read for project context and conventions

---

*Last updated: 2026-02-19*
