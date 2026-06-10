# Minky 백엔드 통합 실행 플랜 (Option A: React + Rust)

> 목표 독자: **Sonnet 4.6** (단독 실행). 어려운 판단은 Opus가 미리 끝냈고, **Sonnet이 안전하게
> 할 수 있는 부분과, 사람(Mario) 승인이 필요한 부분을 등급으로 분리**했다.
> 작성: Opus 4.8, 2026-06-10 (리뷰·개정 반영). 근거: `~/.claude/references/2026-06-10_minky_backend_consolidation_adr.md`

## ⚠️ 먼저 읽어라 — 이 플랜의 현실 (정직한 평가)

Option A는 "Rust를 배선만 하면 됨"이 아니다. 감사 결과 **Rust는 Flask와 다른 스키마·DB·auth로
지어진 그린필드 재구현**이다. 따라서 **전체 컷오버의 ~70%는 Sonnet이 안전히 할 수 있으나,
스키마/데이터 통합과 auth는 사람 체크포인트가 필요**하다. 아래 등급을 반드시 지켜라.

- 🟢 **Sonnet-safe**: 기계적·가역적·테스트로 검증됨. 단독 진행 OK.
- 🟡 **Sonnet-care**: 먼저 "검증 태스크"로 사실을 확정한 뒤 진행. 불확실하면 멈춰라.
- 🔴 **Human-checkpoint**: Sonnet은 **분석·제안만** 만들고 **실행 전 Mario 승인**. 비가역/대규모.

## 1. 확정된 블로커 (재조사 불필요, 근거 라인 포함)

1. **별도 DB**: Flask=`minky`(문서 4,545건), Rust=`minky_rust`(별도, 사실상 빈). compose:9 vs :53.
2. **문서 PK 충돌**: Flask `id INTEGER`(models/document.py:47) vs Rust `id UUID`(migrations/001:33). **Rust에서 문서 UUID를 참조하는 파일 41개**(models 20+/routes/services) — 재정렬 시 전부 영향.
3. **본문 필드명**: Flask `markdown_content`(document.py:52) vs Rust `content`(001:35).
4. **응답 형태**: Flask `{documents:[to_dict_lite], pagination:{page,per_page,total}}` vs Rust `{success, data:[DocumentResponse], meta:{total,page,limit,total_pages}}`.
5. **Auth (모순·미확정)**: 프론트 api.js는 `withCredentials:true`+**Authorization 헤더 주입 없음**(services/api.js). Flask 로그인은 토큰을 **body로 반환**(auth.py:115) sub=`str(user.id)`. Rust는 **Bearer 헤더 전용**(middleware/auth.rs:24)+`sub:i32`. → **프론트가 토큰을 어떻게 전송하는지 정적으로 불명** = S0에서 반드시 실측.
6. **Flask-only 도메인**: org_roam, document_clustering, collaboration, ml_analytics, chat — Rust 부재.

## 2. 전략 결정 (Sonnet 재결정 금지 / 🔴는 사람 승인 후)

- **결정 A — 응답 형태 방향 [🟢 고정 규칙]**: **항상 Rust 응답을 Flask 형태에 맞춘다**(프론트 무변경). 프론트는 이미 테스트된 자산이므로 변경 표면을 최소화. (예외 없음. 슬라이스마다 다시 고르지 말 것.)
- **결정 B — 전환 배선 [🟢]**: nginx **경로 기반 점진 전환**. 이관 완료 도메인만 `location /api/<도메인> → rust-backend:8000`, 나머지 `/api → app:5000`. 프론트 base URL `/api` 불변.
- **결정 C — 스키마/데이터 [✅ 확정: A1 데이터 이관]** (Opus 2026-06-10):
  - **방향**: 라이브 데이터를 `minky`(Flask) → `minky_rust`(Rust UUID 스키마)로 **이관**. **Rust 코드·스키마·1842 테스트는 불변(green 유지).** Flask `minky`는 **읽기 전용·무변경**(완전 가역).
  - **근거**: ① 위험을 41파일·1842테스트로 확산시키는 A2 대신 **검증 게이트 있는 마이그레이션 스크립트 1개에 집중** ② go-forward 스키마(UUID)를 장기 설계로 보존 ③ 원본 무변경 → 가역.
  - **가드레일(Sonnet 준수)**: (a) 결정적 id 매핑 int→uuid(uuidv5 또는 매핑테이블, **idempotent**) — 모든 FK(document_tags/comments/versions/attachments/embeddings)에 동일 적용. (b) 필드 매핑 `markdown_content→content`; author/html_content/document_metadata는 minky_rust에 **컬럼 추가**(additive, 빈 DB라 안전). (c) **검증 게이트(baram 패턴)**: 테이블별 row-count 일치 + 표본 N개 content 해시 일치 + FK orphan 0. (d) 임베딩: Flask는 OpenSearch 검색이라 pgvector 임베딩이 `minky`에 없을 수 있음 → 이관 후 **재생성**(embedding 파이프라인). (e) **가정**: `minky_rust`는 그린필드(빈 DB) — **CAS 복구 후 실확인, 실데이터 있으면 에스컬레이트.**
  - **컷오버는 Phase F에서 Mario 최종 승인**(프로덕션 전환만 🔴).
