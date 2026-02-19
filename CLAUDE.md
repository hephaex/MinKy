# MinKy - Team Knowledge Intelligence Platform

---

## 🚀 Quick Start (다음 세션 시작 시)

### 방법 1: 자동 PM 시스템 사용 (권장)
```bash
/pm                   # PM 에이전트 시작 (자동 상태 복구)
/ci start             # CI/CD 세션 시작 (헬스체크 포함)
/ci auto 5            # 자동 모드로 5개 작업 연속 실행
```

### 방법 2: 수동 확인
```
1. PLAN.md 읽기       → "다음 세션 우선 작업" 섹션 확인
2. PROGRESS.md 읽기   → "현재 진행 상황" 섹션 확인
3. /health            → 시스템 상태 확인
4. /queue next        → 다음 작업 확인
```

**현재 상태 (2026-02-20):**
- ✅ PM 자동화 시스템 완성 (상태관리, 스케줄링, 피드백, 알림)
- ✅ CI/CD 통합 시스템 구축 (지속적 실행, 헬스체크, 파이프라인)
- ✅ GitHub 이슈/PR 자동화 (이슈 분석, PR 생성, 기술 보고서)
- ✅ **이력 기반 멀티 에이전트 오케스트레이션** (pm-orchestrate)
- ✅ Phase 1 완료 (테스트 1,511개 모두 통과)

---

## 🤖 PM/CI 자동화 프로토콜

### 자동화 시스템 아키텍처
```
┌─────────────────────────────────────────────────────────────────────────┐
│                           PM Agent (Orchestrator)                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐   ┌────────────┐ │
│  │   state-    │   │    work-     │   │  feedback-  │   │  health-   │ │
│  │   manager   │   │   scheduler  │   │    loop     │   │  checker   │ │
│  └──────┬──────┘   └──────┬───────┘   └──────┬──────┘   └─────┬──────┘ │
│         └────────────┬────┴──────────────────┼────────────────┘        │
│                      ▼                       ▼                          │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │                    pm-orchestrate (이력 기반)                       │ │
│  │   execution-patterns.json → 레시피 선택 → 병렬 에이전트 실행        │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                      │                                                  │
│  ┌───────────────────┴───────────────────────────────────────────────┐ │
│  │                    Execution Layer (병렬 실행)                      │ │
│  ├─────────────┬─────────────┬─────────────┬─────────────────────────┤ │
│  │   tdd-      │    task-    │   code-     │      security-          │ │
│  │   guide     │   executor  │  reviewer   │      reviewer           │ │
│  └─────────────┴─────────────┴─────────────┴─────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### 세션 자동 시작 프로토콜

Claude Code가 세션을 시작할 때:

```
1. /health quick              # 빠른 헬스 체크
2. state-manager.restore()    # 이전 상태 복구
3. work-scheduler.sync()      # GitHub/PLAN.md 동기화
4. feedback-loop.load()       # 관련 패턴/인사이트 로드
5. PM 작업 선택 또는 대기
```

### 주요 커맨드

| 커맨드 | 설명 | 사용 시점 |
|--------|------|----------|
| `/pm` | PM 에이전트 시작 | 세션 시작 시 |
| `/pm-orchestrate` | 이력 기반 멀티 에이전트 실행 | 복잡한 작업 시 |
| `/ci start` | CI 세션 시작 (대화형) | 작업 시작 시 |
| `/ci auto N` | N개 작업 자동 실행 | 연속 작업 시 |
| `/health` | 시스템 상태 확인 | 문제 발생 시 |
| `/queue` | 작업 대기열 확인 | 다음 작업 확인 |
| `/state` | 상태 저장/복구 | 세션 전환 시 |
| `/feedback` | 패턴/인사이트 확인 | 작업 전 참고 |
| `/notify` | 알림/리포트 확인 | 진행 확인 시 |

### CI/CD 실행 모드

#### 대화형 모드 (Interactive)
```bash
/ci start
```
- 각 작업 완료 후 사용자 확인
- 안전한 기본 모드

#### 자동 모드 (Autonomous)
```bash
/ci auto 5          # 5개 작업 자동 실행
/ci auto --no-limit # 무제한 실행
```
- 사용자 확인 없이 연속 실행
- 에러 시 자동 일시정지

#### 감시 모드 (Watch)
```bash
/ci watch
```
- GitHub 이벤트 감시
- 새 이슈 감지 시 자동 처리

### 작업 흐름 자동화

```
GitHub 이슈 등록
      │
      ▼
