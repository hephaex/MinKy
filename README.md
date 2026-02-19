# MinKy - Team Knowledge Intelligence Platform

<p align="center">
  <strong>Turn tacit knowledge into searchable, connected team intelligence</strong>
</p>

<p align="center">
  <a href="#features">Features</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#architecture">Architecture</a> |
  <a href="#api-reference">API Reference</a> |
  <a href="#documentation">Documentation</a> |
  <a href="#contributing">Contributing</a>
</p>

---

## Overview

MinKy is a knowledge intelligence platform designed for small teams (3-9 members) that transforms **tacit knowledge into explicit, searchable assets**. Unlike traditional document management systems that rely on manual tagging and categorization, MinKy uses AI to understand, connect, and surface relevant knowledge through natural language conversations.

### The Problem

- Team knowledge is scattered across Slack, emails, documents, and individual minds
- Manual tagging is inconsistent and rarely maintained
- Finding relevant past decisions or solutions requires knowing where to look
- When team members leave, their knowledge leaves with them

### The Solution

MinKy captures knowledge from various sources, uses AI to understand context and relationships, stores it with vector embeddings, and enables natural language search through RAG (Retrieval-Augmented Generation).

---

## Features

### Phase 1: Knowledge Understanding (Current)

- **AI Document Analysis**: Claude analyzes uploaded documents to extract topics, summaries, and insights
- **Vector Embeddings**: Documents stored with pgvector for semantic similarity search
- **Automatic Connections**: Related documents linked based on content similarity, not manual tags

### Phase 2: Conversational Search (In Progress)

- **Natural Language Q&A**: Ask questions in plain language, get answers with source citations
- **RAG-Powered Search**: Combines vector search with AI generation for accurate responses
- **Context-Aware Responses**: Understands your team's terminology and context

### Phase 3: Knowledge Connections (Planned)

- **Knowledge Graph**: Visualize relationships between documents and concepts
- **Smart Recommendations**: "People who found this useful also looked at..."
- **Gap Detection**: Identify undocumented areas in your knowledge base

### Additional Features

- **Multi-Source Ingestion**: Markdown, Safari Clipper, Slack messages
- **Korean Language Support**: Full Korean NLP with MeCab integration
- **OCR Processing**: Extract text from images and scanned documents
- **Real-Time Collaboration**: WebSocket-based collaborative editing
- **Analytics Dashboard**: Track knowledge usage and team engagement
- **Document Workflows**: Approval processes with customizable templates
- **Version Control**: Full document history with diff and restore

---

## Quick Start

### Prerequisites

- **Rust** 1.75+ (for backend)
- **Node.js** 18+ (for frontend)
- **PostgreSQL** 15+ with **pgvector** extension
- **OpenSearch** 2.x (optional, for full-text search)

### 1. Clone the Repository

```bash
git clone https://github.com/hephaex/minky.git
cd minky
```

### 2. Set Up the Database

```bash
# Create PostgreSQL database
createdb minky

# Enable pgvector extension
psql minky -c "CREATE EXTENSION vector;"
```

### 3. Configure Environment

```bash
# Copy example configuration
cp minky-rust/.env.example minky-rust/.env

# Edit with your settings
nano minky-rust/.env
```

Key configuration options:

```env
DATABASE_URL=postgres://user:password@localhost:5432/minky
JWT_SECRET=your-secure-secret-key
ANTHROPIC_API_KEY=sk-ant-...  # For AI features
OPENAI_API_KEY=sk-...         # For embeddings
```

### 4. Run Database Migrations

```bash
cd minky-rust
cargo install sqlx-cli
sqlx migrate run
```

### 5. Start the Backend

```bash
cd minky-rust
cargo run
# Server starts at http://localhost:8000
```

### 6. Start the Frontend

```bash
cd frontend
npm install
npm start
# App opens at http://localhost:3000
```

### 7. Verify Installation

```bash
# Check backend health
curl http://localhost:8000/health

# Expected response:
# {"status":"healthy","version":"0.1.0"}
```

> For detailed setup instructions, see [Docs/GETTING_STARTED.md](Docs/GETTING_STARTED.md)

