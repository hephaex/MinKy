# MinKy Development Plan

> 이 파일은 해야 할 작업들을 관리합니다.
> 에이전트는 세션 시작 시 이 파일을 읽고, 작업 추가/완료 시 업데이트합니다.

---

## 🎯 다음 세션 우선 작업

### ✅ 보안 이슈 수정 완료 (2026-02-20)

**우선순위 1 - 즉시 수정 필요: ✅ 완료**
- [x] **Document 엔드포인트 인증 추가** (Critical 3건) - 커밋 `1e5da9a3`
  - `GET /documents/{id}` - AuthUser 추가 + 소유권 확인
  - `PUT /documents/{id}` - AuthUser 추가 + 소유권 확인
  - `DELETE /documents/{id}` - AuthUser 추가 + 소유권 확인

**우선순위 2 - 프로덕션 전 수정: ✅ 완료**
- [x] **List 엔드포인트 인증 추가** (High) - 커밋 `1e5da9a3`
  - `GET /documents` - 사용자 소유 문서 + 공개 문서만 반환
- [x] **JWT 저장 방식 개선** (High) - 커밋 `28fd3bbf`
  - localStorage → HttpOnly 쿠키로 변경
  - XSS 공격 방지 완료

---

### ✅ Phase 1-3 완료! Phase 4 준비

**Phase 1 (Knowledge Understanding) 완료:**
- ✅ pgvector 설정 및 마이그레이션
- ✅ Document Understanding 파이프라인
- ✅ 벡터 임베딩 API (7개 엔드포인트)
- ✅ RAG 검색 API (3개 엔드포인트)
- ✅ 프론트엔드 검색 UI
- ✅ 프론트엔드 채팅 UI
- ✅ API 문서화

**Phase 2 (Real-time Features) 완료:**
- ✅ WebSocket 실시간 연결 (/ws)
- ✅ Chat 스트리밍 응답 (SSE)
- ✅ 프론트엔드 스트리밍 UI

**Phase 3 (Knowledge Graph Enhancement) 완료 (2026-02-24):**
- ✅ Path Finding - BFS 최단 경로 탐색 (커밋 `b2829e36`)
- ✅ Cluster Analysis - Label Propagation 알고리즘 (커밋 `acba9ad5`)
- ✅ Timeline View - 날짜 범위 필터링 (커밋 `a04a78de`)
- ✅ Graph Export - JSON/CSV 내보내기 (커밋 `be39fef8`)

**Phase 4 (Frontend Enhancement) 완료 (2026-02-24):**
- ✅ 지식 그래프 상세 패널 강화 (커밋 `7c9ea5b9`)
  - 생성일 표시 (상대 시간 + 툴팁)
  - 노드 통계 (연결 수, 평균 강도, 문서 수)
  - 클러스터 정보 표시
  - 빠른 액션 (경로 탐색, 연결 필터, 내보내기)
- ✅ 문서 목록 대시보드 개선 (커밋 `0930033f`, `9956d3c3`)
  - 정렬 옵션 추가 (최근/오래된 업데이트, 제목 A-Z/Z-A)
  - 그리드/리스트 뷰 토글 (localStorage 저장)
  - 태그 필터 추가 (멀티 선택, 태그별 문서 수 표시)
- ✅ 검색 결과 하이라이팅 개선 (커밋 `fb2a8daf`)
  - CSS 클래스명 통일 (.search-highlight)
  - SearchResultItem에서 중복 로직 제거, 유틸리티 함수 사용
  - SourceDocuments에 검색어 하이라이팅 추가
- ✅ 채팅 UI 개선 (커밋 `c3374b02`)
  - 소스 카드 클릭 시 문서로 이동 가능
  - 복사 버튼 애니메이션 (체크마크 아이콘)
  - 글자 수 제한 진행 바 표시

**Phase 5 (Production Readiness) 완료 (2026-02-24):**
- ✅ 환경 변수 보안화 (커밋 `a8129087`)
  - docker-compose.yml에서 하드코딩된 비밀번호 제거
  - ${VAR:?required} 문법으로 필수 변수 체크
  - .env.example 포괄적 문서화
- ✅ Redis 기반 Rate Limiting (커밋 `a8129087`)
  - RateLimiterBackend trait 추상화
  - RedisRateLimiter (프로덕션), InMemoryRateLimiter (개발)
  - REDIS_URL 환경변수로 자동 선택
- ✅ DB 커넥션 풀 프로덕션 설정 (커밋 `a8129087`)
  - min_connections, acquire_timeout, max_lifetime, idle_timeout
- ✅ Health 엔드포인트 확장 (커밋 `a8129087`)
  - /api/health/ready (K8s readiness probe)
  - /api/health/live (K8s liveness probe)
  - DB 풀 통계, Redis 상태, 응답 시간 측정
- ✅ Redis 프로덕션 설정 (커밋 `a8129087`)
  - 메모리 제한 (256mb 기본), LRU 퇴거 정책, AOF 영속성

**E2E 테스트 결과:**
1. [x] PostgreSQL 데이터베이스 마이그레이션 실행
   - pgvector 0.8.0 소스 빌드 및 설치 완료
   - minky_rust_db 생성 및 4개 마이그레이션 적용 완료
   - 빌드: 0 errors, 0 warnings
