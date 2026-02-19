---
name: pm-orchestrate
description: "이력 기반 멀티 에이전트 오케스트레이션. 과거 실행 참조하여 최적 에이전트 조합 실행."
---

# /pm-orchestrate 커맨드

과거 실행 이력을 분석하여 최적의 에이전트 조합을 병렬로 실행합니다.

## 사용법

```bash
/pm-orchestrate                    # 자동 작업 감지 및 실행
/pm-orchestrate --recipe=test_fix  # 특정 레시피 지정
/pm-orchestrate --analyze          # 이력 분석만 수행
```

## 실행 프로토콜

### 1단계: 이력 로드

```
READ .claude/state/execution-patterns.json
READ .history/2026-02-*.md (최근 10개)
READ PROGRESS.md (현재 상황)
```

### 2단계: 작업 분류 및 레시피 선택

| 키워드 | 레시피 | 에이전트 조합 |
|--------|--------|--------------|
| test, 테스트, fix | test_fix | validator → tdd-guide+debugger → code-reviewer |
| feature, 기능, implement | feature_impl | planner → task-executor → validator+security → code-reviewer |
| refactor, 리팩토링 | refactor | architect+refactor-cleaner → task-executor → validator+code-reviewer |
| security, 보안 | security_audit | security-reviewer+code-reviewer → validator |
| docs, 문서 | docs_update | doc-updater + technical-writer |
| build, 빌드, error | build_fix | build-error-resolver → validator |

### 3단계: 병렬 실행

```
FOR each phase IN recipe.phases:
  IF phase.parallel:
    # 병렬 실행 (Task 도구 동시 호출)
    results = Task[agent1], Task[agent2], Task[agent3]
  ELSE:
    # 순차 실행
    result = Task[agent]

  # 결과 검증
  IF any_failure(results):
    → 복구 프로토콜 실행
```

### 4단계: 결과 기록

```
UPDATE .claude/state/execution-patterns.json
  - 성공률 갱신
  - 에이전트 통계 갱신
  - recent_sessions 추가

UPDATE PROGRESS.md
  - 완료된 작업 기록
```

## 출력 예시

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
PM Orchestrate: 이력 기반 멀티 에이전트 실행
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 이력 분석:
  - 유사 작업: 3개 (test_fix 패턴)
  - 성공률: 95%
  - 평균 소요: 45초

🚀 실행 계획:
  Phase 1 (병렬): validator
  Phase 2 (병렬): tdd-guide, debugger
  Phase 3 (순차): code-reviewer

[Phase 1/3] ━━━━━━━━━━━━━━━━━━━━ 100%
  ✅ validator: 488 tests passed

[Phase 2/3] ━━━━━━━━━━━━━━━━━━━━ 100%
  ✅ tdd-guide: 분석 완료
  ✅ debugger: 원인 파악

[Phase 3/3] ━━━━━━━━━━━━━━━━━━━━ 100%
  ✅ code-reviewer: approved

✅ 완료 (38초) - 순차 대비 42% 단축
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## 옵션

| 옵션 | 설명 |
|------|------|
| `--recipe=NAME` | 특정 레시피 강제 사용 |
| `--max-parallel=N` | 최대 병렬 에이전트 수 |
| `--no-history` | 이력 참조 없이 실행 |
| `--analyze` | 분석만 수행, 실행 안함 |
| `--dry-run` | 실행 계획만 표시 |