---

## Architecture

```
+-------------------+     +-------------------+     +-------------------+
|     Frontend      |     |   Rust Backend    |     |    PostgreSQL     |
|     (React)       |<--->|     (Axum)        |<--->|    + pgvector     |
|                   |     |                   |     |                   |
+-------------------+     +--------+----------+     +-------------------+
                                   |
                    +--------------+--------------+
                    |              |              |
              +-----v----+  +------v-----+  +-----v-----+
              | Claude   |  | OpenSearch |  | OpenAI    |
              | (AI)     |  | (Search)   |  | Embedding |
              +----------+  +------------+  +-----------+
```

### Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Frontend** | React 18 | Single-page application |
| **Backend** | Rust + Axum | High-performance API server |
| **Database** | PostgreSQL | Primary data storage |
| **Vector DB** | pgvector | Semantic embeddings |
| **Search** | OpenSearch | Full-text search |
| **AI** | Claude API | Document understanding, Q&A |
| **Embedding** | OpenAI API | Vector generation |

### Key Components

- **Document Pipeline**: Ingest -> AI Analysis -> Embed -> Store -> Index
- **RAG Search**: Query -> Vector Search -> Context Assembly -> AI Generation
- **Real-time**: WebSocket for collaborative editing

> For detailed architecture, see [Docs/ARCHITECTURE.md](Docs/ARCHITECTURE.md)

---

## Project Structure

```
minky/
├── minky-rust/           # Rust backend (Active)
│   ├── src/
│   │   ├── models/       # Data models (40+ types)
│   │   ├── routes/       # API endpoints (25+ routes)
│   │   ├── services/     # Business logic (20+ services)
│   │   └── middleware/   # Auth, rate limiting
│   └── migrations/       # SQL migrations
│
├── frontend/             # React frontend
│   ├── src/
│   │   ├── components/   # UI components
│   │   ├── pages/        # Page views
│   │   ├── services/     # API clients
│   │   └── hooks/        # Custom React hooks
│   └── public/
│
├── app/                  # Python backend (Legacy)
├── Docs/                 # Documentation
└── .claude/              # Claude Code agents & tools
```

---

## API Reference

### Authentication

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/auth/register` | POST | Register new user |
| `/api/auth/login` | POST | User login |
| `/api/auth/refresh` | POST | Refresh access token |
| `/api/auth/me` | GET | Get current user profile |

### Documents

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/documents` | POST | Create document |
| `/api/documents` | GET | List documents (pagination, search) |
| `/api/documents/:id` | GET | Get document with AI analysis |
| `/api/documents/:id` | PUT | Update document |
| `/api/documents/:id` | DELETE | Delete document |

### Knowledge Search (RAG)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/search/ask` | POST | Natural language Q&A |
| `/api/search/semantic` | POST | Vector similarity search |
| `/api/search/korean` | POST | Korean text search |

### Embeddings

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/embeddings` | POST | Generate embedding |
| `/api/embeddings/document/:id` | POST | Embed document |
| `/api/embeddings/search` | POST | Search by embedding |

### AI Analysis

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/understanding/:id` | GET | Get document understanding |
| `/api/understanding/:id/analyze` | POST | Analyze document |
| `/api/ai/suggestions` | POST | Get AI suggestions |

### Additional Endpoints

<details>
<summary>Tags & Categories</summary>

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/tags` | GET | List all tags |
| `/api/tags` | POST | Create tag |
| `/api/tags/:slug` | GET/PUT/DELETE | Tag CRUD |
| `/api/categories` | GET/POST | Category management |

</details>

<details>
<summary>Comments & Ratings</summary>

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/documents/:id/comments` | GET/POST | Document comments |
| `/api/comments/:id` | PUT/DELETE | Comment CRUD |
| `/api/documents/:id/rating` | GET/POST/DELETE | Document ratings |

</details>