- **결정 D — auth [🟡]**: §4 Phase 0에서 **실측으로 전송 방식 확정 후** Rust를 그에 맞춘다(쿠키면 쿠키 읽기 추가, body+헤더면 프론트 헤더 주입 확인). JWT 시크릿(`JWT_SECRET_KEY`)·HS256·문자열 sub 통일.

## 3. Sonnet 실행 규칙

1. **Vertical Slice + Demo Gate**: 슬라이스 = 도메인 관통 + **통과 테스트**. 테스트 없는 완료 금지.
2. **검증 명령(이 환경에서 검증됨)**:
   - Rust: `cd minky-rust && SQLX_OFFLINE=true cargo test --lib <module>`, `cargo clippy --lib --all-targets`(경고 0). sqlx는 런타임 문자열 → **DB 없이 컴파일·테스트 가능**.
   - 프론트: `cd frontend && CI=true npx react-scripts test --watchAll=false --testPathPattern='<X>'`.
   - Flask(대조): `legacy/python-backend`에서 `../../.venv/bin/python -m pytest tests/<X>`(venv에 pytest 설치돼 있음).
3. **로컬 한계 [중요]**: **CAS 다운 → 라이브 DB·실배포 검증 불가.** 로컬에선 **컴파일+단위/통합 테스트까지만**이 Demo Gate. "프론트↔Rust↔실DB" 통합과 nginx 실프록시는 **CAS 복구 후** 별도 검증(각 슬라이스에 `[CAS-deferred]` 체크 남겨라). **풀스택 기동 시도하지 말 것**(로컬에 minky DB 없음).
4. **가역성**: nginx는 한 `location`씩. 문제 시 그 줄만 Flask로 되돌림. 라이브 `minky` 테이블 **ALTER/DROP/UPDATE/DELETE 금지**(컬럼 추가만, 백업+개별 승인 — `destructive-ops.md`).
5. **에스컬레이트(멈추고 Mario에게)**: ① 🔴 작업 실행 직전 ② 라이브 데이터 변경 필요 ③ §2 전략 변경 필요 ④ auth 실측이 모순일 때 ⑤ 동일유형 슬라이스 3연속.

## 4. 단계별 로드맵 (등급 표시)

### Phase 0 — Auth 호환 [🟡 검증 먼저]
0. **[검증 태스크]** 프론트가 토큰을 어떻게 보내는지 실측: `grep -rnE "Authorization|setItem.*token|defaults.headers|withCredentials|Cookies" frontend/src` + `frontend/src/contexts`/auth 관련 컴포넌트 정독. 결과를 이 문서 §6에 기록. **이게 모순/불명이면 멈추고 Mario에게.**
1. 확정된 전송 방식에 맞춰 Rust `middleware/auth.rs` 수정: (쿠키면) 쿠키에서 JWT 읽기 fallback 추가 / (헤더면) 프론트가 헤더를 붙이는지 확인. `Claims.sub`를 **String**으로(현 i32) 통일, JWT 시크릿/알고리즘 Flask와 동일 env.
2. **Demo Gate [🟢 로컬]**: Rust 단위테스트 — Flask 형식 토큰(동일 시크릿·HS256·문자열 sub)을 Rust가 검증 성공. `cargo test --lib middleware::auth`. **[CAS-deferred]** 실제 프론트 로그인→Rust 호출 200.

### Phase 1 — DB-free 도메인으로 배선 증명 [🟢 Sonnet-safe / 권장 첫 슬라이스]
스키마 늪을 건드리지 않고 auth+nginx+형태적응 패턴을 검증한다.
- **대상: query expansion**(이미 이번 세션에 solar-open2 구현됨, `routes/search.rs` 또는 별도) — LLM만 쓰고 documents 테이블 무관.
- 절차: nginx `location /api/search/expand → rust-backend:8000` 추가 → Rust 응답을 (결정A대로) 프론트 기대 형태로 → 프론트에서 호출.
- **Demo Gate [🟢]**: Rust 통합테스트 green + (형태 변경 시) 프론트 테스트. **[CAS-deferred]** 실호출.
- 효과: **Slice B(미연결이던 Rust solar 확장)가 비로소 라이브 경로에 올라옴.**

