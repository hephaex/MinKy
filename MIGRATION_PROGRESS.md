# Minky 백엔드 통합 — 진행 로그

> 실행서: `MIGRATION_PLAN.md`. 슬라이스 완료마다 아래에 append (§10 컨벤션).
> 상태 태그: [DONE] = 로컬 Demo Gate(컴파일+테스트 green) / [CAS-deferred] = 실DB·실배포 검증 대기 / [BLOCKED] = 에스컬레이트.

---

## Phase 0 — Auth 호환 [🟡]

### Slice: Claims.sub i32→String + Flask 호환 테스트 — 2026-06-10

- 변경 파일:
  - `minky-rust/src/services/auth_service.rs` — Claims.sub: String, generators: user.id.to_string(), 기존 round-trip 테스트 수정, Flask-compat 신규 테스트 추가
  - `minky-rust/src/middleware/extractor.rs` — AuthUser/OptionalAuthUser: claims.sub.parse::<i32>()
  - `minky-rust/src/middleware/auth.rs` — 주석 업데이트 (타입 변경 반영)
  - `MIGRATION_PLAN.md` — §6 검증칸 완성
- 테스트: Rust `1843 passed` (`SQLX_OFFLINE=true cargo test --lib`) — 이전 대비 +1 (Flask-compat test)
- 커밋: `169e1689`
- 상태: [DONE] [CAS-deferred: 실 Flask 토큰으로 Rust 검증 엔드투엔드는 CAS 복구 후]

**§6 검증 결과 요약:**
- 토큰 전송: **없음** (현 프로덕션은 사실상 무인증 — no login UI, no token storage, optional=True)
- Flask sub: `str(user.id)` HS256 headers 위치
- Rust extractor: Bearer 헤더 → cookie `access_token` fallback (이미 구현됨)
- 이 슬라이스로 Flask 발급 토큰을 Rust가 파싱 가능해짐 (sub 타입 불일치 해소)

**Phase 0→1 핸드오프 준비 완료.** Opus 확인 항목: §11 Phase 0→1 경계 참조.

## Phase 1 — DB-free 배선 증명 (query expansion) [🟢]

### Slice: nginx /api/hybrid/ → Rust 배선 + UPSTAGE_API_KEY 전달 — 2026-06-11

- 변경 파일:
  - `frontend/nginx.conf` — `location /api/hybrid/` 블록 추가 (rust-backend:8000, `/api/` catch-all 앞)
  - `docker-compose.cas.yml` — Rust 서비스에 `UPSTAGE_API_KEY=${UPSTAGE_API_KEY:-}` 추가
- 경로 정정: 계획서 `/api/search/expand` → 실제 `POST /api/hybrid/expand` (routes/hybrid.rs 기준)
- Flask 상당 엔드포인트: **없음** → 결정 A(형태 변환) 불필요, Rust 응답 그대로 노출
- 인증: hybrid/expand는 AuthUser extractor 없음 — 무인증 OK (DB-free LLM-only)
- 테스트: `1843 passed` (SQLX_OFFLINE=true, query_expansion 14건 포함)
- 커밋: `be164be3`
- 상태: [DONE] [CAS-deferred: nginx 실구동은 CAS 복구 후]

**Demo Gate 달성:**
- Rust 통합테스트 1843 green ✅
- nginx config: `/api/hybrid/` 블록이 `/api/` 보다 먼저 (longest-match prefix 우선) ✅
- `[CAS-deferred]` 실호출: CAS 복구 후 `curl -X POST nginx:3000/api/hybrid/expand` 검증

## Phase 2 — 데이터 소유 도메인 (결정 C = A1 이관) [🟡]

### Slice: 이관 자산 (migration 011 + 스크립트 + 단위테스트) — 2026-06-11

- 변경 파일:
  - `minky-rust/migrations/011_flask_compat_columns.sql` — additive columns + flask_document_id_mapping 테이블
  - `scripts/migrate_flask_to_rust.py` — 이관 스크립트 (순수 함수 + DB 함수)
  - `scripts/verify_migration.py` — row-count/hash/orphan 검증 쿼리
  - `scripts/tests/test_migrate.py` — 단위 테스트 39건 (DB 없이)
- 테스트: Python `39 passed` (`python -m pytest scripts/tests/test_migrate.py`) + Rust `1843 passed` (SQLX_OFFLINE=true)
- 커밋: `50ff9e1d`
- 상태: [DONE] [CAS-deferred: 실 이관 실행 + row-count/hash/orphan 검증 → CAS 복구 후 Mario 승인 필수]

**Phase 2 Demo Gate (로컬):**
- 스크립트 단위테스트 39건 green ✅
- Rust 1843 tests green (migration 011은 additive, SQLX_OFFLINE 무관) ✅
- [CAS-deferred] 실 이관·검증: `DRY_RUN=1 python scripts/migrate_flask_to_rust.py` → 통과 확인 후 Mario 승인 받고 실행