<details>
<summary>Workflows</summary>

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/documents/:id/workflow` | GET | Get workflow info |
| `/api/documents/:id/workflow/action` | POST | Perform workflow action |
| `/api/workflows/pending` | GET | Pending reviews |
| `/api/workflow-templates` | GET/POST | Workflow templates |

</details>

<details>
<summary>Admin & Security</summary>

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/admin/users` | GET | List users (admin) |
| `/api/admin/stats` | GET | System statistics |
| `/api/security/status` | GET | Security status |
| `/api/security/logs` | GET | Security logs |

</details>

> Full API documentation: [Docs/API_DOCUMENTATION.md](Docs/API_DOCUMENTATION.md)

---

## Environment Variables

### Required

| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_SECRET` | JWT signing key (min 32 chars) |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | 127.0.0.1 | Server bind address |
| `PORT` | 8000 | Server port |
| `DATABASE_MAX_CONNECTIONS` | 10 | Connection pool size |
| `JWT_EXPIRATION_HOURS` | 24 | Token expiration |
| `OPENSEARCH_URL` | - | OpenSearch server |
| `OPENAI_API_KEY` | - | For embeddings |
| `ANTHROPIC_API_KEY` | - | For AI analysis |
| `RUST_LOG` | info | Logging level |

---

## Documentation

| Document | Description |
|----------|-------------|
| [GETTING_STARTED.md](Docs/GETTING_STARTED.md) | Detailed setup guide |
| [ARCHITECTURE.md](Docs/ARCHITECTURE.md) | System architecture deep-dive |
| [API_DOCUMENTATION.md](Docs/API_DOCUMENTATION.md) | API reference |
| [SECURITY.md](Docs/SECURITY.md) | Security guidelines |
| [CLAUDE.md](CLAUDE.md) | Project context for AI assistants |

---

## Development

### Running Tests

```bash
# Backend tests
cd minky-rust
cargo test

# Frontend tests
cd frontend
npm test
```

### Code Quality

```bash
# Rust linting
cargo clippy

# Rust formatting
cargo fmt

# Frontend linting
cd frontend
npm run lint
```

### Development Commands (Claude Code)

This project includes Claude Code agents for development automation:

```bash
/pm           # Start PM agent for task management
/next         # Get next priority task
/ci start     # Start CI/CD session
/health       # Check system health
/review       # Request code review
```

---

## Contributing

We welcome contributions! Please see our contribution guidelines:

### Development Workflow

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feat/amazing-feature`)
3. **Commit** changes (`git commit -m 'feat: Add amazing feature'`)
4. **Push** to branch (`git push origin feat/amazing-feature`)
5. **Open** a Pull Request

### Commit Convention

```
feat:     New feature
fix:      Bug fix
refactor: Code refactoring
docs:     Documentation
test:     Tests
chore:    Maintenance
```

### Code Standards

- **Rust**: Follow `cargo clippy` recommendations
- **React**: ESLint + Prettier formatting
- **Tests**: Aim for 80% coverage
- **Documentation**: Update docs with code changes

---

## Roadmap

- [x] **Phase 0**: Rust backend migration, Agent system
- [ ] **Phase 1**: Document Understanding (Current)
  - [x] pgvector integration
  - [ ] Document analysis pipeline
  - [ ] RAG search API
- [ ] **Phase 2**: Conversational Search
  - [ ] Chat interface
  - [ ] Streaming responses
- [ ] **Phase 3**: Knowledge Connections
  - [ ] Knowledge graph visualization
  - [ ] Auto-linking
- [ ] **Phase 4**: Tacit Knowledge Capture
  - [ ] Slack integration
  - [ ] Conversation mining

---

## Legacy Python Backend

The original Python/Flask backend is available in the `app/` directory for reference. It includes:

- Flask API with 140+ endpoints
- SQLAlchemy ORM
- JWT authentication
- Korean NLP support
- OpenSearch integration

The Rust backend (`minky-rust/`) is the active development target with improved performance and type safety.

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [Anthropic Claude](https://www.anthropic.com/) - AI document understanding
- [pgvector](https://github.com/pgvector/pgvector) - Vector similarity search
- [Axum](https://github.com/tokio-rs/axum) - Rust web framework

---

<p align="center">
  Built with care for teams who value their collective knowledge.
</p>
