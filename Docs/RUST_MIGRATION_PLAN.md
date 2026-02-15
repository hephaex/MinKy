# MinKy: Python to Rust Migration Plan

**Version:** 1.0
**Date:** 2026-02-15
**Status:** Planning Phase

## Executive Summary

MinKy는 Flask 기반 문서 관리 시스템으로, 140+ API 엔드포인트, 12개 핵심 모델, 14개 서비스로 구성됨. Rust로의 마이그레이션을 통해 성능 5-10배 향상, 메모리 50-80% 절감, 동시성 안전성 확보 가능.

---

## 1. Current Architecture Overview

### 1.1 Statistics

| 구분 | 수량 |
|------|------|
| API Endpoints | 140+ |
| SQLAlchemy Models | 12 |
| Services | 14 |
| Routes (Blueprints) | 31 |
| Frontend Components | 94 |
| Python LOC | ~15,000 |
| Dependencies | 56 |

### 1.2 Complexity Distribution

```
┌─────────────────────────────────────────────────────────────┐
│                  ENDPOINT COMPLEXITY                         │
├───────────┬───────────┬───────────┬───────────┬─────────────┤
│  Simple   │ Database  │ External  │   ML/AI   │  Real-time  │
│   CRUD    │  Queries  │ Services  │  Services │  WebSocket  │
├───────────┼───────────┼───────────┼───────────┼─────────────┤
│    35     │    28     │    18     │    22     │     15      │
│ endpoints │ endpoints │ endpoints │ endpoints │  endpoints  │
└───────────┴───────────┴───────────┴───────────┴─────────────┘
```

---

## 2. Technology Stack Mapping

### 2.1 Core Framework

| Python | Rust | Maturity | Migration Effort |
|--------|------|----------|------------------|
| Flask | **axum** | ★★★★★ | Low |
| SQLAlchemy | **sqlx** / sea-orm | ★★★★☆ | Medium |
| Flask-JWT | jsonwebtoken | ★★★★★ | Low |
| Flask-SocketIO | tokio-tungstenite | ★★★★☆ | Medium |
| Flask-CORS | tower-http | ★★★★☆ | Low |
| Flask-Limiter | governor | ★★★★☆ | Low |
| Bcrypt | argon2 / bcrypt | ★★★★★ | Low |

### 2.2 Data & Search

| Python | Rust | Notes |
|--------|------|-------|
| psycopg2 | sqlx (postgres) | Native async support |
| opensearch-py | opensearch-rs | Full API coverage |
| redis-py | redis-rs | Optional caching |

### 2.3 AI/ML Services

| Python | Rust | Notes |
|--------|------|-------|
| openai | async-openai | Full API parity |
| anthropic | anthropic-rs | Claude models |
| scikit-learn | ndarray + linfa | Basic ML only |
| nltk/textblob | rust-bert / candle | Limited NLP |

### 2.4 Document Processing

| Python | Rust | Effort |
|--------|------|--------|
| Pillow | image | Low |
| PyMuPDF | pdfium-render | Medium |
| python-docx | docx-rs | Medium |
| pytesseract | tesseract-rs | High |
| WeasyPrint | printpdf + wkhtmltopdf | High |
| Markdown | comrak | Low |

### 2.5 Challenging Dependencies

| Package | Challenge | Strategy |
|---------|-----------|----------|
| konlpy | Korean NLP | mecab-rs + custom lexicon |
| WeasyPrint | PDF generation | wkhtmltopdf FFI |
| google-cloud-vision | OCR | HTTP API direct |
| orgparse | Org-roam | Custom parser |

---

## 3. Migration Phases

### Phase 1: Foundation (4-6 weeks)

```
목표: 핵심 인프라 구축

├── axum 웹 서버 설정
├── sqlx + PostgreSQL 연결
├── JWT 인증 미들웨어
├── 기본 CRUD 엔드포인트
│   ├── /api/documents (5 endpoints)
│   ├── /api/tags (6 endpoints)
│   └── /api/categories (8 endpoints)
├── 에러 핸들링 통일
└── 로깅 및 트레이싱 설정

예상 성과:
- 기본 API 동작 확인
- 10x 응답 속도 향상
- 타입 안전성 확보
```

**Rust 구조 예시:**