2. [ ] OpenAI API 크레딧 부족 - 새 API 키 또는 크레딧 보충 필요
3. [x] Rust 서버 기동 확인 (/api/health 응답 정상)
4. [x] 프론트엔드-백엔드 통합 테스트 완료
5. [ ] RAG 파이프라인 E2E 테스트 (API 키/크레딧 필요)
6. [x] documents CRUD DB 연동 구현 완료

**코드 품질 개선:**
- [x] Rust clippy 경고 80개 → 0개 (type alias, Display impl, derive, allow)
- [x] Frontend 테스트 520개 통과
- [x] Rust 단위 테스트 1,465개 통과 (routes 100% 커버리지)
- [x] Auth 라우트 실제 구현 (login, register, refresh, /me)
- [x] Documents 인증 연동 (AuthUser 추출기로 user_id 교체)
- [x] AuthUser 연동 (tags, categories, comments, notifications, workflows, versions, attachments)
- [x] **ESLint 경고 592 → 0개** (2026-03-01 완료, 100% 해결)
  - PropTypes 추가 (27개 컴포넌트)
  - 접근성 개선 (jsx-a11y: onKeyDown, role, tabIndex)
  - label-input 연결 (htmlFor/id)
  - 코드 품질 수정 (no-case-declarations, eslint-disable)
  - 테스트 파일 PropTypes 규칙 비활성화

**사용 방법:**
```
/ci start     # CI/CD 세션 시작 (권장)
/pm           # PM 에이전트 시작
/validate     # 빌드/테스트/린트 검증
```

**참고 파일:**
- `PROGRESS.md` - 완료된 작업 상세 내역
- `Docs/GETTING_STARTED.md` - 설치 및 설정 가이드

---

## Current Phase: Phase 1 - Knowledge Understanding

### 목표
문서 업로드 시 AI가 자동으로 이해하고, 벡터 임베딩으로 저장하여 자연어 검색 가능하게

---

## TODO (우선순위 순)

### 🔴 High Priority - ✅ ALL COMPLETED

- [x] **pgvector 설정** ✅ (2026-02-19 완료)
  - PostgreSQL에 pgvector 확장 설치
  - 벡터 컬럼이 있는 테이블 마이그레이션 작성
  - 임베딩 모델/서비스 구현
  - 결과: migrations/003_pgvector_embeddings.sql, models/embedding.rs, services/embedding_service.rs

- [x] **Document Understanding 파이프라인** ✅ (2026-02-19 완료)
  - 문서 업로드 시 Claude로 분석
  - 핵심 주제, 요약, 인사이트 추출
  - 결과: services/understanding_service.rs, routes/understanding.rs

- [x] **벡터 임베딩 서비스** ✅ (2026-02-19 완료)
  - OpenAI text-embedding-3-small 연동 (1536 dimensions)
  - 문서/청크별 임베딩 생성 및 저장
  - 결과: routes/embeddings.rs (7개 엔드포인트)

### 🟡 Medium Priority - ✅ ALL COMPLETED

- [x] **RAG 검색 API** ✅ (2026-02-19 완료)
  - 자연어 질문 → 벡터 검색 → 컨텍스트 조합 → Claude 답변
  - `/api/search/ask`, `/api/search/semantic`, `/api/search/history`
  - 결과: models/rag.rs, services/rag_service.rs, routes/rag.rs

- [x] **시맨틱 청킹** ✅ (2026-02-19 완료)
  - 문서를 의미 단위로 분할 (ChunkEmbedding 모델)
  - 청크별 임베딩 저장 및 검색
  - 결과: chunk_embeddings 테이블, POST /api/embeddings/chunks/{id}

### 🟢 Low Priority - ✅ ALL COMPLETED

- [x] **관련 문서 자동 연결** ✅ (2026-02-19 완료)
  - 벡터 유사도 기반 관련 문서 추천
  - 결과: GET /api/embeddings/similar/{id}, RelatedDocsList 컴포넌트

- [x] **대화형 채팅 UI** ✅ (2026-02-19 완료)
  - React 채팅 인터페이스 (5개 컴포넌트)
  - 마크다운 렌더링, 코드 하이라이팅
  - 결과: frontend/src/components/Chat/, ChatPage.jsx

---

## Backlog (Phase 2+)

- [x] **WebSocket 실시간 연결** ✅ (2026-02-23 완료)
  - `/ws` 엔드포인트 구현
  - 인증된 WebSocket 연결
  - Subscribe/Unsubscribe/Ping 메시지 처리
  - 실시간 이벤트 브로드캐스트

- [x] **Chat 스트리밍 응답 (SSE)** ✅ (2026-02-23 완료)
  - POST `/api/search/ask/stream` 엔드포인트
  - Claude API 스트리밍 연동
  - 실시간 토큰 전송
  - SSE 이벤트: sources, delta, done, error

- [x] **지식 그래프 시각화** ✅ (2026-02-19 완료)
  - SVG 기반 포스-다이렉티드 그래프
  - 노드: 문서/토픽/기술/사람/인사이트 타입별 색상
  - 줌/팬, 노드 클릭 상세 패널, 타입 필터, 검색
  - 라우트: /graph
- [x] 지식 그래프 백엔드 API (GET /api/knowledge/graph) ✅ (2026-02-19 완료)
  - pgvector 코사인 유사도 기반 엣지 생성
  - Document Understanding 토픽/기술 노드 자동 생성
  - GET /api/knowledge/graph (필터: threshold, max_edges, include_topics/technologies/insights)
  - GET /api/knowledge/team (팀원 전문성 맵)
