# MinKy Architecture Analysis

**Version:** 1.0
**Date:** 2026-02-15
**Purpose:** Rust Migration Preparation

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         MinKy System                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   React     │    │    Flask    │    │  PostgreSQL │         │
│  │  Frontend   │◄──►│   Backend   │◄──►│   Database  │         │
│  │   (SPA)     │    │   (API)     │    │             │         │
│  └─────────────┘    └──────┬──────┘    └─────────────┘         │
│                            │                                    │
│         ┌──────────────────┼──────────────────┐                │
│         │                  │                  │                │
│  ┌──────▼──────┐    ┌──────▼──────┐    ┌─────▼──────┐         │
│  │  OpenSearch │    │  LLM APIs   │    │   Socket   │         │
│  │  (Search)   │    │ (AI/ML)     │    │   (WS)     │         │
│  └─────────────┘    └─────────────┘    └────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Backend Architecture

### 2.1 Layer Structure

```
app/
├── routes/           # API Layer (31 blueprints)
│   ├── documents.py
│   ├── auth.py
│   └── ...
├── models/           # Data Layer (12 models)
│   ├── document.py
│   ├── user.py
│   └── ...
├── services/         # Business Logic (14 services)
│   ├── ai_service.py
│   ├── collaboration_service.py
│   └── ...
├── middleware/       # Cross-cutting Concerns
│   └── security.py
└── utils/            # Utilities (18 files)
    ├── auth.py
    ├── validation.py
    └── ...
```

### 2.2 API Blueprint Summary

| Blueprint | Endpoints | Complexity | Key Dependencies |
|-----------|-----------|------------|------------------|
| documents | 5 | Medium | DB, Search |
| auth | 7 | Medium | JWT, Bcrypt |
| ai_suggestions | 10 | High | LLM APIs |
| comments | 7 | Medium | DB |
| tags | 10 | Medium | DB |
| workflows | 8 | Medium | State Machine |
| attachments | 8 | Medium | File System |
| analytics | 7 | Medium | DB Aggregations |
| admin | 8 | High | All Services |
| ml_analytics | 9 | High | NLP, ML |
| ocr | 5 | High | Tesseract, Vision |
| collaboration | WS | High | Socket.IO |

### 2.3 Data Model Relationships

```
                    ┌──────────┐
                    │   User   │
                    └────┬─────┘
         ┌───────────────┼───────────────┐
         │               │               │
    ┌────▼────┐    ┌─────▼─────┐   ┌─────▼─────┐
    │Document │    │  Comment  │   │  Rating   │
    └────┬────┘    └───────────┘   └───────────┘
         │
    ┌────┼────────┬──────────┬───────────┐
    │    │        │          │           │
┌───▼──┐│   ┌────▼────┐ ┌───▼───┐ ┌────▼────┐
│ Tag  │├───│ Version │ │Attach │ │Workflow │
└──────┘│   └─────────┘ └───────┘ └─────────┘
        │
   ┌────▼────┐
   │Category │
   └─────────┘
```

---

## 3. Service Dependencies

### 3.1 Core Services

```
ai_service
├── depends on: LLM Providers (OpenAI, Anthropic)
├── used by: ai_suggestions routes
└── complexity: HIGH

collaboration_service
├── depends on: Socket.IO, Redis (optional)
├── used by: websocket_events
└── complexity: HIGH (threading, sync)

opensearch_service
├── depends on: OpenSearch cluster
├── used by: documents_search, korean_search
└── complexity: MEDIUM

document_import_service
├── depends on: markitdown, file parsers
├── used by: documents_import
└── complexity: MEDIUM

ocr_service
├── depends on: Tesseract, Google Vision
├── used by: ocr routes
└── complexity: HIGH (FFI)
```

### 3.2 Dependency Graph

```
┌─────────────────┐
│   Routes (31)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│  Services (14)  │────►│  External APIs  │
└────────┬────────┘     │  - OpenAI       │
         │              │  - Anthropic    │
         │              │  - Google OCR   │
         ▼              └─────────────────┘
┌─────────────────┐
│   Models (12)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│   PostgreSQL    │     │   OpenSearch    │
└─────────────────┘     └─────────────────┘
```

---

## 4. Technology Analysis

### 4.1 Python Dependencies (56 packages)

**Core Framework:**
- Flask 2.3.3
- SQLAlchemy (via Flask-SQLAlchemy)
- Flask-JWT-Extended
- Flask-SocketIO

**AI/ML:**
- openai, anthropic
- scikit-learn, nltk, textblob
- konlpy (Korean NLP)

**Document Processing:**
- Pillow, PyMuPDF
- python-docx, WeasyPrint
- pytesseract, google-cloud-vision

**Search:**
- opensearch-py

### 4.2 Rust Equivalents

| Python | Rust | Readiness |
|--------|------|-----------|
| Flask | axum | ★★★★★ |
| SQLAlchemy | sqlx | ★★★★☆ |
| Flask-JWT | jsonwebtoken | ★★★★★ |
| Flask-SocketIO | tokio-tungstenite | ★★★★☆ |
| openai | async-openai | ★★★★☆ |
| scikit-learn | ndarray + linfa | ★★★☆☆ |
| opensearch-py | opensearch-rs | ★★★★☆ |
| Pillow | image | ★★★★★ |
| pytesseract | tesseract-rs | ★★★☆☆ |
| konlpy | mecab-rs | ★★☆☆☆ |

### 4.3 Migration Challenges

**High Complexity:**
1. Korean NLP (konlpy) - Requires custom implementation
2. WeasyPrint - No direct equivalent
3. Real-time collaboration - Complex state management