**이관 자산 요약:**
| 항목 | 내용 |
|------|------|
| UUID 매핑 | UUIDv5(namespace=`1d6b1000...`, `minky:document:{id}`) — 결정적·멱등 |
| 필드 매핑 | `markdown_content` → `content`, `is_admin` → `role enum`, NULL user_id → default |
| additive 컬럼 | documents 5개, categories 5개, tags 3개, document_versions 5개 |
| 원본 보호 | minky DB SELECT-only (ALTER/DROP/UPDATE/DELETE 없음) |
| 실행 순서 | users → categories → tags → documents → document_tags → comments → document_versions |

### Slice: 결정A 적용 (Flask-compat response shape + nginx 라우팅) — 2026-06-11

- 변경 파일:
  - `minky-rust/src/routes/documents.rs` — Flask-compat 응답 타입 추가 + 핸들러 반환 타입 변경
  - `frontend/nginx.conf` — `/api/documents` 블록 → rust-backend:8000 추가
- 변경 내용:
  - `FlaskDocumentItem`: `markdown_content` (content 필드 rename), additive 컬럼, `tag_names: Vec<String>`
  - `FlaskListResponse`: `{documents, pagination{page,per_page,total,pages}}` — frontend 기대 형태와 일치
  - 단일 문서(get/create/update): `{success, data}` 래퍼 제거, 직접 dict 반환 (Flask와 동일)
  - `ListQuery.per_page` 추가 (frontend가 `per_page` 파라미터 전송)
  - `CreateDocumentRequest`, `UpdateDocumentRequest`: `markdown_content` 필드 (frontend가 보내는 키)
  - nginx: `/api/documents` 블록이 `/api/` Flask catch-all보다 앞에 위치
- 테스트: Rust `1850 passed` (SQLX_OFFLINE=true, +7 Flask-compat tests)
- 커밋: `53fb5c66`
- 상태: [DONE] [CAS-deferred: nginx 실구동 + Flask→Rust E2E 토큰 검증 → CAS 복구 후]

**Phase 2 Demo Gate (로컬) — 전체 완료:**
- migration 011 (additive columns) ✅
- 이관 스크립트 단위테스트 39건 green ✅
- Flask-compat response shape (Decision A) 구현 ✅
- nginx `/api/documents` 블록 추가 ✅
- Rust 1850 tests green ✅

**프론트엔드 호환성 보장:**
| 프론트 기대값 | Rust 응답 |
|-------------|---------|
| `response.data.documents` | `FlaskListResponse.documents` ✅ |
| `response.data.pagination.per_page` | `FlaskPagination.per_page` ✅ |
| `response.data.pagination.pages` | `FlaskPagination.pages` ✅ |
| `data.markdown_content` (single GET) | `FlaskDocumentItem.markdown_content` ✅ |
| request body `markdown_content` (POST/PUT) | `CreateDocumentRequest.markdown_content` ✅ |

### Slice: Opus 리뷰 수정 (FlaskDocumentLite + consumer-contract + nginx M1 + H3) — 2026-06-11

- 변경 파일:
  - `minky-rust/src/routes/documents.rs` — FlaskDocumentLite/FlaskDocumentLiteRow 분리, FlaskPagination has_next/has_prev, list SQL LEFT JOIN+ARRAY_AGG, consumer-contract 테스트 17건 추가
  - `frontend/nginx.conf` — /sync, /export, /tree 명시 Flask 블록 (M1 fix)
  - `scripts/migrate_flask_to_rust.py` — is_published/is_public None 명시 coercion (H3 fix)
  - `scripts/tests/test_migrate.py` — None coercion 테스트 5건 추가 (총 44건)
- 수정 항목:
  - C1: tag_names JOIN → ARRAY_AGG(t.name) FILTER(NOT NULL) from document_tags+tags
  - C2: FlaskDocumentLite — 리스트에서 markdown_content/html_content 제거
  - C3: FlaskPagination has_next/has_prev 추가 (Pagination.js PropTypes.isRequired)
  - H1: consumer-contract 테스트 — Flask to_dict_lite 키 + Pagination.js 실소비 키 대조
  - H3: None coercion (dict.get(k,default) → `v if v is not None else default`)
  - M1: nginx /sync|export|tree → Flask 명시 블록, /api/documents prefix 앞에 배치
- 테스트: Rust `1861 passed` (SQLX_OFFLINE=true, +11) + Python `44 passed` (+5)
- 커밋: `457edb61`
- 상태: [DONE] [CAS-deferred: nginx 실구동 + 이관 실행 → CAS 복구 후]

**Demo Gate:**
- Rust 1861 tests green ✅
- Python 44 tests green ✅
- consumer-contract 테스트가 FlaskDocumentLite와 Pagination.js 키 일치 보장 ✅
- nginx prefix 순서: /sync|export|tree(Flask) → /api/documents(Rust) → /api/(Flask) ✅

**프론트엔드 호환성 (갱신):**
| 프론트 기대값 | Rust 응답 |
|-------------|---------|
| `doc.author`, `doc.updated_at`, `doc.title`, `doc.id` | `FlaskDocumentLite.*` ✅ |
| `doc.tag_names` (string array) | `ARRAY_AGG(t.name)` ✅ |
| `pagination.has_next`, `pagination.has_prev` | `FlaskPagination.has_next/has_prev` ✅ |
| 리스트에 `markdown_content` 없음 | `FlaskDocumentLite` body 없음 ✅ |

