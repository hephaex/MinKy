# MinKy Development Plan

> 이 파일은 해야 할 작업들을 관리합니다.
> 에이전트는 세션 시작 시 이 파일을 읽고, 작업 추가/완료 시 업데이트합니다.

---

## 🎯 다음 세션 우선 작업

### ✅ Phase 1 완료! Phase 2 준비

**Phase 1 (Knowledge Understanding) 완료:**
- ✅ pgvector 설정 및 마이그레이션
- ✅ Document Understanding 파이프라인
- ✅ 벡터 임베딩 API (7개 엔드포인트)
- ✅ RAG 검색 API (3개 엔드포인트)
- ✅ 프론트엔드 검색 UI
- ✅ 프론트엔드 채팅 UI
- ✅ API 문서화

**E2E 테스트 결과 (2026-02-19):**
1. [x] PostgreSQL 데이터베이스 마이그레이션 실행
   - pgvector 0.8.0 소스 빌드 및 설치 완료
   - minky_rust_db 생성 및 4개 마이그레이션 적용 완료 (004: search_history 추가)
   - Axum 0.8 라우트 문법 수정 (/:param -> /{param})
   - 빌드: 0 errors, 0 warnings (경고 80개 모두 제거 완료)
2. [ ] OpenAI API 크레딧 부족 - 새 API 키 또는 크레딧 보충 필요
3. [x] Rust 서버 기동 확인 (/api/health 응답 정상)
4. [x] 프론트엔드-백엔드 통합 테스트 완료 (포트 8000으로 업데이트)
5. [ ] RAG 파이프라인 E2E 테스트 (API 키/크레딧 필요)
   - ANTHROPIC_API_KEY 설정 필요 (문서 이해 분석)
   - OpenAI 크레딧 보충 필요 (임베딩, 시맨틱 검색)
6. [x] documents CRUD DB 연동 구현 (2026-02-19 완료)

**코드 품질 개선 (2026-02-19):**
- [x] Rust clippy 경고 80개 → 0개 (type alias, Display impl, derive, allow)
- [x] Frontend 테스트 228/228 통과 (DocumentView 버그 수정)
- [x] Auth 라우트 실제 구현 (login, register, refresh, /me)
- [x] Documents 인증 연동 (AuthUser 추출기로 user_id 교체)
- [x] Rust 단위 테스트 27개 → 67개 (category tree, user model, attachment, version diff, comment tree, embeddings)
- [x] AuthUser 연동 (tags, categories, comments, notifications, workflows, versions, attachments)

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

- [x] **지식 그래프 시각화** ✅ (2026-02-19 완료)
  - SVG 기반 포스-다이렉티드 그래프
  - 노드: 문서/토픽/기술/사람/인사이트 타입별 색상
  - 줌/팬, 노드 클릭 상세 패널, 타입 필터, 검색
  - 라우트: /graph
- [ ] 지식 그래프 백엔드 API (GET /api/knowledge/graph)
  - 실제 문서 임베딩 유사도에서 그래프 생성
  - pgvector 코사인 유사도 기반 엣지 생성
- [ ] Slack/Teams 연동
- [ ] 대화에서 지식 자동 추출
- [ ] 팀원 전문성 맵핑

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

*Last updated: 2026-02-19*
