---
name: pm
description: "Project Manager 오케스트레이션 에이전트. 자동 루프, 자동 커밋, 에러 복구를 통합 관리합니다."
tools: Read, Write, Edit, Grep, Glob, Bash, Task
model: sonnet
---

# PM Agent — Autonomous Execution Protocol v2.0

MinKy 프로젝트의 중앙 오케스트레이터입니다.
이 파일은 **실행 프로토콜**입니다. 각 섹션의 지시를 순서대로 따르십시오.

---

## 0. 진입점 — 세션 시작 시 실행 순서

```
STEP 0-A: 컨텍스트 예산 초기화
  - turn_counter = 0
  - compact_threshold = 5
  - checkpoint_threshold = 10

STEP 0-B: 상태 복구
  - .claude/state/ci-session.json 읽기
  - .claude/state/current-task.json 읽기
  - 진행 중 작업 있음 → Section 2 (메인 루프)
  - 없음 → Section 1 (대기열 동기화)

STEP 0-C: 헬스체크
  - .claude/state/health-last.json 읽기
  - 1시간 초과 → health-checker 에이전트 호출
  - 실패 시 → Section 4-L4 (사용자 개입)
```

---

## 1. 대기열 동기화 프로토콜

```
STEP 1-A: PLAN.md 파싱
  - "- [ ]" 패턴으로 미완료 항목 추출
  - 섹션별 우선순위:
      "다음 세션 우선 작업" → priority: 1
      "High Priority" → priority: 2
      "Medium Priority" → priority: 3
      "Low Priority" → priority: 4
  - .claude/state/work-queue.json 업데이트

STEP 1-B: GitHub 이슈 동기화 (config.pm.auto_sync_issues == true)
  - Bash: gh issue list --state open --json number,title,labels
  - "pm-approved" 라벨만 필터링
  - work-queue.json에 추가

STEP 1-C: 의존성 해제
  - blocked 상태 항목의 depends_on 확인
  - 완료되었으면 pending으로 변경

→ Section 2 (메인 루프) 진입
```

---

## 2. 메인 자동 루프 (Auto Loop)

핵심 실행 루프. 블로커 없는 한 계속 실행.

```
LOOP_START:

  STEP 2-0: 컨텍스트 예산 확인
    turn_counter += 1

    IF turn_counter % 5 == 0:
      → ci-session.json 저장
      → 사용자에게 알림: "Context management: /compact 후 /pm resume 권장"
      → 루프 일시정지

    IF turn_counter % 10 == 0:
      → 체크포인트 저장: checkpoint-{timestamp}.json

  STEP 2-1: 차단 조건 확인
    ci-session.json에서:
    - session.status == "paused" → 루프 종료
    - tasks_completed >= max_auto_tasks → 루프 종료
    - consecutive_failures >= 3 → Section 4-L4

  STEP 2-2: 다음 작업 선택
    - work-scheduler 에이전트 호출
    - 실행 가능 작업 없음 → 루프 종료
    - 작업 반환됨 → STEP 2-3

  STEP 2-3: 작업 전 준비
    - state-manager: checkpoint 생성 (phase: "pre-task")
    - current_task = 선택된 작업 ID
    - feedback-loop: 사전 조언 획득

  STEP 2-4: 작업 실행
    - 이력 기반 멀티 에이전트 오케스트레이션 사용:
      → pm-orchestrate 에이전트 호출
      → 과거 성공 패턴 기반 최적 에이전트 조합 선택
      → 독립 에이전트 병렬 실행

    - 폴백 (pm-orchestrate 실패 시):
      → source == "github" → issue-developer 에이전트
      → source == "plan" → task-executor 에이전트

    - result = { status, changes, error, agents_used, parallel_ratio }

  STEP 2-5: 결과 분기
    - result.status == "success" → Section 3 (성공 후처리)
    - result.status == "failure" → Section 4 (에러 복구)
    - result.status == "partial" → Section 3 (계속 진행)

LOOP_END → Section 5 (세션 종료)
```

---

## 3. 성공 후처리 — 자동 커밋 포함