## Phase 3 — Flask-only 도메인 분류 [🟢]

### 분류 완료 — 2026-06-12

**분류 기준**: 프론트 실 API 호출 여부 × 컴포넌트 실 렌더링 여부 × Rust 라우트 존재 여부

#### A. 계획상 "Flask-only 5개" 분류 결과

| 도메인 | 프론트 호출 경로 | 컴포넌트 렌더? | Rust 라우트 | 판정 |
|--------|--------------|-------------|------------|------|
| **org_roam** | 0건 | N/A | 없음 | `[DROP]` — 호출 없음, 안전 폐기 |
| **clustering** | `DocumentClustering.js`→`/api/clustering/*` | ❌ 어느 페이지도 임포트 안 함 (orphan) | `/ml/clustering` (경로 불일치) | `[DROP-ORPHAN]` — 컴포넌트 미연결, 폐기 가능 |
| **ml-analytics** | `MLAnalytics.js`→`/api/ml-analytics/*` | ❌ 어느 페이지도 임포트 안 함 (orphan) | `/ml/*` (경로 불일치 `/ml-analytics`≠`/ml`) | `[DROP-ORPHAN]` — 컴포넌트 미연결, 폐기 가능 |
| **collaboration** | `CollaborativeEditor.js` (socket.io WebSocket) | ✅ `DocumentEdit.js` → App.js | Rust: `ws.rs` → `/ws` (axum WebSocket, 단일 엔드포인트) | `[ASSESS]` — Flask socket.io ≠ Rust `/ws`. 기능 차이 분석 필요 (Mario 승인 후 방향 결정) |
| **chat** | `chatService.js`→`/api/chat/{message,sessions}` | ✅ `ChatPage` → App.js `/chat` | ❌ 없음 (mod.rs에 chat 없음) | `[FLASK-STAY]` — Rust 구현 없음. Flask 유지 (Phase F까지) |

#### B. "Rust 있지만 nginx가 Flask로 보내는" 도메인 — 추가 발견

현재 nginx catch-all(`/api/ → Flask`)이 Rust 구현이 있는 도메인도 Flask로 보내고 있음.
이 도메인들은 Phase 3 범위 외이나, Phase F 준비를 위해 목록화:

| Rust 라우트 | 프론트 호출 | 전환 조건 | 우선순위 |
|-----------|-----------|---------|---------|
| `/auth/*` | ✅ login/logout/me/refresh | Flask JWT_SECRET_KEY ↔ Rust JWT_SECRET 통일 필요 (§7 함정) | 🔴 Mario 확인 |
| `/categories/*` | ✅ categories CRUD | Flask compat 응답 형태 확인 필요 | 🟡 |
| `/tags/*` | ✅ tags CRUD + statistics | Flask compat 확인 필요 | 🟡 |
| `/ai/*` | ✅ suggestions/autocomplete | UPSTAGE_API_KEY 전달 확인 (Phase 1에서 완료) | 🟡 |
| `/ocr/*` | ✅ OCRPage, ImportPage, DocumentCreate | ocr.rs 구현 상태 확인 필요 | 🟡 |
| `/knowledge/*` | ✅ KnowledgeSearch, KnowledgeGraphPage | knowledge.rs 구현 상태 확인 필요 | 🟡 |
| `/analytics/*` | ✅ AnalyticsDashboard | analytics.rs 확인 필요 | 🟡 |
| `/git/*` | ✅ GitSettings | git.rs 확인 필요 | 🟡 |
| `/admin/*` | ✅ AdminPanel | admin.rs 확인 필요 | 🟡 |
| `/search/*` | ✅ search/ask, search/semantic | search.rs 확인 필요 | 🟡 |

#### C. 핵심 발견 — 경로 불일치 2건

- **Frontend → `/api/clustering/*`** but **Rust → `/api/ml/clustering`** (nginx 전환 시 404)
- **Frontend → `/api/ml-analytics/*`** but **Rust → `/api/ml/*`** (nginx 전환 시 404)
- 두 컴포넌트 모두 orphan → nginx 전환 불필요, 현 상태 유지로 충분

#### D. 다음 단계 제안 (Mario 확인 필요)

현재 Phase 3 분류 완료. 남은 전환 작업:
1. **auth 도메인 전환** (🔴) — JWT 시크릿 통일(`JWT_SECRET_KEY`) + nginx `/api/auth → Rust` 전환
2. **categories/tags/search 전환** (🟡) — Flask compat 응답 확인 후 nginx 추가
3. **collaboration** (🟡 ASSESS) — socket.io(Flask) vs WebSocket(Rust `/ws`) 기능 비교
4. **chat** (Flask 유지) — Phase F에서 Rust 구현 여부 결정

**커밋**: 코드 변경 없음 (grep+분류 only), 이 문서만 업데이트

## Phase F — Flask 폐기 [🔴 Mario 승인]
_(대기)_