- [x] 팀원 전문성 맵핑 모델/API ✅ (2026-02-19 완료)
  - ExpertiseLevel (Beginner/Intermediate/Advanced/Expert)
  - TeamExpertiseMap, UniqueExpert
  - GET /api/knowledge/team 엔드포인트
- [x] 통합 테스트 구조 구축 ✅ (2026-02-19 완료)
  - tests/common/mod.rs - TestApp 헬퍼
  - tests/health_test.rs - 4개 통합 테스트
  - tests/knowledge_graph_model_test.rs - 11개 모델 테스트
- [x] Slack/Teams 연동 모델 및 서비스 설계 ✅ (2026-02-19 완료)
  - models/slack.rs: MessagingPlatform, PlatformMessage, ExtractedKnowledge, ExtractionStatus, MessageFilter (18 테스트)
  - services/slack_service.rs: 순수 함수 (thread 분석, 프롬프트 빌드, LLM 파싱, 필터, 분류) + ConversationStats (27 테스트)
- [x] Slack/Teams OAuth 엔드포인트 ✅ (2026-02-19 완료)
  - GET /api/slack/oauth/callback
  - POST /api/slack/confirm
  - GET /api/slack/summary
  - GET /api/slack/extract/{id}
- [x] 대화에서 지식 자동 추출 파이프라인 ✅ (2026-02-19 완료)
  - services/conversation_extraction_service.rs (LLM 호출, quality gate, 파싱)
  - POST /api/slack/extract 엔드포인트
- [x] Slack Webhook 수신 핸들러 ✅ (2026-02-19 완료)
  - POST /api/slack/webhook (url_verification + event_callback)
  - SlackWebhookPayload 타입 정의
- [x] platform_configs/extraction_jobs DB 마이그레이션 ✅ (2026-02-19 완료)
  - migrations/005_slack_platform.sql
  - platform_configs, platform_messages, extraction_jobs, extracted_knowledge 테이블
- [x] OAuth 토큰 교환 실구현 (Slack oauth.v2.access API 호출 + DB 저장) ✅ (2026-02-19 완료)
  - SlackOAuthService: exchange_code, save_workspace_credentials, validate_state
  - Config: slack_client_id, slack_client_secret, slack_redirect_uri, slack_signing_secret
- [x] Webhook 이벤트 → 자동 지식 추출 파이프라인 연결 (event_callback 처리) ✅ (2026-02-19 완료)
  - classify_webhook_action() 순수 함수
  - tokio::spawn으로 비동기 ConversationExtractionService 실행

---

## Sprint 5 완료 (2026-05-08)

- [x] Upload → EmbeddingService 큐 자동 연결 (fire-and-forget)
- [x] GET /documents/{id}/status — 문서 처리 상태 조회 (큐 + 폴백)
- [x] POST /documents/{id}/reprocess — 소유자 전용 재처리 요청
- [x] ProcessingStatus::Failed variant + Frontend red badge
- [x] 5 unit tests, clippy clean
- Commit: `1617d8df`

## Sprint 6 완료 (2026-05-08)

- [x] queue_position 실제 계산 (priority DESC, created_at ASC 기준)
- [x] EmbeddingConfig ..Default::default() 활용, EmbeddingModel import 제거
- [x] 3 unit tests (queue position, completed status, variant roundtrip)
- [x] Frontend Failed badge → clickable retry button (onReprocess prop)
- Commit: `66d5b549`

## Sprint 7 완료 (2026-05-08)

- [x] documentService에 getDocumentStatus, reprocessDocument API 추가
- [x] DocumentList → handleReprocess → refetch 연결
- [x] document_routes_test.rs — 5개 라우트 shape 통합 테스트
- Commit: `05ca1c2a`

## Sprint 8 완료 (2026-05-08)

- [x] useToast hook + Toast component (auto-dismiss, success/error/info)
- [x] DocumentList: toast 피드백 in handleReprocess
- [x] DocumentView: pending/failed badge + reprocess button + toast
- [x] DateExplorer: onReprocess prop 전달 + toast
- Commit: `7ed594a4`

## Sprint 9 완료 (2026-05-08) — Review Debt Cleanup

- [x] useToast unmount cleanup (memory leak fix)
- [x] DocumentView handleReprocess response.data double-unwrap fix
- [x] ARIA role="status" → aria-label (DocumentCard + DocumentView)
- [x] Remove deprecated defaultProps (Toast + DocumentCard)
- [x] Extract formatAuthor to utils/documentUtils.js (immutable, DRY)
- [x] Toast message prop: remove isRequired, keep null guard
- Net: -51 lines
- Commit: `ce857752`

## Sprint 10 완료 (2026-05-08)

- [x] useDocumentStatus polling hook (5s interval, auto-stop on completed/failed)
- [x] DocumentView live status badges with queue position
- [x] 16 unit tests for formatAuthor (documentUtils.test.js)
- [x] Review fixes: polling race condition, response unwrap, ARIA, type guard
- Commits: `29b0a59a`, `83903ae0`

## Sprint 11 완료 (2026-05-08) — DocumentCard Badges + List View Reprocess

- [x] DocumentCard: completed "Embedded" green badge 추가
- [x] DocumentList list view: pending/completed/failed badge + clickable retry button
- [x] Event propagation: e.preventDefault + e.stopPropagation (Link 내부 button)
- Commit: `977d61aa`

## Sprint 12 완료 (2026-05-08) — defaultProps Migration + Polling Tests

