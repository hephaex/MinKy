# minky 모노레포 미사용/데드코드 전수 감사 (2026-10-25)

## 개요

Anthropic Provider 제거 ralplan 완료 후, minky 모노레포 전체(백엔드 Rust + 프런트엔드 React)를 대상으로 미사용 의존성·데드코드·미참조 pub 선언을 전수 감사했다. knip/depcheck/ts-prune/cargo-udeps가 PATH에 없어 수동 grep 휴리스틱으로 진행했고, 최종 상태는 전체 `cargo test` **2026 passed / 0 failed / 13 ignored**.

## 해결 방법

### 1. 도구 확인이 감사 방식을 결정한다
- **무엇**: knip(JS), depcheck(JS), ts-prune(TS), cargo-udeps(Rust)가 모두 PATH에 없음을 확인.
- **왜 중요**: 도구 기반 감사가 불가능하면 grep 휴리스틱(선언 라인 제외 전수 참조 카운트)으로 대체해야 하고, 이때 openapi.rs·tests·benches 포함 여부가 오판을 만든다.
- **적용**: 크레이트별로 "선언 라인 외 코드베이스 등장 0회"를 기준으로 미참조 판정.

### 2. Cargo deps는 사용 흔적을 두 갈래로 검증한다
- **무엇**: `image`, `futures`가 실사용 없음을 확인 후 Cargo.toml에서 제거. `mime_guess`/`tempfile`(File Processing), `async-trait`(Async utilities)만 남김.
- **왜 중요**: lib 크레이트 특성상 `pub` 항목은 rustc `dead_code` 경고 대상에서 빠진다 — `cargo check --all-targets` warning 0이 "데드코드 없음"을 보장하지 않는다.
- **적용**: deps 제거 후 `cargo check` GREEN + 전체 `cargo test` 재검증까지 완료해야 종결.

### 3. pub 선언 전수 감사는 스크립트로 재산출한다
- **무엇**: python3 센서스 스크립트로 pub 선언 총 1,644개 중 미참조 72개 / 33파일 확인 (초기 수동 집계 65개/31파일 대비 +7/+2 오차 정정).
- **왜 중요**: 초기 집계는 파일별 필터 누락으로 과소 계산됐고, 제거 승인 판단의 근거 수치라 재산출 전에는 실행 보류가 정답이었다.
- **적용**: 분포 — services 36/14파일, models DTO 12/9파일, nitl_config.rs 10, pipeline 9/5파일, routes/plot.rs 2, 나머지 3.

### 4. `#[allow(dead_code)]`는 per-item 원칙으로 분류한다
- **무엇**: Flask-compat 쿼리 파라미터 5곳(`categories.rs` flat/include_inactive, `documents.rs` group_by, `slack.rs` since/state)은 KEEP, `mcp_server.rs:361`(row-shape 문서화 근거)은 KEEP, 단일 임베딩 경로는 제거.
- **왜 중요**: hwp2md Phase 2d dead-code review 선례 — spec 상수·호환 쿼리는 유지, 진짜 죽은 경로만 제거, 모듈 단위 allow 금지.
- **적용**: `#[allow(dead_code)]`가 붙은 항목마다 "왜 남기는가"의 명시적 근거를 코드 주석 또는 문서로 남긴다.

### 5. doc-impact는 ralplan 종료 후에도 남는다
- **무엇**: `Docs/GETTING_STARTED.md` line 497 env block, `Docs/ARCHITECTURE.md` References 외부 링크, `Docs/ARCHITECTURE_ANALYSIS.md` 의존성 다이어그램, `Docs/RUST_MIGRATION_PLAN.md` crate 매핑 표, `minky-rust/.env.example` line 20에 Anthropic 잔여가 남았음을 정리.
- **왜 중요**: 멀티라인 교체 패턴이 헤더 라인에 고정되면 파일 내 두 번째 발생을 놓친다 (GETTING_STARTED.md 3회 OK 후 line 497 잔여).
- **적용**: Anthropic 관련 교체 후에는 전체 파일을 `grep ANTHROPIC|Claude`로 재검색해 잔여를 잡는다.

## 학습 포인트

- **도구 없음 ≠ 감사 불가**: grep 휴리스틱 + 스크립트 재산출로 도구 기반 감사와 동등한 근거 수치를 만들 수 있다.
- **warning 0 ≠ 데드코드 0**: lib 크레이트 pub 항목은 컴파일러 경고 대상에서 빠진다.
- **제거는 개별 승인**: 72개 일괄 제거는 파괴적 작업 — 스크립트 수치 확정 후 사용자 승인 대기가 절차다.

## 레퍼런스

- 감사 상세: `minky-rust/notes/2027-unused-dead-code-audit.md` (단계 1–8)
- hwp2md Phase 2d dead-code review (2026-04-22) — per-item `#[allow(dead_code)]` 선례, commits `5e297d7`/`20909b8`
- Anthropic 제거 ralplan 계획: `minky-rust/notes/2026-07-19_ralplan_anthropic제거_계획수정_승인대기.md`

#rust #refactor #bugfix #backend

### 6. [Dead Orphan Module] 모듈 선언 부재 파일은 컴파일 그래프 밖 — census가 잡지 못하는 유령

**무엇**: `src/routes/plot.rs`(54줄)가 파일 시스템에는 존재하지만 어디에서도 `mod plot;`으로 선언되지 않아 Rust 컴파일 그래프에서 완전히 제외된 상태였다. 내부는 이미 깨져 있었다 — `AppError::GpmNotConfigured`/`GpmError`(src/error.rs에 없는 변형)와 `crate::models::plot::PlotRecord`(파일 자체가 없음)를 참조. wired 시 컴파일 불가. 유래는 `c5ffb35a` CT306 fork-point import.

**왜 중요**: `#[allow(dead_code)]` census와 cargo check/warning은 모두 이런 유령 파일을 잡지 못한다 — 컴파일 안 되는 파일은 warning도 안 난다. dead-code 감사의 마지막 게이트는 "src/ 아래의 모든 .rs가 컴파일 그래프에 포함되는가" 확인이어야 한다. `mod` 선언 목록 대조가 유일한 탐지법.

**적용**: minky-rust에서 `grep -rn "mod plot" routes/mod.rs models/mod.rs lib.rs main.rs` → 0 matches로 확인. 파일 삭제는 파괴적 작업이므로 사용자 승인 대기 중.