### Phase 2 — 데이터 소유 도메인 [🟡 결정C=A1 확정됨, 실행은 care]
documents/tags/categories 등. **결정 C=A1**이므로 Rust 코드는 안 바꾼다. 데이터를 Rust 스키마로 옮기는 **마이그레이션 자산**을 만든다.
1. **마이그레이션 스크립트 작성 [🟡]**: `minky`→`minky_rust` (가드레일 결정C 준수). id 매핑·필드 매핑 로직은 **단위테스트로 검증**(int→uuid 결정성, 필드 변환). 원본 `minky`는 SELECT만.
2. **minky_rust 스키마 보강 [🟢]**: 누락 컬럼(author/html_content/document_metadata) **추가 마이그레이션**(additive). `cargo test --lib` green 유지.
3. **검증 쿼리 작성 [🟢]**: row-count 일치 / 표본 content 해시 / FK orphan 0.
4. **결정A 적용 [🟢]**: 각 엔드포인트 Rust 응답을 Flask 형태로(§5). nginx 도메인 전환.
5. **Demo Gate**: 마이그레이션 스크립트 단위테스트 + 도메인 Rust 통합테스트 + 프론트 테스트 green. **[CAS-deferred]** 실제 이관 실행 + row-count 검증(CAS 복구 후). **실데이터 이관·컷오버 직전 Mario 승인.**

### Phase 3 — Flask-only 도메인 처리 [🟢 grep → 분류]
각각 `grep -rnE "/(org.roam|clustering|collaboration|ml.analytics|chat)" frontend/src`:
- 호출 0 → `[DROP]`(포팅 안 함, 이 문서에 기록). 호출 있음 → `[PORT]` 백로그.

### Phase F — Flask 폐기 [🔴 사람 승인]
모든 프론트 호출이 Rust로 갈 때만. compose에서 `app` 제거, nginx `/api` 전부 Rust. **Mario 승인 + 전체 E2E 후.**

## 5. 응답 형태 대조 템플릿 (슬라이스마다 채움)
```
도메인/엔드포인트: GET /api/documents
Flask:  { "documents":[{id:int,title,markdown_content,...to_dict_lite}], "pagination":{page,per_page,total} }
Rust :  { "success":true, "data":[{id,title,content,...}], "meta":{total,page,limit,total_pages} }
적용(결정A): Rust 응답을 Flask 형태로 변환(키 documents/pagination, per_page, markdown_content, id타입)
프론트 영향(무변경 목표): DocumentList.js가 response.data.documents/.pagination 사용 → 그대로 동작해야 함
```

## 6. 검증으로 채울 칸 (2026-06-10 Phase 0 실측 완료)

- [x] **프론트 토큰 전송 방식: 없음 (무인증 동작)**
  - `api.js`: `withCredentials: true` (Flask session 쿠키용)이지만 JWT 토큰을 어디에도 저장하지 않음
  - `authService.login()`: 서버 응답 body에서 `access_token` 수신 → `sessionStorage('user')` 저장 **끝** (토큰 자체 미저장)
  - Authorization 헤더 인터셉터 없음
  - 일부 컴포넌트(DateSidebar 등)가 `localStorage.getItem('token')` 시도 → 항상 null (저장되지 않음)
  - `authService.getToken()` 호출하는 컴포넌트 존재 → 하지만 `authService`에 해당 메서드 미정의(항상 undefined)
  - **결론**: 현재 시스템은 `@jwt_required(optional=True)` 기반 사실상 무인증으로 동작. Login UI 없음, 개인 지식베이스 용도.

- [x] **Flask JWT_TOKEN_LOCATION: 미설정 (기본값 = headers)**
  - `app/__init__.py`에 `JWT_TOKEN_LOCATION` 없음 → flask-jwt-extended 기본 = `["headers"]`
  - JWT sub = `str(user.id)` (문자열 "1", "2" ...) HS256
  - login 응답: body로만 반환(`success_response({"access_token": ...})`). HttpOnly 쿠키 미설정.
  - api.js 코멘트 "HttpOnly cookie by the backend"는 **미구현 계획임** — 실제와 다름.

- [x] **쿠키명: access_token (Rust extractor.rs에만 정의, Flask가 설정하지 않음)**
  - Rust `extractor.rs`는 이미 `Authorization: Bearer` → cookie `access_token` fallback 구현됨
  - 하지만 Flask가 해당 쿠키를 발급하지 않으므로 현재 경로 미활성

- [x] **결정C 방향: A1 (데이터 이관) — Opus 2026-06-10 확정** (§2 참조)

