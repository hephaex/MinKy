&gt; ralplan "Remove Anthropic Claude Provider from minky-rust" 실행 완료 — 수정안 4건 반영, 마이그레이션·스윕·삭제·전체 cargo test 통과

## 단계 1: 메인 브랜치 실측 재검증 (완료)
- `ChatProvider` grep → 0 matches (artifact 2542). ChatProvider enum + `chat_provider()`는 `main-pre-ct306-backup` 브랜치 config.rs에만 존재(L192/194/204/366). 메인 config.rs = `anthropic_api_key: Option<SecretString>` L85 / None L203 / 테스트 L289.
- rag_service.rs: `generate_answer_anthropic` 없음. 함수 = `ask` L121, `semantic_search` L182, `get_history` L230, `generate_answer` L303(LocalLlm*, `LLM_BASE_URL` env L308–309), `persist_history` L387. anthropic 언급 = doc L21 + import L38.
- anthropic_types.rs(430 lines) 하이브리드: 12개 Anthropic* 타입 + 7개 LIVE LocalLlm* 타입(L187~L253) + tests mod L259. LocalLlm*는 rag_service.rs L38, routes/rag.rs L33에서 import.

## 단계 2: 수정안 3건 pending-approval.md 반영 (완료)
- Amendment #1: ChatProvider 관련 계획 항목(Group A item 1 일부, AC#8, Intent Diff 행) stale 정정 → config.rs 작업은 필드+테스트 제거만.
- Amendment #2: rag_service.rs Group B item 6 축소 → import L38 제거 + doc L21 갱신만.
- Amendment #3 (CRITICAL): anthropic_types.rs 전체 삭제 UNSAFE → LocalLlm*를 `services/local_llm_types.rs`로 추출 후 두 임포터 리타겟 → 이후 삭제.

## 단계 3: 수정안 4건 반영 (완료)

- Amendment #4: `grep -ri 'claude'`가 계획 목록에 없는 5파일(hybrid_search.rs 4곳, pipeline/error.rs L29, models/function.rs L49, models/rag.rs L5·L64·L263·L357, skill_service.rs L191·L581)에서 일치 — doc comment/주석/테스트 문자열 갱신 범위로 Sequencing Step 5에 추가. analysis.rs 기본 모델은 이미 solar-open2-260528로 확인, stale doc comment는 Group E 범위 내.
- 단계 2 정정: Amendment #4 포함 총 4건 반영.
- 수정된 24파일 계획 + Amendment #4 반영. 실행 결과는 단계 4.

## 단계 4: 실행 및 검증 (완료)

- LocalLlm* 추출: `src/services/local_llm_types.rs` 신설(163 lines, LocalLlmRequest/Message/Response/Choice/Usage/StreamChunk + `first_text` + tests). services/mod.rs 스왑, rag_service.rs L38 / routes/rag.rs L33 리타겟.
- 마이그레이션: understanding_service.rs(348 lines), conversation_extraction_service.rs(415 lines, claude/anthropic 0), pipeline/stages/analysis.rs(455 lines), query_expansion.rs + documents.rs(`SOLAR_MODEL = "solar-open2-260528"` 통일). Anthropic `system: Some(prompt)` → `LocalLlmMessage { role: "system", ... }` 선행 + `max_tokens` 필수값(analysis 8192 / understanding 1024).
- anthropic_types.rs 삭제: 임포터 0건 grep 확인 후 rm, 빌드 GREEN(0.84s).
- 스윕 Group 1+2: 문서/주석 Claude → Solar, 502 "Anthropic API error" → "LLM API error", fallback/test 모델 문자열 → solar-open2-260528 또는 local-llm(routes/rag.rs 로컬 Qwen3 gguf/Strix Halo 경로), mcp_server.rs "Claude Desktop"·reasoning.rs:86 음성 테스트·rag_service.rs L19-22 이력 주석 KEEP.
- 전체 cargo test: test profile 컴파일 GREEN, 경고 1건(src/models/ai.rs:148 duplicate #[test])만 존재. lib 1980 tests + 통합 스위트 모두 통과 — 합계 2027 passed / 0 failed(2 ignored).
- pipeline_auth_qa VersionMismatch 해결: QA DB 환경변수 미설정 시 DATABASE_URL이 `minky_db`(마이그레이션 1-3, 001 체크섬 불일치)를 가리켜 발생. 실제 앱 DB `minky_rust_db`(25 files 체크섬 일치)로 DATABASE_URL 지정 재실행 → 5 passed. 제품 코드 결함 아닌 테스트 인프라 문제.
