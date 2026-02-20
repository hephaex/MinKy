---
name: pm-orchestrate
description: "이력 기반 멀티 에이전트 오케스트레이션. 과거 실행 참조하여 최적 에이전트 조합 실행."
invocation: /pm-orchestrate
keywords:
  - orchestrate
  - multi-agent
  - history
  - 이력
  - 에이전트
  - 병렬
  - parallel
---

# PM Orchestrate Skill

과거 실행 이력을 분석하여 최적의 에이전트 조합을 병렬로 실행합니다.

## 실행 단계

### 1. 이력 분석

```
READ .claude/state/execution-patterns.json
ANALYZE:
  - 작업 유형별 성공률
  - 에이전트별 성능 통계
  - 최근 세션 결과
```

### 2. 레시피 선택

현재 작업을 분석하여 최적의 레시피를 선택합니다:

| 작업 유형 | 레시피 | 병렬 실행 비율 | 사전 검증 |
|----------|--------|--------------|----------|
| 테스트 수정 | test_fix | 66% | - |
| 기능 구현 | feature_impl | 50% | cargo check, npm build |
| 리팩토링 | refactor | 60% | - |
| 보안 감사 | security_audit | 66% | - |
| 빌드 수정 | build_fix | 50% | - |

### 2.5. 사전 검증 (Pre-check)

feature_impl 패턴은 구현 전 사전 검증을 수행합니다:

```python
if recipe.pre_check and recipe.pre_check.enabled:
    for command in recipe.pre_check.commands:
        result = run_bash(command)
        if not result.success:
            if recipe.pre_check.on_failure == "abort_with_build_fix":
                # 빌드 수정 레시피로 자동 전환
                return execute_recipe("build_fix")
            elif recipe.pre_check.on_failure == "abort":
                return error("Pre-check failed")
            # "continue" → 경고 후 계속 진행
```

### 3. 병렬 에이전트 실행

```python
# 각 Phase의 병렬 그룹에서 동시 실행
for phase in recipe.phases:
    if phase.parallel:
        # Task 도구로 동시에 여러 에이전트 호출
        results = parallel_execute([
            Task(subagent_type=agent, prompt=task_prompt)
            for agent in phase.agents
        ])
    else:
        result = sequential_execute(phase.agents)
```

### 4. 결과 기록

```
UPDATE execution-patterns.json:
  - 성공률 갱신
  - 에이전트 통계 업데이트
  - recent_sessions 추가

OUTPUT:
  - 실행 요약
  - 병렬 실행 효율
  - 다음 추천 작업
```

## 이력 참조 방식

### execution-patterns.json 구조

```json
{
  "patterns": {
    "test_fix": {
      "success_rate": 0.95,
      "agents": ["validator", "tdd-guide", "debugger", "code-reviewer"],
      "parallel_groups": [
        ["validator"],
        ["tdd-guide", "debugger"],
        ["code-reviewer"]
      ]
    }
  },
  "agent_stats": {
    "validator": {
      "success": 52,
      "fail": 3,
      "avg_time_sec": 25,
      "best_with": ["code-reviewer", "tdd-guide"]
    }
  },
  "recent_sessions": [
    {
      "date": "2026-02-20",
      "task_type": "test_fix",
      "success": true,
      "duration_sec": 45
    }
  ]
}
```

### 유사 작업 매칭

1. 현재 작업의 키워드 추출
2. patterns에서 일치하는 유형 찾기
3. recent_sessions에서 유사 작업 참조
4. 성공률 높은 에이전트 조합 선택

## 사용 예시

```bash
# 자동 실행 (이력 기반)
/pm-orchestrate

# 특정 레시피 지정
/pm-orchestrate --recipe=test_fix

# 분석만 수행
/pm-orchestrate --analyze

# 병렬 수준 조정
/pm-orchestrate --max-parallel=3
```

## PM Agent 통합

PM Agent의 STEP 2-4에서 자동으로 pm-orchestrate가 호출됩니다:

```
STEP 2-4: 작업 실행
  → pm-orchestrate 에이전트 호출
  → 이력 기반 최적 에이전트 조합 선택
  → 독립 에이전트 병렬 실행
```
