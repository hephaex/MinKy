> minky 모노레프로 미사용/데드코드 감사 완료 — Rust Cargo deps `image`/`futures` 제거 + `SecretString` 미사용 임포트 제거, 프런트엔드 13 deps 전부 사용 확인, 전체 cargo test 2026 passed / 0 failed.

## 단계 1: 도구 확인 및 Rust 데드코드 센서스
- knip/depcheck/ts-prune/cargo-udeps 모두 PATH에 없음 → 수동 grep 방식으로 감사 진행.
- `#[allow(dead_code)]` 센서스: 10곳 / 6파일 (embedding.rs 2, categories.rs 2, documents.rs 1, slack.rs 2, agent_service.rs 2, mcp_server.rs 1). 전부 원문 확인 완료. *(단계 3 마이그레이션 후 fresh 재산출: 6곳 / 4파일 — embedding.rs 2·agent_service.rs 2 제거로 소멸. 단계 1 기록은 감사 당시 스냅샷으로 보존.)*

## 단계 2: Cargo.toml 미사용 의존성 제거
- `image = "0.25"`, `futures = "0.3"` 제거 (둘 다 주석 처리된 데드 라인이던 것 포함 정리).
- File Processing 블록: `mime_guess 2.0`, `tempfile 3.14`만 남김. Async utilities: `async-trait 0.1`만 남김.
- `cargo check` GREEN (dev profile, 0.87s).

## 단계 3: 미사용 임포트 제거
- `cargo check --all-targets`에서 유일한 warning: `unused import: secrecy::SecretString` at `src/services/agent_service.rs:339` (테스트 모듈).
- 해당 라인 제거, 재컴파일 — warning 0, GREEN.

## 단계 4: 전체 cargo test 재검증
- 초기 post-cleanup 실행에서 `pipeline_auth_qa` 5 failed (`VersionMismatch(1)`) — DATABASE_URL이 `minky_db`를 가리키던 알려진 테스트 인프라 이슈 재발.
- `DATABASE_URL=postgresql://mare@localhost/minky_rust_db cargo test` 재실행: **2026 passed / 0 failed / 13 ignored** (13 suites 모두 ok).
  - lib unittests: 1979 tests, 1968 passed / 11 ignored.
  - `pipeline_auth_qa`: 5 passed (1.94s) — 제품 결함 아님, 환경 문제로 확정.

## 단계 5: 프런트엔드 감사 (읽기 전용, 변경 없음)
- `frontend/package.json`: 13 dependencies + 12 devDependencies 전부 src 사용 확인 — 미사용 의존성 없음.
- `mermaid`는 `MermaidDiagram.js:26`의 동적 `import('mermaid')`로 런타임 사용 (테스트 전용 아님); jest `moduleNameMapper`가 `^mermaid$` → `mermaid/dist/mermaid.core.mjs` 매핑 (CRA jest 호환).
- `react-scripts`: CRA 스크립트(start/build/test/eject)에 사용 — 유지.
- `socket.io-client`: `collaborationService.js` 단독 사용.
- package-lock.json root 항목과 package.json 완전 동기화 (stale/missing 없음).

## 데드코드 10곳 분류 (유지/제거 판정)
- **KEEP**: `mcp_server.rs:361` — "category_id/created_at/updated_at document the full scoped-row shape" 명시적 근거 있음.
- **KEEP (Flask-compat 수용 쿼리 파라미터)**: `categories.rs:29/33` (`flat`, `include_inactive`), `documents.rs:244` (`group_by`), `slack.rs:93` (`since`), `slack.rs:101` (`state`).
- **KEEP (테스트 헬퍼, 실사용)**: `agent_service.rs:341/369` (`make_config`/`make_service` — `test_agent_row_structure`에서 사용).
- **REMOVED (재검증 후 제거 완료)**: `embedding.rs` — 단일 텍스트 경로(`generate_embedding` + `OpenAIEmbeddingRequest`) 제거. 배치 경로(`generate_embeddings_batch` + `OpenAIBatchEmbeddingRequest`)만 실사용 확인. `cargo check --all-targets` warning 0 / GREEN, 전체 `cargo test` 재검증 **2026 passed / 0 failed / 13 ignored** (13 suites 모두 ok) — 데드코드 0건 수렴.