```rust
// src/main.rs
use axum::{Router, routing::{get, post}};
use sqlx::postgres::PgPool;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let pool = PgPool::connect(&env::var("DATABASE_URL").unwrap()).await?;

    let app = Router::new()
        .nest("/api/documents", document_routes())
        .nest("/api/tags", tag_routes())
        .nest("/api/auth", auth_routes())
        .layer(CorsLayer::permissive())
        .with_state(AppState { pool });

    axum::serve(listener, app).await?;
}

// src/routes/documents.rs
pub fn document_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_documents).post(create_document))
        .route("/:id", get(get_document).put(update_document).delete(delete_document))
}
```

### Phase 2: Business Logic (6-8 weeks)

```
목표: 핵심 비즈니스 로직 구현

├── Document Processing
│   ├── Markdown → HTML 변환 (comrak)
│   ├── HTML Sanitization (ammonia)
│   └── 버전 관리 및 diff
├── Comment System
│   ├── 중첩 댓글 (recursive CTE)
│   └── Soft delete
├── Notification System
│   ├── 알림 생성/조회
│   └── 환경설정 관리
├── Workflow Engine
│   ├── 상태 머신 구현
│   └── 리뷰어 할당
└── Attachment Handling
    ├── 파일 업로드 (multer)
    └── 썸네일 생성 (image)

예상 성과:
- 50% 메모리 절감
- 컴파일 타임 검증
```

**타입 안전 워크플로우 예시:**

```rust
// src/models/workflow.rs
#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(type_name = "workflow_status", rename_all = "snake_case")]
pub enum WorkflowStatus {
    Draft,
    PendingReview,
    InReview,
    Approved,
    Rejected,
    Published,
    Archived,
}

#[derive(Debug, Clone)]
pub enum WorkflowAction {
    SubmitForReview,
    StartReview,
    Approve,
    Reject,
    RequestChanges,
    Publish,
    Archive,
    Withdraw,
}

impl WorkflowStatus {
    pub fn valid_actions(&self) -> Vec<WorkflowAction> {
        match self {
            Self::Draft => vec![WorkflowAction::SubmitForReview],
            Self::PendingReview => vec![WorkflowAction::StartReview, WorkflowAction::Withdraw],
            Self::InReview => vec![WorkflowAction::Approve, WorkflowAction::Reject, WorkflowAction::RequestChanges],
            // ... compile-time exhaustive matching
        }
    }
}
```

### Phase 3: Advanced Services (6-8 weeks)

```
목표: AI/ML 및 검색 기능

├── LLM Provider Abstraction
│   ├── OpenAI (async-openai)
│   ├── Anthropic (anthropic-rs)
│   └── Provider trait 정의
├── OpenSearch Integration
│   ├── 인덱싱 서비스
│   ├── 전문 검색
│   └── 자동완성
├── Document Clustering
│   ├── TF-IDF 구현 (ndarray)
│   ├── 코사인 유사도
│   └── K-means 클러스터링
└── Analytics Service
    ├── 대시보드 메트릭
    └── 사용자 통계

예상 성과:
- AI 응답 지연 30% 감소
- 검색 처리량 5x 향상
```

**LLM Provider Trait 예시:**

```rust
// src/services/llm/mod.rs
use async_trait::async_trait;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn generate(&self, prompt: &str, options: GenerateOptions) -> Result<String, LLMError>;
    async fn suggest_tags(&self, content: &str) -> Result<Vec<String>, LLMError>;
    async fn suggest_title(&self, content: &str) -> Result<String, LLMError>;
    fn provider_name(&self) -> &'static str;
}

pub struct OpenAIProvider { client: async_openai::Client }
pub struct AnthropicProvider { client: anthropic::Client }

// 컴파일 타임에 인터페이스 준수 검증
```

### Phase 4: External Integrations (4-6 weeks)

```
목표: 외부 서비스 연동

├── OCR Service
│   ├── Tesseract FFI (tesseract-rs)
│   ├── Google Vision (HTTP API)
│   └── 이미지 전처리
├── Korean NLP
│   ├── MeCab 바인딩 (mecab-rs)
│   └── 형태소 분석기
├── Export Services
│   ├── PDF (printpdf + wkhtmltopdf)
│   ├── DOCX (docx-rs)
│   └── Markdown bundle
└── Git Integration
    ├── git2-rs 라이브러리
    └── 커밋 히스토리

예상 성과:
- OCR 처리 2x 향상
- 안정적인 FFI 바인딩
```