- [x] defaultProps → ES default params (17 파일, 18 사용처)
- [x] useDocumentStatus unit tests (23개, 8 describe blocks)
- Net: 438 insertions, 113 deletions
- Commit: `736c3d97`

## Sprint 13 완료 (2026-05-08) — defaultProps 완전 제거 + Lint Fix

- [x] 서브디렉토리 defaultProps 마이그레이션 (12 파일, 16 사용처) — 코드베이스 전체 0건
- [x] useDocumentStatus eslint-disable 제거 (stopPolling을 deps에 추가)
- [x] 엣지 케이스 테스트 2건 (empty string, processing status)
- Net: -66 lines
- Commit: `6ea17153`

## Sprint 14 완료 (2026-05-21) — Critical Recovery

- [x] S14-01: Frontend node_modules 복구 (npm install + 테스트 561개 재검증)
- [x] S14-02: Python 레거시 uncommitted changes stash 보존
- [x] S14-03: .env.cas 확인 (gitignore/dockerignore로 안전)
- Commit: `66068482`

## Sprint 15 완료 (2026-05-21) — Toast Portal + Auto-Polling

- [x] S15-01: Toast를 createPortal로 document.body에 렌더링
- [x] S15-02: DocumentList에서 업로드 후 자동 status polling (5초 간격, 완료 시 toast + 새로고침)
- Commit: `fc6d1f3a`

## Sprint 16 완료 (2026-05-21) — Optimistic Update + Accessibility

- [x] S16-01: DocumentView reprocess를 optimistic update로 개선
- [x] S16-02: 접근성 개선 (SearchBar, TreeSidebar, Header에 aria-label 추가)
- Commit: `2a5c9d1b`

## Sprint 17 완료 (2026-05-21) — Legacy Cleanup

- [x] S17-01: Python 레거시를 legacy/python-backend/로 아카이브
- [x] S17-02: .history/ 2025년 파일 24개를 archive-2025/로 이동
- [x] S17-03: Rust TODO 31건 분류 (아래 표 참조)

## Sprint 18 완료 (2026-05-21) — fastembed-rs 로컬 임베딩

- [x] S18-01: fastembed v5 Cargo.toml 추가 + migration 009 (nomic_embed_text_v1_5 enum)
- [x] S18-02: NomicEmbedTextV15 variant (768 dim) + EmbeddingService::new_with_local()
- [x] S18-03: AppState에 Arc<EmbeddingService> 통합 (HIGH 이슈 수정)
- [x] S18-04: 4개 단위 테스트 추가
- 커밋: `5fa10308`, `8e596136`
- 결과: OpenAI 크레딧 없이 LOCAL_EMBEDDING_ENABLED=true로 로컬 임베딩 활성화 가능

## Sprint 19 완료 (2026-05-21) — fastembed 배치 최적화 + Obsidian Vault 인제스트 API

- [x] S19-01: fastembed generate_embeddings_batch 단일 embed() 호출 + vault ingest API
- [x] S19-02: vault 보안 강화 — symlink traversal 차단, 10MB 파일 크기 상한, 공유 EmbeddingService
- 커밋: `52637974`, `df89f5bd`
- 결과: POST /api/vault/ingest 구현, O(N) Mutex lock → O(1) 배치 최적화, 보안 취약점 해결

## Sprint 20 완료 (2026-05-21) — Obsidian Vault 파일 감시 (notify-debouncer-full)

- [x] S20-01: notify v6 + notify-debouncer-full v0.3 추가 + VaultWatcherService 스켈레톤 + vault_common 모듈
- [x] S20-02: 파일 이벤트 → 인제스트 파이프라인 연결 (DocumentSource::File, dotfile 필터, 루트 검증)
- [x] S20-03: VaultWatchConfig, AppState 라이프사이클, /vault/watch/status+reload (admin-gated)
- 커밋: `88df037a`, `bc092f14`
- 결과: VAULT_WATCH_ENABLED=true + roots + user_id 설정 시 Obsidian vault .md 파일 변경 자동 인제스트

## Sprint 21 완료 (2026-05-21) — source_path dedup + 초기 스캔 + sync report

- [x] S21-01: migration 010 — documents.source_path 컬럼 + partial unique index (user_id, source_path)
- [x] S21-02: storage.rs upsert source_path 분기 (dedup by path vs title) + ingest_single_file DocumentSource::File
- [x] S21-03: VaultWatcher 초기 스캔 (VaultWatchConfig.initial_scan: bool, default true)
- [x] S21-04: GET /api/vault/sync/report — disk vs DB orphan/untracked 비교 (read-only, AdminUser)
- [x] 리뷰 HIGH 수정: axum_extra::Query (multi-root), sync_report DB 에러 전파, 초기 스캔 usize::MAX cap
- 커밋: `1c0157e6`, `b0eb8b41`
- 결과: source_path 중복 방지, 기존 파일 일괄 인제스트, DB-파일 sync 보고서 API

## Sprint 22 완료 (2026-05-21) — sync_report robustness hardening