**Phase 0 코드 수정 결론 (🟢 Sonnet-safe, 플랜 §4-1 명시)**:
- `Claims.sub: i32` → `String` (Flask sub=str와 일치, Rust 자체 생성 시 `user.id.to_string()`)
- `extractor.rs`: `id: claims.sub.parse::<i32>()` (AuthUser.id는 i32 유지 — DB FK 타입 보존)
- 프론트 Bearer 인터셉터 추가는 Phase 2 이후로 연기 (무인증으로 Phase 1 진행 가능)

## 7. 함정 모음 (이번 세션 실측)
- CRA `resetMocks:true` → jest.mock 팩토리 구현 리셋. mock 반환은 **테스트 본문**에서 설정.
- Rust Config 필드 추가 시 **모든 Config 리터럴 ~9곳**(테스트 포함)에 추가. `grep -rn 'anthropic_api_key: None' src` 전수. **PipelineConfig/ExtractionConfig도 동명 필드 보유 → 오적용 주의**(perl 일괄치환이 2곳 오염시킨 사례).
- Rust sqlx = 런타임 문자열(매크로 0) → `SQLX_OFFLINE=true`로 DB 없이 테스트.
- DocumentView "뒤로"는 `to="/"` → sessionStorage 패턴 사용(이미 적용).
- 라이브 `minky` ALTER/DROP 금지.

## 8. 무엇이 Sonnet-safe가 아닌가 (요약)
- 🔴 결정C(스키마/데이터, 41파일·라이브 위험) → 분석만, 사람 승인.
- 🔴 Flask 폐기 → 사람 승인.
- 🟡 auth(전송방식 모순) → 실측 후, 불명이면 멈춤.
- 나머지(nginx 배선, 형태 적응, DB-free 도메인, Flask-only grep, 단위/통합 테스트) → 🟢 Sonnet 단독 가능.

## 9. 현재 상태
- Slice A(Flask 태깅 solar)=라이브 경로 / Slice B(Rust 확장 solar)=미연결(Phase 1에서 활성).
- 로컬 main 다수 커밋 ahead(미push). 이관도 로컬 main에 누적, push 전략 별도(ADR 참조).

## 10. PROGRESS 로그 컨벤션 [Sonnet 필수]

슬라이스 완료마다 `minky/MIGRATION_PROGRESS.md`에 **append**(agents.md Sprint Docs Gate 준수). 형식:
```
## Phase <N> / Slice: <도메인> — YYYY-MM-DD
- 변경 파일: <목록>
- 테스트: Rust <X passed> (`SQLX_OFFLINE=true cargo test --lib <mod>`) / 프론트 <Y passed> (`...testPathPattern=<X>`)
- 커밋: <해시>
- 상태: [DONE] | [CAS-deferred: 실DB/실배포 검증 대기] | [BLOCKED: <사유> → 에스컬레이트]
```
- `[DONE]`은 **로컬 Demo Gate(컴파일+테스트 green)** 통과 시. 라이브 검증이 남았으면 `[CAS-deferred]` 병기.
- Opus는 Phase 경계에서 이 로그 + git diff로 리뷰한다.

## 11. Phase별 핸드오프 계약 [Opus↔Sonnet 경계]

| 경계 | Sonnet 제출물 | Opus 확인 사항 |
|------|--------------|----------------|
| **Phase 0→1** | §6 auth 검증칸 채움 + `middleware::auth` 테스트 green | 토큰 전송방식 실측이 타당한가? JWT 시크릿/HS256/문자열 sub 통일됐나? 모순이면 반려 |
| **Phase 1→2** | DB-free 슬라이스(query expansion) 테스트 green + nginx diff | 결정A(Rust→Flask 형태) 준수? 배선이 한 줄씩 가역? Slice B 라이브화 확인 |
| **Phase 2 내부(이관자산)** | 마이그레이션 스크립트 + id매핑 단위테스트 + 검증쿼리(row-count/해시/orphan) | id 매핑 결정성·idempotent? 원본 `minky` SELECT-only? 검증 게이트 충분? **실행 전 멈춤** |
| **→Phase F(컷오버)** | 전 도메인 Rust 테스트 green + PROGRESS 완결 + Flask-only 분류 | 누락 도메인 0? 데이터 이관 검증 통과? → **Mario 최종 승인** |

**운영 모델**: Opus=설계·🔴/🟡결정·Phase경계 리뷰 / Sonnet=Phase 내부 자율실행(Demo Gate까지)·PROGRESS 기록·트리거 시 멈춤. 세션 단위 `/model` 전환이 기본, 병렬 기계작업은 Sonnet 서브에이전트.