### Phase 5: Real-time Features (4-6 weeks)

```
목표: 실시간 협업 기능

├── WebSocket Server
│   ├── tokio-tungstenite
│   ├── 연결 관리
│   └── 메시지 브로드캐스트
├── Collaborative Editing
│   ├── Operation Transform (OT)
│   ├── 커서 동기화
│   └── 충돌 해결
├── Real-time Notifications
│   └── 푸시 알림 시스템
└── Session Management
    ├── Redis 세션 저장
    └── 동시 편집자 추적

예상 성과:
- 동시 편집자 수 10x 증가
- 지연 시간 80% 감소
```

**Rust 협업 서비스 예시:**

```rust
// src/services/collaboration.rs
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct CollaborationService {
    sessions: RwLock<HashMap<DocumentId, DocumentSession>>,
}

impl CollaborationService {
    pub async fn handle_operation(
        &self,
        doc_id: DocumentId,
        op: TextOperation,
        user_id: UserId,
    ) -> Result<(), CollabError> {
        // Rust의 소유권 시스템이 동시성 안전성을 컴파일 타임에 보장
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(&doc_id)
            .ok_or(CollabError::SessionNotFound)?;

        session.apply_operation(op, user_id).await?;
        Ok(())
    }
}
```

### Phase 6: Integration & Testing (4 weeks)

```
목표: 통합 및 최적화

├── Frontend Integration
│   ├── API 호환성 검증
│   ├── WebSocket 클라이언트 업데이트
│   └── 에러 핸들링 통일
├── Performance Testing
│   ├── 벤치마크 (criterion)
│   ├── 부하 테스트
│   └── 메모리 프로파일링
├── Security Audit
│   └── 기존 435+ 보안 패턴 적용
└── Documentation
    ├── API 문서 (utoipa)
    └── 배포 가이드
```

---

## 4. Project Structure

```
minky-rust/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── documents.rs
│   │   ├── tags.rs
│   │   ├── comments.rs
│   │   ├── auth.rs
│   │   ├── admin.rs
│   │   ├── ai.rs
│   │   ├── search.rs
│   │   ├── workflows.rs
│   │   └── websocket.rs
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── document.rs
│   │   ├── tag.rs
│   │   ├── comment.rs
│   │   ├── workflow.rs
│   │   └── notification.rs
│   │
│   ├── services/
│   │   ├── mod.rs
│   │   ├── ai/
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs
│   │   │   ├── openai.rs
│   │   │   └── anthropic.rs
│   │   ├── collaboration.rs
│   │   ├── search.rs
│   │   ├── ocr.rs
│   │   ├── export.rs
│   │   └── analytics.rs
│   │
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── rate_limit.rs
│   │   └── security.rs
│   │
│   └── utils/
│       ├── mod.rs
│       ├── markdown.rs
│       ├── validation.rs
│       └── datetime.rs
│
├── migrations/
│   └── *.sql
│
└── tests/
    ├── integration/
    └── unit/
```

---

## 5. Dependencies (Cargo.toml)

```toml
[package]
name = "minky"
version = "2.0.0"
edition = "2021"

[dependencies]
# Web Framework
axum = { version = "0.7", features = ["ws", "multipart"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "compression-gzip"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "postgres", "chrono", "uuid", "json"] }

# Authentication
jsonwebtoken = "9"
argon2 = "0.5"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async
async-trait = "0.1"
futures = "0.3"

# WebSocket
tokio-tungstenite = "0.21"

# Search
opensearch = "2"

# AI/LLM
async-openai = "0.18"

# Document Processing
comrak = { version = "0.18", features = ["syntect"] }
ammonia = "3"
image = "0.24"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
thiserror = "1"
anyhow = "1"
dotenvy = "0.15"

# Rate Limiting
governor = "0.6"

# Validation
validator = { version = "0.16", features = ["derive"] }

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"
proptest = "1"
```

---

## 6. Migration Timeline