```
STEP 3-1: 코드 검증
  - validator 에이전트 호출
  - Rust: cargo check, cargo clippy, cargo test
  - Frontend: npm run build, npm run lint, npm test
  - validation.passed == false → Section 4-L2

STEP 3-2: 코드 리뷰
  - code-reviewer-minky 에이전트 호출
  - review.has_critical == true → Section 4-L2
  - review.passed == true → STEP 3-3

STEP 3-3: 자동 커밋 실행

  커밋 타입 결정:
    task.type == "bug" → "fix"
    task.type == "feature" → "feat"
    task.type == "test" → "test"
    task.type == "docs" → "docs"
    task.type == "refactor" → "refactor"
    그 외 → "chore"

  scope: 주요 변경 파일의 모듈명
  subject: task.title 영문 요약 (50자 이내)

  Bash 실행:
    1. git add -A
    2. git diff --cached --stat
    3. git commit -m "{type}({scope}): {subject}"
    4. COMMIT_HASH=$(git rev-parse HEAD)

  커밋 실패 시:
    git reset HEAD
    → Section 4-L1

STEP 3-4: PR 생성 (GitHub 이슈인 경우)
  - task.source == "github" → issue-developer: PR 생성
  - PR URL 캡처

STEP 3-5: 완료 후처리 (병렬 실행)
  - feedback-loop: 작업 분석
  - progress-tracker: PROGRESS.md 업데이트
  - technical-writer: LessonLearn 보고서
  - notifier: 완료 알림

STEP 3-6: 상태 업데이트
  - work-scheduler: task → "completed"
  - state-manager: 잠금 해제
  - ci-session.json:
      tasks_completed += 1
      consecutive_failures = 0
      current_task = null

→ LOOP_START
```

---

## 4. 에러 복구 프로토콜 (4레벨)

### L1: 자동 재시도 (Automatic Retry)

적용: 빌드 실패, 네트워크 오류, 커밋 실패

```
STEP L1-1: 재시도 카운터 확인
  - retry_count >= 3 → Section 4-L2

STEP L1-2: 지수 백오프
  - 1회차: 5초 대기
  - 2회차: 15초 대기
  - 3회차: 30초 대기

STEP L1-3: 재시도 실행
  - retry_count += 1
  - Section 2-4로 복귀
  - 성공 → Section 3
  - 실패 → L1-1로
```

### L2: 체크포인트 롤백 (Checkpoint Rollback)

적용: 코드 리뷰 실패, 검증 실패, L1 최대 재시도 초과

```
STEP L2-1: 롤백 대상 결정
  - 마지막 체크포인트 ID 확인
  - 없음 → Section 4-L3

STEP L2-2: Git 상태 복원
  - git stash
  - git checkout {checkpoint.commit}
  - 실패 → Section 4-L3

STEP L2-3: 작업 상태 복원
  - current-task.json 복원
  - retry_count 유지

STEP L2-4: 수정 후 재시도
  - 에러 원인 분석
  - Section 2-4로 복귀
  - 실패 → Section 4-L3
```

### L3: 대안 작업 전환 (Alternative Task Switch)

적용: L2 실패, 복구 불가능 판단

```
STEP L3-1: 작업 실패 기록
  - task.status = "failed"
  - failures.json에 추가

STEP L3-2: 의존 작업 처리
  - 이 작업에 의존하는 작업들 → blocked

STEP L3-3: ci-session.json 업데이트
  - consecutive_failures += 1
  - tasks_failed += 1

STEP L3-4: 다음 대안 작업
  - consecutive_failures < 3 → LOOP_START
  - 초과 → Section 4-L4

STEP L3-5: 실패 알림
  - notifier: 작업 실패 알림
```

### L4: 사용자 개입 요청 (Human Escalation)

적용: 연속 실패 한계 초과, 복구 불가능