- [x] S22-01: escape_like_prefix() — LIKE `\`, `%`, `_` 이스케이프 + `ESCAPE '\\'`
- [x] S22-02: dedup_roots() — 중첩 root 자동 제거 (component-wise prefix 비교)
- [x] S22-03: SYNC_REPORT_DB_LIMIT=50_000 + `LIMIT N+1` + `truncated: bool` 응답 필드
- [x] S22-04: try_canonicalize() — disk/DB 양쪽 경로 정규화 (symlink 해소)
- [x] 리뷰 HIGH 수정: reverse-order dedup 테스트 추가 (order-independence 증명)
- [x] 리뷰 MEDIUM-5 수정: SQL LIMIT을 상수에서 derive (format!("{}", SYNC_REPORT_DB_LIMIT + 1))
- 커밋: `a52c0f81`, `e5785f85`
- 결과: sync_report가 경로 특수문자, 중첩 root, 대용량 결과, macOS symlink 환경에서 올바르게 동작

## Sprint 23 완료 (2026-05-21) — sync_report per-root budget + O(1) 정규화

- [x] S23-01: per-root truncation budget — remaining 차감, early break `> SYNC_REPORT_DB_LIMIT`
- [x] S23-02: canonicalize-once-per-root — root_to_canonical Vec + prefix-swap (O(N roots))
- [x] 리뷰 HIGH-1: trailing-slash root → trim_end_matches('/') 정규화
- [x] 리뷰 HIGH-2: 인접 root prefix 오매칭 → path-component boundary 체크 (bytes[len] == b'/')
- [x] 리뷰 HIGH-3: false-positive truncated at LIMIT → `>` 로 변경 (+1 sentinel 일치)
- [x] 리뷰 MEDIUM-1: clippy::useless_vec → 배열 리터럴 교체
- 커밋: `9cb0663d`, `cc607fc1`
- 결과: sync_report DB 쿼리 stat() O(N파일)→O(N루트), root-biased truncation 제거, 경로 버그 수정

## Sprint 24 완료 (2026-05-21) — sort_roots_longest_first + escape_like 테스트 coverage

- [x] S24-01: `sort_roots_longest_first(&mut [(String, String)])` 헬퍼 추출 — root_to_canonical pairs를 raw root 길이 내림차순 정렬 (defense-in-depth: dedup_roots가 overlapping root를 허용해도 longer root가 prefix-swap 승리)
- [x] S24-02: 단위 테스트 8개 추가
  - `root_to_canonical_sorted_longest_first` — 헬퍼가 longest-first 정렬 검증
  - `root_to_canonical_sorted_find_hits_first` — 정렬 후 find()가 올바른 canonical 선택 검증
  - `escape_like_empty_string_unchanged`
  - `escape_like_plain_path_unchanged`
  - `escape_like_backslash_doubled_first` — ordering invariant (`\` → `\\` must happen before `%` escaping)
  - `escape_like_percent_escaped`
  - `escape_like_underscore_escaped`
  - `escape_like_all_special_chars_combined`
- [x] 리뷰 MEDIUM 수정: 테스트가 `sort_roots_longest_first` 헬퍼를 직접 호출하도록 변경 (inline sort 제거 → production helper에 binding)
- 커밋: `cceaf5a7`, `3e4d72f7`
- 결과: 1,745 Rust pass / 0 fail / 2 ignored / 0 clippy warnings. 현재는 correctness-neutral (dedup_roots가 overlap을 보장하지 않음)이지만 dedup_roots regression 시 안전망 역할.

## Sprint 25 완료 (2026-05-21) — vault_common 심링크 엣지 케이스 + escape_like contract 정리

- [x] S25-01: vault_common.rs 심링크 + 안전성 단위 테스트 4건 추가
  - `collect_empty_dir_returns_empty` — empty dir yields empty result without panic
  - `collect_symlink_root_returns_empty` (#[cfg(unix)]) — root-level symlink guard (lines 124-131)
  - `is_safe_md_path_accepts_real_md_file` — happy path acceptance criteria
  - `is_safe_md_path_rejects_symlink_to_md` (#[cfg(unix)]) — symlink_metadata sees symlink type, extension never checked
- [x] 코드 리뷰 MEDIUM-1, MEDIUM-2 수정: vault.rs ESCAPE contract 중복 테스트 2건 제거
  - 기존 `assert_eq!` 테스트와 contains() 단정문이 중복 → 더 약한 contains() 제거
  - ESCAPE clause 계약은 escape 테스트 섹션 블록 코멘트로 문서화
- [x] 코드 리뷰 LOW-2, LOW-3, LOW-5 수정: 추가 안전성 테스트 3건
  - `is_safe_md_path_rejects_dangling_symlink` — dangling symlink still rejected (symlink_metadata succeeds)
  - `collect_skips_symlinks_inside_dir` — per-entry symlink guard in collect_md_recursive
  - `is_safe_md_path_accepts_uppercase_extension` — eq_ignore_ascii_case coverage
- 커밋: `e0787245`, `fb6cc8ff`
- 결과: 1,752 Rust pass / 0 fail / 2 ignored / 0 clippy warnings. 두 심링크 guard 레이어 + happy/edge path 모두 명시적 단위 테스트로 보호.

## Sprint 26 완료 (2026-05-21) — HTML 추출 구현 + sync_report 응답 형상 테스트

- [x] S26-01: `parsing.rs` HTML headings/links 추출 — regex `(?is)` (dotall+case-insensitive), `position` = byte offset, OnceLock 정적 컴파일
- [x] S26-02: sync_report response shape contract test — orphan/untracked/truncated 필드 검증 (DB 없이)
- [x] 리뷰 H1 수정: parse_html 내 9개 regex OnceLock 상수화 (per-document 재컴파일 제거)
- [x] 리뷰 H3 수정: `position` 하드코딩 0 → `cap.get(0).start()` (raw.content 내 byte offset)
- [x] 리뷰 M6 수정: flag 검증 테스트 3건 추가 (dotall, case-insensitive, position byte offset)
- 커밋: `b2fa2233`, `a1ae4271`
- 결과: 1,737 Rust pass / 0 fail / 0 clippy warnings. parsing.rs HTML 추출 TODO 2건 해소.

## Sprint 27 완료 (2026-05-21) — HTML 텍스트 정규화 + code_blocks 추출

- [x] S27-01: `decode_html_entities` DRY helper 추출 + heading/link 텍스트 정규화
  - inner tag → space 치환 (`foo<br>bar` → `foo bar`)
  - HTML entity decode (`&amp;` → `&`, `&apos;` → `'` 등)