```
2026 Q1 (현재)
├── Week 1-2: 프로젝트 설정, 기본 구조
├── Week 3-4: Phase 1 시작 (Foundation)
├── Week 5-6: Phase 1 완료, 테스트

2026 Q2
├── Week 1-4: Phase 2 (Business Logic)
├── Week 5-8: Phase 2 완료
├── Week 9-12: Phase 3 (Advanced Services)

2026 Q3
├── Week 1-4: Phase 3 완료
├── Week 5-8: Phase 4 (External Integrations)
├── Week 9-12: Phase 5 (Real-time)

2026 Q4
├── Week 1-4: Phase 6 (Integration)
├── Week 5-8: 성능 최적화
├── Week 9-12: 프로덕션 배포
```

---

## 7. Risk Assessment

### High Risk
| 리스크 | 영향 | 완화 전략 |
|--------|------|-----------|
| Korean NLP 호환성 | 검색 품질 저하 | MeCab FFI + 폴백 |
| WeasyPrint 대체 | PDF 품질 차이 | wkhtmltopdf + 테스트 |
| 실시간 협업 복잡도 | 기능 지연 | 단계적 구현 |

### Medium Risk
| 리스크 | 영향 | 완화 전략 |
|--------|------|-----------|
| 팀 Rust 학습 곡선 | 개발 속도 | 페어 프로그래밍 |
| FFI 안정성 | 런타임 크래시 | 철저한 테스트 |

### Low Risk
| 리스크 | 영향 | 완화 전략 |
|--------|------|-----------|
| API 호환성 | 프론트엔드 수정 | OpenAPI 검증 |
| 성능 목표 미달 | 기대치 조정 | 벤치마크 기반 |

---

## 8. Success Metrics

| 메트릭 | 현재 (Python) | 목표 (Rust) |
|--------|---------------|-------------|
| API 응답 시간 (p99) | ~200ms | <30ms |
| 메모리 사용량 | ~500MB | <150MB |
| 동시 연결 | ~1,000 | >10,000 |
| CPU 사용률 | 40-60% | 10-20% |
| 빌드 시간 | N/A | <2min |
| 콜드 스타트 | ~3s | <100ms |

---

## 9. Coexistence Strategy

마이그레이션 기간 동안 Python과 Rust 서비스 공존:

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
        │  (Legacy) │ │ (New)   │ │  (Legacy) │
        │  /api/ai  │ │/api/docs│ │  /api/ocr │
        └───────────┘ └─────────┘ └───────────┘
              │            │            │
              └────────────┼────────────┘
                           │
                    ┌──────▼──────┐
                    │  PostgreSQL │
                    └─────────────┘
```

- 엔드포인트별 점진적 마이그레이션
- 동일 DB 공유
- Feature flag로 트래픽 전환

---

## 10. Next Steps

1. **즉시**: Rust 프로젝트 초기화 (`cargo new minky-rust`)
2. **1주차**: 기본 axum 서버 + sqlx 연결
3. **2주차**: `/api/documents` CRUD 구현
4. **3주차**: JWT 인증 + 보안 미들웨어
5. **4주차**: 첫 번째 벤치마크 및 비교

---

## Appendix: Key Rust Patterns for MinKy

### A. Error Handling

```rust
// src/error.rs
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into()),
        };

        (status, axum::Json(serde_json::json!({ "error": message }))).into_response()
    }
}
```

### B. Type-Safe User ID

```rust
// src/models/user.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, serde::Serialize, serde::Deserialize)]
#[sqlx(transparent)]
pub struct UserId(i32);

impl UserId {
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    pub fn as_i32(&self) -> i32 {
        self.0
    }
}

// 타입 혼동 방지: UserId와 DocumentId를 섞어 사용 불가
```

### C. Async Service Pattern

```rust
// src/services/document_service.rs
use std::sync::Arc;

pub struct DocumentService {
    pool: Arc<PgPool>,
    search: Arc<SearchService>,
}

impl DocumentService {
    pub async fn create(&self, input: CreateDocument, user_id: UserId) -> Result<Document, AppError> {
        let doc = sqlx::query_as!(
            Document,
            r#"INSERT INTO documents (title, content, user_id) VALUES ($1, $2, $3) RETURNING *"#,
            input.title,
            input.content,
            user_id.as_i32()
        )
        .fetch_one(&*self.pool)
        .await?;

        // 비동기 인덱싱
        self.search.index_document(&doc).await?;

        Ok(doc)
    }
}
```