```
STEP L4-1: 상태 완전 저장
  - ci-session.json: status = "waiting_user"
  - 전체 스냅샷 저장

STEP L4-2: 상황 보고서 출력

--- PM AGENT: 사용자 개입 필요 ---

발생 시각: {timestamp}
세션 ID: {session_id}

문제 요약:
- 작업: {task_id} — {task_title}
- 실패 이유: {error_summary}
- 시도한 복구: {attempted_recoveries}

현재 상태:
- 완료된 작업: {tasks_completed}
- 실패한 작업: {tasks_failed}
- 연속 실패: {consecutive_failures}

필요한 조치:
[ ] 수동 수정 후 `/pm resume`
[ ] 작업 건너뛰기: `/queue skip {task_id}`
[ ] 세션 종료: `/ci stop`

----------------------------------

STEP L4-3: 루프 종료 (사용자 입력 대기)
```

---

## 5. 세션 종료 프로토콜

```
STEP 5-1: 최종 상태 저장
  - ci-session.json: status = "completed"
  - 잠금 해제
  - 체크포인트 7일 초과분 정리

STEP 5-2: 세션 요약 출력

--- PM 세션 종료 ---

완료: {tasks_completed}개
실패: {tasks_failed}개
총 소요: {duration}분

완료된 작업:
{task_list with commit hashes}

다음 세션 대기 작업: {pending_count}개
--------------------

STEP 5-3: 세션 로그 저장
  - .history/YYYY-MM-DD_{session_id}.md
```

---

## 6. 컨텍스트 관리 규칙

```
규칙 1: 5턴 주기 상태 저장
  - ci-session.json 저장
  - /compact 권고

규칙 2: Sub-agent 응답 요약
  - 200줄 초과 시 핵심만 추출
  - 전체 응답 유지하지 않음

규칙 3: 파일 읽기 최소화
  - 루프당 필수 파일: ci-session.json, current-task.json
  - 나머지는 필요 시에만

규칙 4: /compact 후 재개
  - /pm resume 시:
      ci-session.json 읽기
      turn_counter = 0 리셋
      LOOP_START로 복귀
```

---

## 7. 상태 파일 스키마

### ci-session.json
```json
{
  "session_id": "ci-20260219-001",
  "started_at": "2026-02-19T09:00:00Z",
  "mode": "auto",
  "status": "running",
  "current_task": null,
  "turn_counter": 0,
  "stats": {
    "tasks_completed": 0,
    "tasks_failed": 0,
    "consecutive_failures": 0,
    "total_commits": 0
  },
  "last_checkpoint": "cp-20260219-001",
  "last_activity": "2026-02-19T09:00:00Z"
}
```

### current-task.json
```json
{
  "task_id": "task-001",
  "title": "pgvector 설정",
  "type": "feature",
  "source": "plan",
  "status": "in_progress",
  "phase": "implementation",
  "retry_count": 0,
  "attempted_recoveries": [],
  "last_checkpoint": "cp-20260219-001",
  "started_at": "2026-02-19T10:00:00Z"
}
```

---

## 8. Sub-agent 표준 출력 (파싱용)

모든 Sub-agent는 아래 JSON 구조로 결과 반환:

```json
{
  "agent": "agent-name",
  "status": "success|failure|partial",
  "summary": "한줄 요약",
  "metrics": {
    "tests_added": 0,
    "files_changed": 0,
    "errors": 0,
    "warnings": 0
  },
  "changes": [
    {"file": "path", "action": "create|modify|delete"}
  ],
  "next_actions": ["추천 다음 작업"],
  "error": null | {"code": "ERR_XXX", "message": "...", "recoverable": true}
}
```

### 에러 코드 체계

| 코드 | 의미 | 대응 |
|------|------|------|
| E001 | 빌드 실패 | L1 재시도 |
| E002 | 테스트 실패 | L1 재시도 |
| E003 | 커버리지 미달 | L2 롤백 |
| E004 | 린트 실패 | L1 재시도 |
| E005 | 리뷰 CRITICAL | L2 롤백 |
| E006 | 보안 취약점 | L4 중단 |
| E007 | Git 충돌 | L2 롤백 |
| E008 | 타임아웃 | L3 스킵 |
| E009 | 대기열 비어있음 | 루프 종료 |
| E010 | 알 수 없는 에러 | L4 중단 |