- [x] S27-02: `parse_html()` HTML code_blocks 추출 (`Vec::new()` 해소)
  - `<pre><code class="language-rust">` → `CodeBlock { language: Some("rust"), code, start_position }`
  - `pre_regex`, `code_tag_regex` (no `$` anchor, truly non-greedy), `code_language_regex` (class= anchored)
- [x] S27-03: `Docs/RAG_E2E_SETUP.md` RAG E2E SOP 문서 작성 (507줄)
- [x] 리뷰 수정: unknown entity 보존, `$` anchor 제거, `class=` 앵커링, bare-pre tag strip
- 커밋: `f107729f`, `dd637ee4`, `e7e71c9e`
- 결과: 1,752 Rust pass / 0 fail / 0 clippy warnings

## Sprint 28 완료 (2026-05-22) — Named Entity 디코딩 + Multi-Code Sibling

- [x] S28-01: named HTML entity 32종 + `&#decimal;` + `&#xhex;` 디코딩
  - `&rsquo;` → `'`, `&mdash;` → `—`, `&hellip;` → `…` 등 32종 named entity
  - `&#39;` decimal + `&#x27;`/`&#X27;` hex (case-insensitive, bounds-limited)
  - `char::from_u32` 으로 surrogate/overflow 안전 처리
- [x] S28-02: `<pre>` 내 multi-`<code>` sibling 지원 — `flat_map + captures_iter`
  - `^` anchor 제거 → `captures_iter` 가 모든 sibling `<code>` 탐색 가능
  - bare-pre fallback 유지 (code_tag_regex 매칭 없을 때)
- [x] S28-03: `Docs/SCRAPER_MIGRATION_EVAL.md` — scraper vs regex 트레이드오프 문서
  - 결정: Sprint 29에서 heading/link를 scraper로 마이그레이션; regex는 body strip + code blocks 유지
- [x] S28-04: position 좌표계 module-level docs + `Heading::position` field doc
- [x] 리뷰 수정: entity_regex 길이 bounds, `\p{White_Space}+` NBSP 정규화, 3 새 테스트
- 커밋: `24307ee6` (S28-01~04), `c312df38` (리뷰 수정)
- 결과: 1,762 Rust pass / 0 fail / 0 clippy warnings

## Sprint 29 완료 (2026-05-22) — scraper/html5ever 마이그레이션

- [x] S29-01: `scraper = "0.20"` 추가; `scraper_extract_all()` 단일 파싱 함수
  - heading + link를 하나의 `Html::parse_document`로 추출 (double-parse 제거)
  - `OnceLock<Selector>` 패턴, `Selector: Sync+Send` compile-time assertion
- [x] S29-02: `heading_regex` / `link_regex` 제거; parse_html 콜사이트 교체
  - html5ever: mismatched tag 자동 처리, HTML5 entity 전체 표, text node 직접 수집
  - 테스트 3건 업데이트 (html5ever 스펙 기반)
- [x] S29-03: multi-`<pre>` 통합 테스트 4건 추가
- [x] S29-04: module-level position docs 업데이트 (best-effort HTML heading offset 명시)
- [x] 리뷰 수정: uppercase tag position (`eq_ignore_ascii_case` 윈도우), 단일 파싱, 주석 갱신
- 커밋: `ddcdd35a` (S29-01~04), `8f8ecc74` (리뷰 수정)
- 결과: 1,768 Rust pass / 0 fail / 0 clippy warnings

## Sprint 30 완료 (2026-05-22) — title via scraper, Link::position, 좌표계 문서화

- [x] S30-01: `title_regex()` 제거; `scraper_extract_all()` → `(Option<String>, Vec<Heading>, Vec<Link>)`
  - `T_SEL: OnceLock<Selector>` for `"title"` CSS selector
  - html5ever로 full HTML5 entity table 처리 (title의 `&mdash;` 등 rare entity 해소)
  - whitespace_regex() collapse + trim; absent `<title>` → raw.title fallback
  - 3 테스트: `html_title_decodes_entities`, `html_title_collapses_whitespace`, `html_title_missing_falls_back_to_raw_title`
- [x] S30-02: `Link::position` 필드 추가 (`Heading::position`과 대칭)
  - HTML: 3-byte `<a` window scan + alphanumeric guard (skips `<abbr>`, `<aside>`, `<audio>` 등)
  - Markdown: `plain_text.len()` at `Event::End(Tag::Link)` emit time
  - 4 테스트 + 리뷰 수정 1건 (`markdown_link_inside_heading_has_text`)
- [x] S30-03: position docs — "character offset" → "byte offset into `plain_text`" 전역 수정
  - module-level, `Heading::position`, `CodeBlock::start_position` rustdoc