┌─────────────────────┐
│ issue-manager       │ → 이슈 분석/라벨링
│ (1시간마다 동기화)   │ → PM 코멘트 추가
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│ work-scheduler      │ → 우선순위 정렬
│                     │ → 의존성 확인
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│ issue-developer     │ → 브랜치 생성
│                     │ → 코드 구현
│                     │ → 코드 리뷰
│                     │ → 검증
│                     │ → PR 생성
└─────────────────────┘
      │
      ▼
┌─────────────────────┐
│ technical-writer    │ → 기술 보고서 작성
│                     │ → LessonLearn/ 저장
└─────────────────────┘
      │
      ▼
다음 작업 자동 시작
```

### 설정 파일

- `.claude/config.json` - 전체 시스템 설정
- `.claude/state/` - 상태 파일들
- `.claude/pipelines/` - CI/CD 파이프라인 정의
- `.claude/triggers/` - 외부 트리거 파일

### 에러 복구

| 레벨 | 조치 | 예시 |
|------|------|------|
| 자동 재시도 | 3회까지 재시도 | 빌드 실패, 네트워크 |
| 롤백 | 체크포인트 복구 | 코드 리뷰 실패 |
| 스킵 | 다음 작업 진행 | 최대 재시도 초과 |
| 중단 | 수동 개입 요청 | 복구 불가 에러 |

---

## 자율 운영 프로토콜 (Autonomous Operation Protocol)

### 1. 자동 루프 규칙

PM 에이전트가 자율 운영 중 따라야 할 루프 규칙입니다.

```
루프 진입 조건:
- /ci auto N 또는 /ci auto --no-limit 실행 시
- 외부 트리거(.claude/triggers/queue/) 파일 감지 시

루프 실행 규칙:
1. 블로커(blocker) 없으면 다음 작업으로 계속 진행
2. 5턴마다 /compact 실행 → 컨텍스트 압축 및 정리
3. 10턴마다 체크포인트 저장 → .claude/state/checkpoint-{timestamp}.json

블로커 정의 (루프 일시 정지 조건):
- 빌드 실패 3회 연속
- 테스트 커버리지 80% 미달
- 보안 이슈 CRITICAL 발견
- 작업 대기열이 비어 있음
- 수동 개입 요청 플래그 감지
```

### 2. 자동 커밋 규칙

작업 완료 시 자동으로 커밋을 생성합니다.

```
커밋 트리거:
- 기능 구현 완료 (테스트 통과 후)
- 버그 수정 완료 (검증 후)
- 리팩토링 완료 (빌드 통과 후)

커밋 메시지 컨벤션:
feat: 새 기능 추가
fix: 버그 수정
test: 테스트 추가/수정
docs: 문서 업데이트
refactor: 코드 리팩토링
chore: 빌드/설정 변경
perf: 성능 개선
ci: CI/CD 설정 변경

커밋 작성자 규칙:
- 작성자: Mario Cho <hephaex@gmail.com> 단독
- Co-Authored-By 줄 추가 금지
- AI 생성 표시 문구 추가 금지
- 커밋 메시지는 영문으로 작성 (본문은 한국어 허용)
```

커밋 실행 예시:

```bash
git add {변경된_파일들}
git commit -m "$(cat <<'EOF'
feat: add vector embedding storage for documents

pgvector 기반 문서 임베딩 저장 및 검색 기능 구현
- DocumentEmbedding 모델 정의
- embedding_service.rs 구현
- /api/embeddings 라우트 추가
EOF
)"
```

### 3. 에러 복구 규칙

레벨별 복구 전략과 최대 재시도 횟수를 정의합니다.

| 레벨 | 이름 | 조건 | 복구 전략 | 최대 재시도 |
|------|------|------|-----------|------------|
| L1 | 자동 재시도 | 일시적 실패 (빌드, 네트워크) | 즉시 재시도 | 3회 |
| L2 | 롤백 | 코드 리뷰 실패, 테스트 실패 | 마지막 체크포인트로 복구 | 2회 |
| L3 | 스킵 | L1/L2 최대 재시도 초과 | 현재 작업 건너뛰고 다음 진행 | - |
| L4 | 중단 | 보안 이슈, 복구 불가 에러 | 루프 중단 + 수동 개입 요청 | - |

```
롤백 조건:
- cargo clippy 에러 3회 연속 수정 실패
- 테스트 커버리지가 80% 아래로 하락
- API 통합 테스트 2회 연속 실패