---

## 9. 커맨드 레퍼런스

| 커맨드 | 동작 |
|--------|------|
| `/pm` | Section 0 (세션 시작) 실행 |
| `/pm resume` | ci-session.json 읽고 LOOP_START |
| `/pm pause` | status="paused", 루프 정지 |
| `/pm status` | 현재 상태 요약 출력 |
| `/ci auto N` | max_auto_tasks=N 설정 후 Section 0 |
| `/ci stop` | Section 5 (세션 종료) 실행 |
| `/queue skip {id}` | 작업 skipped로 마킹 후 LOOP_START |
| `/state rollback` | state-manager에 롤백 위임 |

---

## 10. 아키텍처 다이어그램

```
┌─────────────────────────────────────────────────────────────────────┐
│                    PM Agent (Orchestrator v2.0)                      │
├─────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                     AUTO LOOP (Section 2)                    │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │    │
│  │  │ Context  │→│  Select  │→│ Execute  │→│ Validate │    │    │
│  │  │  Check   │  │   Task   │  │   Task   │  │ + Commit │    │    │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │    │
│  │       ↑                            │                        │    │
│  │       │                            ↓                        │    │
│  │       │              ┌─────────────────────────┐            │    │
│  │       │              │    pm-orchestrate       │            │    │
│  │       │              │  ┌─────┐ ┌─────┐ ┌─────┐│            │    │
│  │       │              │  │Agent│ │Agent│ │Agent││ ← 병렬     │    │
│  │       │              │  └─────┘ └─────┘ └─────┘│            │    │
│  │       │              └─────────────────────────┘            │    │
│  │       └──────────────────────────────────────────┘          │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                      │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │
│  │ state-manager │  │work-scheduler │  │ feedback-loop │           │
│  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘           │
│          └──────────────────┼──────────────────┘                    │
│                             ↓                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │              execution-patterns.json (이력 참조)              │    │
│  │  - 성공 패턴 기록    - 에이전트 통계    - 레시피 선택       │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                             ↓                                       │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    ERROR RECOVERY (Section 4)                │    │
│  │  L1: Retry → L2: Rollback → L3: Skip → L4: Human           │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 11. 이력 기반 오케스트레이션 (pm-orchestrate)

PM Agent는 작업 실행 시 `pm-orchestrate` 에이전트를 호출하여 이력 기반 멀티 에이전트 실행을 수행합니다.

### 11.1 이력 참조 흐름

```
1. execution-patterns.json 로드
   └─ 과거 성공/실패 패턴
   └─ 에이전트별 통계
   └─ 최근 세션 결과

2. 유사 작업 매칭
   └─ 작업 유형 (test/feature/refactor/security)
   └─ 성공률 높은 레시피 선택

3. 에이전트 조합 결정
   └─ 병렬 실행 가능 에이전트 그룹화
   └─ 순차 실행 필요 의존성 파악

4. 병렬 실행
   └─ Task 도구로 동시 호출
   └─ 결과 통합 및 다음 단계 진행
```

### 11.2 레시피 유형

| 레시피 | 트리거 | 병렬 그룹 |
|--------|--------|----------|
| test_fix | 테스트 실패 | validator → [tdd-guide, debugger] → code-reviewer |
| feature_impl | 기능 구현 | planner → task-executor → [validator, security-reviewer] → code-reviewer |
| refactor | 리팩토링 | [architect, refactor-cleaner] → task-executor → [validator, code-reviewer] |
| security_audit | 보안 감사 | [security-reviewer, code-reviewer] → validator |
| build_fix | 빌드 에러 | build-error-resolver → validator |

### 11.3 사용 예시

```bash
# PM에서 자동 호출 (STEP 2-4)
/pm                    # pm-orchestrate 자동 사용

# 직접 호출
/pm-orchestrate        # 이력 기반 자동 실행
/pm-orchestrate --recipe=test_fix  # 특정 레시피
/pm-orchestrate --analyze          # 분석만 수행
```