- [x] S30-04: module-level "Entity decoding boundaries" + "Known limitations" 섹션 추가
  - Entity 경로 분기 이유 명시 (DOM restructuring risk, verbatim code)
  - SVG `<title>`, table foster-parenting, markdown link-in-heading position limitations
- [x] 리뷰 수정: `Event::Text` 핸들러에서 heading 내부 링크 text 수집 버그 수정
- 커밋: `6500d710` (S30-01~04), `e343a4d4` (리뷰 수정)
- 결과: 1,776 Rust pass / 0 fail / 0 clippy warnings

## Sprint 31 완료 (2026-05-22) — html-escape entity decode, head>title, link position

- [x] S31-01: `decode_html_entities()` → `html_escape::decode_html_entities()` 위임
  - `html-escape = "0.2"` Cargo.toml 추가; `entity_regex()` + 32-arm match 제거
  - 2,231 HTML5 named entity 전체 지원 (기존: 32종)
  - NUL byte (`&#0;`) → U+FFFD 후처리 (PostgreSQL TEXT 안전)
  - 5 테스트: micro, sect, unknown_preserved, nul_replaced, 기존 4건 유지
- [x] S31-02: T_SEL `"title"` → `"head > title"` (SVG `<title>` false-positive 방지)
  - html5ever implicit `<head>` 자동 삽입으로 암묵적 `<head>` 없는 문서에서도 동작
  - SVG Known Limitations 섹션 제거; 2 테스트 추가
- [x] S31-03: Markdown link-in-heading position 정확화
  - `link_position_snapshot = plain_text.len() + current_heading_text.len()` at `Start(Link)`
  - `End(Link)` 에서 heading 내부 link_text 이중 flush 방지 가드 추가
  - 자기일관성 단언 `plain_text[position..].starts_with(link.text)` 추가
  - 2 테스트: heading position, preceding_text + heading position
- [x] 리뷰 수정: NUL byte 후처리, End(Link) 이중 flush 수정, 자기일관성 단언
- 커밋: `dc829253` (S31-01~03), `b285e90f` (리뷰 H1/H2 수정)
- 결과: 1,784 Rust pass / 0 fail / 0 clippy warnings

## Sprint 32 로드맵

- P1: `html_invalid_hex_entity_surrogate_preserved` 테스트 이름 수정 ("preserved" → "replaced_with_fffd")
- P2: surrogate numeric entity(`&#xD800;`) 처리 경로 일관성 —
  body/code path(html-escape)도 U+FFFD 반환하도록 맞추기 (현재 heading/link는 html5ever → U+FFFD)
- P3: `End(Paragraph)` 에 separator 추가 검토 — 현재 plain_text에 단락-heading 경계 구분자 없음
  (`"introHeader link"` — "intro" 다음 바로 heading 텍스트가 붙음)
- P4: 연속 `<a>` 태그 position 검증 테스트 (`a_search_start = position + 1` 패턴 검증)

## Rust TODO 현황 (29건, 2026-05-21 업데이트)

실제 운영에 영향을 주지 않는 stub/placeholder. 필요 시점에 구현.

| 분류 | 건수 | 파일 | 비고 |
|------|------|------|------|
| ML stub (빈 구현체) | 5 | ml_service.rs | 클러스터링/토픽/트렌드 |
| Admin stub | 6 | admin_service.rs | 백업/설정/업타임 |
| Export/Import stub | 3 | export_service.rs | 큐 기반 작업 |
| Slack DB 연동 미완 | 3 | slack.rs | extraction_jobs 테이블 |
| OCR stub | 2 | ocr_service.rs, ocr.rs | job queue + 결과 저장 |
| Sync stub | 2 | sync_service.rs | job queue + 스케줄 |
| Security DB 연동 | 2 | security_service.rs | 규칙 로드/저장 |
| ~~Parsing 미완~~ | ~~2~~ → **0** | parsing.rs | ~~HTML headings/links 추출~~ ✅ Sprint 26 완료 |
| 기타 | 6 | hybrid, ws, websocket, template | 개별 항목 |

## Blocked / Waiting

- ~~OpenAI API 크레딧 부족~~ → fastembed-rs 로컬 임베딩으로 해결 (Sprint 18)
- RAG E2E 검증: LOCAL_EMBEDDING_ENABLED=true + DB 마이그레이션 실행 필요

---

## Notes

- Rust 백엔드 (`minky-rust/`) 기준으로 개발
- Python 백엔드 (`app/`)는 레거시, 참고용
- 임베딩 API 선택: OpenAI text-embedding-3-small 권장 (`.claude/references/apis/embedding-apis.md` 참조)

## Completed (Phase 0)

- [x] **CI/CD 통합 시스템 완성** (2026-02-19)
  - ci-runner.md, health-checker.md 에이전트
  - /ci, /health 커맨드
  - 파이프라인: default.yml, hotfix.yml, validate-only.yml
  - ci-trigger.yml GitHub Actions
  - CLAUDE.md PM/CI 자동화 프로토콜 추가

