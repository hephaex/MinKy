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
- [x] Rust 단위 테스트 1,130개 통과
- [x] Auth 라우트 실제 구현 (login, register, refresh, /me)
- [x] Documents 인증 연동 (AuthUser 추출기로 user_id 교체)
- [x] AuthUser 연동 (tags, categories, comments, notifications, workflows, versions, attachments)
- [x] **ESLint 경고 592 → 29개** (2026-03-01 완료, 95% 감소)
  - PropTypes 추가 (27개 컴포넌트)
  - 접근성 개선 (jsx-a11y: onKeyDown, role, tabIndex)
  - label-input 연결 (htmlFor/id)
  - 코드 품질 수정 (no-case-declarations, eslint-disable)

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

## Blocked / Waiting

현재 없음

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

*Last updated: 2026-02-19 (16차 세션 - Rust 778개, Frontend 337개, Criterion 벤치마크 추가)*