롤백 절차:
1. 현재 변경사항 git stash
2. 마지막 성공 체크포인트 로드
3. .claude/state/current-task.json 상태를 "failed"로 업데이트
4. 작업을 work-queue.json 맨 뒤로 이동 (재시도 카운터 증가)
5. 다음 작업 선택

L4 중단 시 알림:
- .claude/triggers/queue/manual-intervention-required.json 생성
- PROGRESS.md에 블로커 내용 기록
```

### 4. 컨텍스트 관리

#### 상태 파일 위치

| 파일 | 역할 | 갱신 주기 |
|------|------|-----------|
| `.claude/state/current-task.json` | 현재 실행 중인 작업 | 작업 시작/완료 시 |
| `.claude/state/work-queue.json` | 대기 중인 작업 목록 | 작업 추가/완료 시 |
| `.claude/state/agent-context.json` | 에이전트 공유 컨텍스트 | 5턴마다 |
| `.claude/state/feedback.json` | 패턴/인사이트 누적 | 작업 완료 시 |
| `.claude/state/ci-session.json` | 현재 CI 세션 정보 | 실시간 |
| `.claude/state/checkpoint-{ts}.json` | 10턴 주기 스냅샷 | 10턴마다 |

#### 복구 절차

```
세션 복구 순서:
1. .claude/state/ci-session.json 읽기 → 마지막 세션 상태 확인
2. .claude/state/current-task.json 읽기 → 중단된 작업 확인
3. .claude/state/work-queue.json 읽기 → 대기열 상태 복구
4. .claude/state/agent-context.json 읽기 → 공유 컨텍스트 로드
5. 중단된 작업이 있으면 재개, 없으면 대기열 맨 앞 작업 시작

컨텍스트 압축 (/compact 트리거, 5턴마다):
- 완료된 작업 요약을 agent-context.json에 저장
- 오래된 상세 로그는 .history/에 아카이브
- 현재 작업 컨텍스트만 메모리에 유지
```

### 5. Sub-agent 통신

#### 표준 출력 포맷

Sub-agent는 아래 JSON 구조로 결과를 반환해야 합니다.

```json
{
  "agent": "agent-name",
  "task_id": "task-uuid",
  "status": "success | failure | partial",
  "result": {
    "summary": "작업 결과 한줄 요약",
    "artifacts": ["생성된 파일 절대경로 목록"],
    "metrics": {
      "files_changed": 0,
      "lines_added": 0,
      "lines_removed": 0,
      "tests_added": 0
    }
  },
  "next_action": "continue | review | commit | rollback | stop",
  "error": null
}
```

#### 에러 코드 체계

| 코드 | 이름 | 의미 | 대응 |
|------|------|------|------|
| `E001` | BUILD_FAIL | 빌드 실패 | L1 재시도 |
| `E002` | TEST_FAIL | 테스트 실패 | L1 재시도 |
| `E003` | COVERAGE_LOW | 커버리지 80% 미달 | L2 롤백 |
| `E004` | LINT_FAIL | clippy/eslint 실패 | L1 재시도 |
| `E005` | REVIEW_FAIL | 코드 리뷰 CRITICAL | L2 롤백 |
| `E006` | SECURITY_ISSUE | 보안 취약점 발견 | L4 중단 |
| `E007` | CONFLICT | git 충돌 | L2 롤백 |
| `E008` | TIMEOUT | 작업 타임아웃 (30분) | L3 스킵 |
| `E009` | QUEUE_EMPTY | 대기열 비어 있음 | 루프 종료 |
| `E010` | UNKNOWN | 알 수 없는 에러 | L4 중단 |

#### 에이전트 간 메시지 전달

```
Orchestrator → Sub-agent:
  .claude/triggers/queue/{task-id}.json

Sub-agent → Orchestrator:
  .claude/triggers/processed/{task-id}-result.json