- [x] **PM 자동화 시스템 완성** (2026-02-19)
  - state-manager.md - 상태 저장/복구, 체크포인트, 롤백
  - work-scheduler.md - 작업 대기열, 의존성 관리
  - feedback-loop.md - 패턴 학습, 인사이트 추출
  - notifier.md - 알림, 리포트 생성
  - 커맨드: /state, /queue, /feedback, /notify
  - 설정: .claude/config.json
  - 상태 파일: .claude/state/*
  - PM 에이전트 Enhanced 업그레이드

- [x] **GitHub 이슈/PR 자동화 시스템** (2026-02-19)
  - issue-manager.md - 이슈 관리 (1시간마다 분석)
  - issue-developer.md - 이슈 해결 및 PR 생성
  - technical-writer.md - 기술 보고서 작성
  - github-automation.md - GitHub 자동화 설정
  - 커맨드: /issue, /issue-dev, /tech-report, /setup-github
  - GitHub Actions: issue-triage, pr-check, tech-report
  - LessonLearn 폴더 및 라벨 체계 구축

- [x] **PM 에이전트 시스템 구축** (2026-02-18)
  - pm.md - 프로젝트 매니저 (메인 오케스트레이터)
  - task-executor.md - 태스크 실행기
  - code-reviewer-minky.md - 코드 리뷰어
  - validator.md - 검증 에이전트
  - progress-tracker.md - 진행 상황 추적기
  - 커맨드: /pm, /next, /review, /validate, /progress
  - 스킬: pm/SKILL.md

- [x] **지식 관리 에이전트 시스템 구축** (2026-02-18)
  - doc-analyzer, knowledge-linker, search-assistant
  - insight-extractor, summary-writer, reference-manager

- [x] **커맨드 시스템 구축** (2026-02-18)
  - /ingest, /ask, /capture, /summarize
  - /related, /status, /ref-save, /ref-search

- [x] **스킬 시스템 구축** (2026-02-18)
  - doc-understanding, semantic-search, rag-answering
  - knowledge-linking, tacit-extraction

- [x] **레퍼런스 시스템 구축** (2026-02-18)
  - `.claude/references/` 디렉토리 및 인덱스
  - PKM 도구 조사, RAG 패턴, Embedding API 비교 저장
  - 모든 에이전트에 레퍼런스 활용 안내 추가

---

- [x] **API 문서 최신화 (Slack/Knowledge)** ✅ (2026-02-19 완료)
  - Docs/api/slack.md (6개 엔드포인트 전체 문서)
  - Docs/api/knowledge.md (그래프/팀 전문성 API)
- [x] **테스트 550개 목표** ✅ (2026-02-19 완료 - 592개 달성)
  - models/tag, websocket, sync, template, agent, harness (+76개)
  - services/timeline_service 순수 함수 추출 + 테스트 (+16개)
- [x] **OpenAPI 스펙 자동 생성** ✅ (2026-02-19 완료)
  - openapi.rs - GET /api/docs/openapi.json
  - 전체 API 경로/스키마 문서화

- [x] **테스트 650개 목표** ✅ (2026-02-19 완료 - 655개 달성)
  - models/search, ocr, document, export, user 테스트 확장 (+47개)
  - Rust 총 655개 (unit 639 + integration 4 + kg 11 + doc 1)
- [x] **Frontend ChatContainer 테스트** ✅ (2026-02-19 완료 - 280개 달성)
  - ChatContainer.test.jsx 신규 17개 테스트
  - Frontend 총 280개 (263 -> 280)
- [x] **CI/CD 워크플로우 개선** ✅ (2026-02-19 완료)
  - pr-check.yml: 테스트 카운트 job outputs, PR 코멘트 테스트 수 표시
  - cargo cache restore-keys 추가
- [x] **테스트 700개 목표** ✅ (2026-02-19 완료 - 707개 달성)
  - notification_service: 순수 함수 추출 (15개 테스트)
  - search_service: 순수 헬퍼 함수 추출 (20개 테스트)
  - ml_service: 통계 함수 추출 (18개 테스트)
  - openapi: 추가 테스트 (10개)
  - Rust 총 707개 (unit 639+52 + integration 4 + kg 11 + doc 1)
- [x] **Frontend 테스트 300개 목표** ✅ (2026-02-19 완료 - 304개 달성)
  - dateUtils.test.js 신규 24개 테스트
  - Frontend 총 304개 (280 -> 304)
- [x] **Playwright E2E 테스트 추가** ✅ (2026-02-19 완료)
  - knowledge.spec.js: Knowledge Search + Knowledge Graph (11개 테스트)
  - chat.spec.js: Chat Interface (8개 테스트)
  - navigation.spec.js: 라우트 추가 (10개 테스트)
  - playwright.config.js: Rust 백엔드 웹서버 설정 업데이트
  - 총 E2E 28개 all pass (chromium)

- [x] **Rust 테스트 750개 목표** ✅ (2026-02-19 완료 - 778개 달성)
  - audit_service, comment_service, document_service, tag_service 순수 함수 추출 + 55개 테스트
  - 총 778개 (unit 762 + integration 4 + kg 11 + doc 1)
- [x] **Frontend 테스트 350개 목표** ✅ (2026-02-19 완료 - 337개 달성)
  - obsidianRenderer.test.js 신규 23개, searchService.test.js 신규 10개
  - Frontend 총 337개
- [x] **Criterion 벤치마크 추가** ✅ (2026-02-19 완료)
  - benches/core_functions.rs: 19개 벤치마크 함수 (document/tag/comment/audit service)
  - cargo bench로 실행 가능

*Last updated: 2026-05-21 (Sprint 27 완료 — HTML 텍스트 정규화 + code_blocks 추출, Rust 1,752 pass)*
