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
_(대기)_

## Phase 2 — 데이터 소유 도메인 (결정 C = A1 이관) [🟡]
_(대기)_

## Phase 3 — Flask-only 도메인 분류 [🟢]
_(대기)_

## Phase F — Flask 폐기 [🔴 Mario 승인]
_(대기)_