## 단계 6: 미참조 pub 선언 전수 감사 (/refactor clean, 2026-10-25)
- 도구: knip/depcheck/ts-prune/cargo-udeps 없음 → grep 휴리스틱 (선언 라인 제외 전수 참조 카운트, openapi.rs·tests·benches 포함).
- 배경: lib 크레이트 특성상 pub 항목은 rustc dead_code 경고 대상에서 제외 (`cargo check --all-targets` warning 0) → 전수 grep으로 보완.
- 센서스 재산출(python3 스크립트, 2026-10-25): **미참조 72개 / 33파일**로 정정 — 초기 집계(65개/31파일) 대비 +7/+2 오차.
- 결과: pub 선언 총 1,644개 중 **미참조 72개** (이름이 선언 라인 외 코드베이스에 0회 등장).
- 분포(전체 72개, artifact 전수 확보): services 36 / 14파일 (websocket_service 8, audit 7, migration_service 5, tag 3, notification 3, attachment 2, html_renderer 1, pdf_writer 1, slides_writer 1, hybrid_search 1, security_service 1, slack_oauth_service 1, version_service 1, ai_service 1) · models DTO 12 / 9파일 (agent 1, analytics 1, export_theme 2, function 2, hierarchical_context 1, slack 2, sync 1, websocket 1, workflow 1) · nitl_config.rs 10 · pipeline 9 / 5파일 · routes/plot.rs 2 · middleware/auth.rs 1 · migration.rs 1 · integration/nitl.rs 1.
- 의존성: Rust 36 + dev 6, 프런트엔드 13 + 12 — 전부 사용 확인, 미사용 0.
- `#[allow(dead_code)]` 잔여 6곳: Flask-compat 쿼리 5곳 + mcp_server.rs:361 (문서화 근거) — KEEP 유지.
- ai.rs duplicate #[test] 의심 건: 원문 확인 결과 중복 없음 + cargo check warning 0 — 비이슈 종결.
- **제거 실행 보류 — 사용자 승인 대기** (destructive-ops: 72개 일괄 제거는 개별 승인 필요).
## 단계 7: Anthropic 제거 잔여 doc-impact 정리 (2026-10-25)
- `Docs/GETTING_STARTED.md` line 497: `### 3. Configure AI Features` env block의 잔여 `ANTHROPIC_API_KEY=sk-ant-...` → `UPSTAGE_API_KEY=up-...` 교체 (count==1 검증). 전체 파일 재검색 — `ANTHROPIC` 0건.
- `Docs/ARCHITECTURE.md` line 784: References의 `RAG Best Practices (Anthropic)` 외부 링크 → Upstage Solar / Strix Halo local llama.cpp 링크로 교체.
- `Docs/ARCHITECTURE_ANALYSIS.md` lines 108/144: Service Dependencies 다이어그램의 `LLM Providers (OpenAI, Anthropic)` → `(Upstage Solar, Strix Halo local llama.cpp)`, External APIs 박스 `- Anthropic` → `- Upstage` (ASCII 다이어그램 컬럼 폭 유지).
- `Docs/RUST_MIGRATION_PLAN.md` lines 70/223/257: crate 매핑 표 `anthropic | anthropic-rs | Claude models` 행 제거, Phase 3 LLM Provider Abstraction에서 `Anthropic (anthropic-rs)` 삭제, `AnthropicProvider` 구조체 예시 → `UpstageProvider (OpenAI 호환)` 교체.
- `minky-rust/.env.example` line 20: 잔여 `ANTHROPIC_API_KEY=sk-ant-...` 제거 + line 21–22 주석을 solar-open2 (query expansion, hybrid search)로 갱신.