**Medium Complexity:**
1. OCR integration - FFI bindings needed
2. Document clustering - ML algorithms
3. Org-roam parser - Custom implementation

**Low Complexity:**
1. CRUD operations
2. JWT authentication
3. Basic file operations

---

## 5. Security Architecture

### 5.1 Current Security Measures (435+ fixes)

```
Authentication
├── JWT with refresh tokens
├── Bcrypt password hashing
├── Brute-force protection (5 attempts, 15min lock)
└── Session management

Authorization
├── Role-based (user, admin)
├── Resource ownership checks
├── IDOR protection
└── Admin override patterns

Input Validation
├── Schema validation (Marshmallow)
├── SQL injection prevention (parameterized queries)
├── XSS prevention (bleach sanitization)
├── File type validation (magic bytes)
└── ZIP bomb protection

Rate Limiting
├── Per-endpoint limits
├── Per-user limits
└── Global limits

Audit Logging
├── All CRUD operations
├── Authentication events
├── Admin actions
└── Security events
```

### 5.2 Security Patterns for Rust

```rust
// Type-safe user ID (prevents type confusion)
#[derive(Debug, Clone, Copy)]
pub struct UserId(i32);

// Audit logging middleware
pub async fn audit_middleware<B>(
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let user_id = extract_user_id(&request);
    let path = request.uri().path().to_string();
    let method = request.method().to_string();

    let response = next.run(request).await;

    // Log after response
    state.audit_log.log(AuditEvent {
        user_id,
        path,
        method,
        status: response.status().as_u16(),
        timestamp: Utc::now(),
    });

    response
}
```

---

## 6. Performance Characteristics

### 6.1 Current Metrics (Python)

| Metric | Value |
|--------|-------|
| API Response (p50) | ~50ms |
| API Response (p99) | ~200ms |
| Memory Usage | ~500MB |
| Max Concurrent Users | ~1,000 |
| Cold Start | ~3s |

### 6.2 Expected Rust Improvements

| Metric | Python | Rust (Expected) | Improvement |
|--------|--------|-----------------|-------------|
| API Response (p50) | 50ms | 5ms | 10x |
| API Response (p99) | 200ms | 30ms | 6x |
| Memory Usage | 500MB | 100MB | 5x |
| Max Concurrent | 1,000 | 10,000+ | 10x |
| Cold Start | 3s | 50ms | 60x |

---

## 7. Migration Strategy

### 7.1 Phased Approach

```
Phase 1: Foundation (Weeks 1-6)
├── axum server setup
├── sqlx database integration
├── JWT authentication
└── Basic CRUD (documents, tags)

Phase 2: Business Logic (Weeks 7-14)
├── Document processing
├── Comment system
├── Workflows
└── Notifications

Phase 3: Advanced (Weeks 15-22)
├── AI/LLM integration
├── OpenSearch
├── Analytics
└── Clustering

Phase 4: External (Weeks 23-28)
├── OCR services
├── Korean NLP
├── Export services
└── Git integration

Phase 5: Real-time (Weeks 29-34)
├── WebSocket server
├── Collaborative editing
└── Real-time notifications

Phase 6: Integration (Weeks 35-40)
├── Frontend integration
├── Performance testing
├── Security audit
└── Documentation
```

### 7.2 Coexistence Period

```
                    ┌─────────────┐
                    │   Nginx     │
                    │   (Proxy)   │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼─────┐ ┌────▼────┐ ┌─────▼─────┐
        │  Python   │ │  Rust   │ │  Python   │
        │  (Legacy) │ │  (New)  │ │  (Legacy) │
        │  /api/ai  │ │/api/docs│ │  /api/ocr │
        └───────────┘ └─────────┘ └───────────┘
              │            │            │
              └────────────┼────────────┘
                           │
                    ┌──────▼──────┐
                    │  PostgreSQL │
                    │  (Shared)   │
                    └─────────────┘
```

---

## 8. Recommendations

### 8.1 Immediate Actions

1. **Initialize Rust project** with recommended structure
2. **Set up CI/CD** for both Python and Rust
3. **Create database migrations** compatible with both
4. **Define API contracts** using OpenAPI

### 8.2 Technical Decisions

| Decision | Recommendation | Rationale |
|----------|----------------|-----------|
| Web Framework | axum | Modern, tokio-native |
| Database | sqlx | Compile-time SQL checks |
| Serialization | serde | Industry standard |
| Error Handling | thiserror + anyhow | Type-safe errors |
| Async Runtime | tokio | Most mature |
| WebSocket | tokio-tungstenite | Native tokio |

### 8.3 Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Korean NLP | FFI to MeCab + fallback |
| PDF Generation | wkhtmltopdf subprocess |
| Team Learning | Pair programming, workshops |
| Performance Goals | Continuous benchmarking |

---

## 9. Documents Created

| Document | Purpose |
|----------|---------|
| `docs/ARCHITECTURE_ANALYSIS.md` | This document |
| `docs/RUST_MIGRATION_PLAN.md` | Detailed migration plan |
| `docs/UIUX_IMPROVEMENT_PLAN.md` | Frontend improvements |

---

## 10. Conclusion

MinKy is a well-architected Flask application with:
- 140+ API endpoints
- 12 data models
- 14 specialized services
- 435+ security fixes applied

Rust migration is feasible over 40 weeks, with expected:
- 10x performance improvement
- 5x memory reduction
- Compile-time safety guarantees
- Better concurrency handling

The modular design translates well to Rust's trait-based patterns, making gradual migration viable.