형식:
{
  "task_id": "uuid",
  "agent": "issue-developer",
  "command": "implement",
  "payload": { ... },
  "timeout_minutes": 30
}
```

---

## IMPORTANT: 에이전트 시작 가이드

**세션 시작 시 반드시 아래 파일들을 먼저 읽으세요:**

```
1. PLAN.md      → 무슨 일을 해야 하는지 (할 일 목록)
2. PROGRESS.md  → 어디까지 진행되었는지 (완료된 작업)
```

### 작업 흐름

```
세션 시작
    │
    ├─→ PLAN.md 읽기 (할 일 확인)
    ├─→ PROGRESS.md 읽기 (진행 상황 확인)
    │
    ▼
작업 수행
    │
    ├─→ 작업 완료 시: PROGRESS.md 업데이트
    ├─→ 새 할 일 발견 시: PLAN.md 업데이트
    │
    ▼
세션 종료
    │
    └─→ .history/YYYY-MM-DD_description.md에 상세 로그 저장
```

### 파일 역할

| 파일 | 목적 | 업데이트 시점 |
|------|------|-------------|
| `PLAN.md` | 해야 할 일, 우선순위, 담당 | 새 작업 추가/완료 시 |
| `PROGRESS.md` | 완료된 작업, 결정사항, 이슈 | 작업 완료마다 |
| `.history/*.md` | 상세 세션 로그 | 세션 종료 시 |

---

## Project Vision

MinKy는 3~9명 소규모 팀의 **암묵지(tacit knowledge)를 형식지로 전환**하고,
팀원 누구나 필요할 때 **자연어로 찾아 활용**할 수 있게 하는 지식 플랫폼입니다.

### 핵심 철학

```
암묵지 → 캡처 → AI 이해 → 연결 → 자연어 검색 → 팀 성장
```

- **태그/분류가 아닌 이해**: AI가 문서를 "읽고 이해"하여 자동으로 연결
- **검색이 아닌 대화**: "이거 어디있지?" 대신 "우리 팀에서 이런 문제 해결한 적 있어?"
- **개인 성장 → 팀 성장**: 한 사람의 학습이 팀 전체의 자산이 됨

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Input Sources                          │
├─────────────┬─────────────┬─────────────┬──────────────────┤
│  Obsidian   │   Safari    │   Slack     │   Direct Input   │
│  Markdown   │   Clipper   │   Messages  │   Chat/Upload    │
└──────┬──────┴──────┬──────┴──────┬──────┴────────┬─────────┘
       │             │             │               │
       └─────────────┴──────┬──────┴───────────────┘
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Knowledge Pipeline                          │
├─────────────────────────────────────────────────────────────┤
│  1. Document Ingestion                                       │
│     - Markdown/HTML/PDF parsing                              │
│     - Metadata extraction                                    │
│                                                              │
│  2. AI Understanding (Claude)                                │
│     - 핵심 주제 추출                                          │
│     - 해결한 문제 식별                                        │
│     - 관련 기술/도구 태깅                                     │
│     - 연결 가능한 지식 파악                                    │
│                                                              │
│  3. Vector Embedding                                         │
│     - Document embeddings                                    │
│     - Chunk-level embeddings                                 │
│     - Semantic indexing                                      │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Knowledge Store                           │
├──────────────────────┬──────────────────────────────────────┤
│   PostgreSQL         │   Vector DB (pgvector/Qdrant)        │
│   - Documents        │   - Embeddings                       │
│   - Metadata         │   - Semantic search                  │
│   - Relations        │   - Similarity matching              │
└──────────────────────┴──────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   RAG Search Layer                           │
├─────────────────────────────────────────────────────────────┤
│  User Question → Query Understanding → Vector Search         │
│       → Context Assembly → Claude Generation → Answer        │
└─────────────────────────────────────────────────────────────┘
```

---

## Tech Stack

### Backend (Rust)
- **Framework**: Axum 0.8
- **Database**: PostgreSQL + pgvector
- **Vector Search**: pgvector (primary) / Qdrant (optional)
- **AI**: Claude API (Anthropic)
- **Embedding**: OpenAI text-embedding-3-small / Voyage AI
- **Async Runtime**: Tokio

### Frontend
- **Framework**: React 18+
- **State**: React Query / Zustand
- **UI**: Tailwind CSS + shadcn/ui
- **Chat**: Real-time WebSocket

### Infrastructure
- **Container**: Docker
- **DB Migration**: sqlx-cli
- **CI/CD**: GitHub Actions

---

## Directory Structure

```
minky/
├── CLAUDE.md                 # 프로젝트 가이드 (이 파일)
├── .history/                 # 세션 로그
├── BK/                       # 백업 파일
├── Docs/                     # 문서
│
├── app/                      # Python Backend (Legacy)
│   ├── middleware/
│   ├── models/
│   ├── routes/
│   ├── schemas/
│   ├── services/
│   │   ├── agents/           # AI 에이전트
│   │   └── llm_providers/    # LLM 제공자
│   └── utils/
│
├── minky-rust/               # Rust Backend (Active)
│   ├── Cargo.toml
│   ├── migrations/           # SQL 마이그레이션
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── config.rs
│       ├── error.rs
│       ├── middleware/       # 인증, 로깅
│       ├── models/           # 데이터 타입, 스키마
│       ├── routes/           # API 엔드포인트
│       ├── services/         # 비즈니스 로직
│       └── utils/
│
├── frontend/                 # React Frontend
│   ├── public/
│   └── src/
│       ├── components/       # UI 컴포넌트
│       ├── hooks/            # 커스텀 훅
│       ├── pages/            # 페이지
│       ├── services/         # API 클라이언트
│       ├── styles/           # CSS/Tailwind
│       ├── utils/            # 유틸리티
│       └── i18n/             # 다국어
│
├── tests/                    # Python 테스트
├── migrations/               # Python DB 마이그레이션
├── logs/                     # 로그 파일
│
└── .claude/                  # Claude Code 설정
    ├── agents/               # 에이전트 정의 (21개)
    ├── commands/             # 커맨드 정의 (23개)
    ├── skills/               # 스킬 정의 (6개)
    ├── config.json           # 전체 시스템 설정
    ├── state/                # 상태 관리
    │   ├── current-task.json
    │   ├── work-queue.json
    │   ├── agent-context.json
    │   ├── feedback.json
    │   └── ci-session.json
    ├── locks/                # 동시 작업 잠금
    ├── pipelines/            # CI/CD 파이프라인 정의
    ├── triggers/             # 외부 트리거
    │   ├── queue/
    │   └── processed/
    ├── logs/ci/              # CI 로그
    ├── backups/              # 상태 백업
    └── references/           # 조사/레퍼런스 저장
        ├── _index.json       # 검색 인덱스
        ├── research/         # 조사 결과
        ├── architecture/     # 아키텍처 패턴
        ├── apis/             # API 문서
        └── best-practices/   # 베스트 프랙티스
```

### 주요 파일 위치

| 목적 | 경로 |
|------|------|
| Rust 진입점 | `minky-rust/src/main.rs` |
| Rust 모델 정의 | `minky-rust/src/models/` |
| Rust API 라우트 | `minky-rust/src/routes/` |
| Rust 서비스 로직 | `minky-rust/src/services/` |
| React 컴포넌트 | `frontend/src/components/` |
| React API 호출 | `frontend/src/services/api.js` |

### 새 기능 추가 시

```
1. 모델 정의     → minky-rust/src/models/{feature}.rs
2. 서비스 구현   → minky-rust/src/services/{feature}_service.rs
3. 라우트 추가   → minky-rust/src/routes/{feature}.rs
4. mod.rs 등록   → 각 디렉토리의 mod.rs에 추가
5. 프론트엔드    → frontend/src/components/{Feature}/
```

---

## Core Features (Implementation Priority)

### Phase 1: Knowledge Understanding (Current Focus)

1. **Document Ingestion Pipeline**
   - [ ] Markdown 파일 업로드/임포트
   - [ ] 메타데이터 추출 (제목, 날짜, 출처)
   - [ ] 청크 분할 (semantic chunking)

2. **AI Document Analysis**
   - [ ] Claude로 문서 분석
   - [ ] 핵심 주제 3-5개 추출
   - [ ] 해결한 문제/인사이트 식별
   - [ ] 관련 기술/도구 자동 태깅

3. **Vector Embedding Storage**
   - [ ] pgvector 설정
   - [ ] 문서 임베딩 생성 및 저장
   - [ ] 청크별 임베딩

### Phase 2: Conversational Search

4. **RAG Search API**
   - [ ] 자연어 질문 → 벡터 검색
   - [ ] 컨텍스트 조합
   - [ ] Claude 답변 생성
   - [ ] 출처 문서 링크

5. **Chat Interface**
   - [ ] 대화형 UI
   - [ ] 스트리밍 응답
   - [ ] 대화 히스토리

### Phase 3: Knowledge Connection

6. **Auto-linking**
   - [ ] 관련 문서 자동 연결
   - [ ] 지식 그래프 시각화
   - [ ] "이것도 볼만해요" 추천

### Phase 4: Tacit Knowledge Capture

7. **Conversation Mining** (Future)
   - [ ] Slack/Teams 연동
   - [ ] 대화에서 지식 추출
   - [ ] 확인 후 자동 저장

---

## API Endpoints (Target)

```
# Document Pipeline
POST   /api/documents/ingest      # 문서 업로드 + AI 분석
GET    /api/documents/{id}        # 문서 조회 (분석 결과 포함)
GET    /api/documents/{id}/related # 관련 문서

# Knowledge Search (RAG)
POST   /api/search/ask            # 자연어 질문 → AI 답변
POST   /api/search/semantic       # 벡터 유사도 검색
GET    /api/search/history        # 검색 히스토리

# Chat
POST   /api/chat/message          # 대화 메시지
GET    /api/chat/sessions         # 대화 세션 목록
WS     /api/chat/stream           # 스트리밍 WebSocket
```

---

## Document Understanding Schema

```rust
/// AI가 분석한 문서 이해 결과
pub struct DocumentUnderstanding {
    pub document_id: Uuid,

    /// 핵심 주제 (3-5개)
    pub topics: Vec<String>,

    /// 한줄 요약
    pub summary: String,

    /// 해결한 문제 (있다면)
    pub problem_solved: Option<String>,

    /// 핵심 인사이트
    pub insights: Vec<String>,

    /// 관련 기술/도구
    pub technologies: Vec<String>,

    /// 누가 알면 좋을까? (역할 기반)
    pub relevant_for: Vec<String>,

    /// 관련 문서 ID
    pub related_documents: Vec<Uuid>,

    /// 분석 일시
    pub analyzed_at: DateTime<Utc>,
}
```

---

## Development Guidelines

### 개발 서버 테스트 필수
변경 후 반드시 개발 서버로 테스트하여 에러 확인

### Session Logging
`.history/YYYY-MM-DD_task_description.md` 형식으로 작업 로그 저장

### Commit Convention
```
feat: 새 기능
fix: 버그 수정
refactor: 리팩토링
docs: 문서
test: 테스트
```

### Code Quality
- Rust: `cargo clippy` 통과
- Frontend: `eslint` + `prettier`
- 테스트 커버리지 80% 목표

---

## Key Design Decisions

### 1. 태그 대신 AI 이해
- 사용자가 태그 고민할 필요 없음
- AI가 문맥을 이해하고 자동 연결
- 검색은 자연어로

### 2. 로컬 우선, 클라우드 선택
- 민감한 팀 지식은 로컬 우선
- Vector DB는 self-hosted 가능
- 외부 AI API 호출만 네트워크 필요

### 3. 점진적 가치 제공
- Phase 1만으로도 사용 가능
- 각 Phase가 독립적 가치 제공
- 팀 상황에 맞게 확장

---

## Success Metrics

- **검색 성공률**: 질문에 관련 답변 찾는 비율
- **지식 재사용률**: 저장된 지식이 실제 활용되는 비율
- **암묵지 전환율**: 대화/경험 → 문서화된 지식
- **팀원 성장**: 다른 팀원 지식으로 문제 해결한 사례

---

## Next Actions

1. [ ] pgvector PostgreSQL 설정
2. [ ] Document Understanding 파이프라인 구현
3. [ ] 벡터 임베딩 저장/검색 API
4. [ ] RAG 기반 자연어 검색
5. [ ] 대화형 인터페이스

---

## References

### 외부 참조
- [RAG Best Practices](https://docs.anthropic.com/claude/docs/retrieval-augmented-generation)
- [pgvector Documentation](https://github.com/pgvector/pgvector)
- [Building a Second Brain](https://www.buildingasecondbrain.com/)

### 내부 레퍼런스 (.claude/references/)

| 파일 | 내용 |
|------|------|
| `_index.json` | 레퍼런스 검색 인덱스 |
| `research/2026-02-18_pkm-tools.md` | PKM 도구 비교 조사 |
| `architecture/rag-patterns.md` | RAG 아키텍처 패턴 |
| `apis/embedding-apis.md` | Embedding API 비교 |

**활용**: `/ref-search` 커맨드로 검색, `/ref-save`로 새 레퍼런스 저장