## 단계 8: LessonLearn 감사 정리 문서
- `LessonLearn/2026-10-25_minky_unused_dead_code_audit.md` 신규 작성 (형식: `### N. [핵심 개념]` + 무엇/왜 중요/적용, 태그 #rust #refactor #bugfix).

## 단계 9: 전체 doc residue 재검증 완료 (2026-10-25)
- 대상: Docs/ 전체(ARCHITECTURE.md, ARCHITECTURE_ANALYSIS.md, GETTING_STARTED.md, RAG_E2E_SETUP.md, RUST_MIGRATION_PLAN.md, api/{api,understanding}.md, memory-role-separation.md, test-upload.md), README.md, CLAUDE.md, broker/README.md, legacy/python-backend/README.md, minky-rust/.env.example.
- 결과: `grep -rln anthropic|Anthropic|ANTHROPIC` — 잔여 0건 (audit history 문서 제외).
- 교체 내역 요약:
  - CLAUDE.md: pipeline 다이어그램 `AI Understanding (Upstage Solar)`, RAG 다이어그램 `Solar Generation`, try-list 2곳 `Upstage Solar`, References 외부 링크 → Upstage Solar API.
  - README.md: 아키텍처 다이어그램 `Solar (AI)`, tech stack `Upstage Solar (OpenAI-compatible)` + `solar-open2, 4096-dim`.
  - broker/README.md: 섹션 제목 `Registering with an MCP Client (Claude Code / GJC)` (본문 Claude Code 언급은 CLI 클라이언트 지칭이라 유지).
  - Docs/memory-role-separation.md: `Claude Code Auto-Memory` → `CLI Agent Auto-Memory` 5곳.
  - Docs/test-upload.md: author 필드 `Claude Code` → `Minky Sample`.
- 비이슈: Docs/api/slack.md 502 에러 표 2곳(`Anthropic API unavailable`)은 이미 단계 7 이전 정리 완료 — 최종 재검색에서 0건 확인.

## 단계 10: dead orphan 발견 — src/routes/plot.rs (컴파일 그래프 밖)

**배경**: 단계 6 pub-decl census(72개/33파일)의 routes/plot.rs 2개 decl(update_plot:43, _openapi_spec_notes:90) 소급 조사.

**원인/판정**:
- `src/routes/plot.rs`는 **어디에서도 `mod plot;`으로 선언되지 않음** — routes/mod.rs(40개 모듈 선언 목록에 plot 부재), models/mod.rs, lib.rs, main.rs 전부 부재.
- 따라서 **컴파일 그래프 밖 dead orphan** — `cargo check` GREEN + warning 0건이 이를 확인(존재하는 파일이 컴파일되면 참조 오류가 날 것이므로).
- 파일 내부가 이미 깨져 있어 wired 시 컴파일 불가:
  - `AppError::GpmNotConfigured` / `AppError::GpmError` — `src/error.rs` AppError에 존재하지 않는 변형.
  - `crate::models::plot::PlotRecord` — `src/models/plot.rs` 파일 자체가 없음.
- 유래: `c5ffb35a` "chore(rust): import CT306 production minky-rust on its fork point" 커밋에서 CT306 fork-point import 시 함께 들어온 잔여물(그 당시에도 미컴파일 상태로 추정).

**조치 판단**: 제거 후보 — 단, 파일 삭제는 파괴적 작업이므로 사용자 승인 대기. census의 routes/plot.rs 2 decl은 이 dead orphan 소속으로 실질 moot.

**검증**:
- `grep -rn "mod plot" routes/mod.rs models/mod.rs lib.rs main.rs` → 0 matches.
- `cargo check` → GREEN, warning 0.
- `grep -n "GpmNotConfigured\|GpmError" src/error.rs` → 0 matches (AppError 212 lines, 12 variants 확인).

## 단계 11: 미참조 pub-decl 배치 제거 (A+B 승인 스윕 완료)

> 한 줄 요약: 승인된 54건 미참조 pub-decl을 구조적 스캐너로 완전 제거하고, cargo check --all-targets 경고 0 / 전체 테스트 1968+71 passed / 0 failed로 검증 완료.

### 배경
- 신규 센서스(1641 total pub-decl / 70 unreferenced / 32 files)에서 사용자 승인: **A+B 전체** (고아 파일 + 유효 미참조 decl).
- 고아 파일 6건은 선행 삭제 완료(단계 10): `src/nitl_config.rs`, `src/migration.rs`(top-level), `src/pipeline/content.rs`, `src/pipeline/safety.rs`, `src/integration/nitl.rs`, `src/routes/plot.rs`.
- 잔여 유효 대상 54건: services 36 + models 12 + pipeline 5 + middleware 1.

### 조치
- `/tmp/batch_remove_decls.py` 재작성(선행 시도는 구문 오류·로직 불충분으로 폐기): 파일별 하향식(descending lineno) 처리, `( ) { }` 구조 이벤트 스캐너(라인/블록 주석·문자열·char·raw string 회피), fn은 인자 괄호 매칭 후 `{` 매칭, struct/enum은 첫 `{` 매칭, 직상단 `///` 문서줄·`#[...]` 속성줄 동반 삭제, 파일별 `/tmp/decl_backup` 스냅샷.
- 54/54 OK. 대표 스팬: `auth_middleware` auth.rs 12-47(완전 axum middleware), `audit_service` 7메서드, `websocket_service` 8메서드, `HtmlExportOptions`/`SlidesExportOptions` struct 블록, `models/*` 12개 struct/fn.
- 제거 후 잔여 참조 검증: `auth_middleware|notify_document_created|get_workspace_credentials|SyncFileInfo|ChannelSubscription` → `optional_auth_middleware`만 잔존(별도 API, 유지).
- **불량 impl 블록 사전 확인**: 제거 대상 struct의 `impl X` 블록 zero — 고아 impl 컴파일 오류 없음 확인 후 실행.

### 검증
- `cargo check --all-targets`: error 0, warning 0 (초기 5개 unused import를 수동 정리: audit_service DateTime/Utc+ResourceType, tag_service uuid::Uuid, websocket_service CollaborationSession/CursorPosition, migration_service MigrationRowCheckpoint/EnqueueVerification → 이후 남은 `chrono::Utc` 1건도 제거).
- `cargo test` (DATABASE_URL=minky_rust_db): **1968 passed / 0 failed / 11 ignored** (lib) + 통합 타깃 합계 71 passed / 0 failed / 2 ignored.
- `pipeline_auth_qa` (G002_QA_DATABASE_URL=minky_rust_qa_db): 5 passed.

### 잔여
- `src/nitl_service.rs` (10324B): 컴파일 그래프 어디에도 선언 없는 고아(undeclared), census 미플래그, 승인 목록 외 → **사용자 승인 전까지 보존**.
- `models/mod.rs`의 `mod migration;`은 `src/models/migration.rs`를 가리키며 top-level `src/migration.rs`와 무관 — top-level 제거는 영향 없음 확인.

## 단계 12: 잔여 고아 nitl_service.rs 삭제 (사용자 승인)

> 한 줄 요약: 컴파일 그래프 밖 고아 nitl_service.rs(10324B, NitlService 래퍼 10개 pub-decl)를 사용자 승인으로 삭제하고 cargo check warning 0 / 1968 passed로 검증 완료 — 감사 전체 종료.

### 조치
- `src/nitl_service.rs` 삭제: 내용은 `crate::nitl_config`를 감싼 `NitlService` 래퍼(config 저장 + init_database/기타 10개 pub-decl). nitl_config.rs가 이미 삭제됨에 따라 wired 불가 상태였고, 어디에도 `mod nitl_service;` 선언 부재 확인.
- 잔여 참조: `nitl_service|NitlService` → src/ tests/ zero.

### 검증
- `cargo check --all-targets`: error 0 / warning 0.
- `cargo test` (minky_rust_db): 1968 passed / 0 failed / 11 ignored + 통합 타깃 합계 71 passed / 0 failed.

### 감사 종료
- 총 7고아 파일 + 54 미참조 pub-decl 제거, cargo check warning 0 / 테스트 0 failed. 남은 `Claude/claude`는 CLI 클라이언트 참조(의도적), `#[allow(dead_code)]` 6곳은 KEEP.

## 단계 13: 전체 테스트 안정성 재검증 (2026-10-26)

- `DATABASE_URL=postgresql://mare@localhost/minky_rust_qa_db cargo test` 전체 스위트: **13 suites 1968 lib + 통합 전체 0 failed**, 2회 연속 반복 실행에서도 재현 없음.
- 1회차 초기 run에서 `pipeline_auth_qa` 5건 `apply migrations: VersionMismatch(1)` 일시적 실패 확인 — 원인: 동일 QA DB에 다른 test binary(chat_message_qa 등)가 동시에 `sqlx::migrate!`를 실행하는 병렬 실행 경합. 격리 실행(`--test pipeline_auth_qa`)에서는 즉시 통과.
- 재실행 시 안정 통과. 정기적 flake로 간주 (sqlx migrate 경합 — 각 test binary가 같은 DB에 동시 migrate 시도). 건드리지 않음.
