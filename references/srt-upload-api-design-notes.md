# SRT 업로드 핸들러 설계 메모

> Solar-Open2 API 호출 패턴과 개선 항목

---

## 현재 구현 (2026-06-17)

- `POST /api/documents/upload/srt` — 20MB 제한
- SRT → rolling 2-line dedup → 2500자 청크 → Solar-Open2 순차 변환 → MD 조립 → DB 저장
- 모델: `solar-open2` (버전 없는 alias)
- 청크당 timeout 120s, 10청크 × ~32s = 320s 총 소요

## 알려진 개선 항목

### 우선순위 A (다음 스프린트)

1. **max-chunk 가드** — `MAX_SRT_CHUNKS = 50` 상한 추가 (현재 무제한)
2. **shared reqwest::Client** — `AppState`에 `http_client` 필드 추가, per-request 생성 제거
3. **body limit 분리** — `/upload` 10MB, `/upload/srt` 20MB 각각 적용

### 우선순위 B (여유 시 개선)

4. `parse_srt_to_text`, `chunk_srt_text` Rust unit test 추가
5. Solar API 에러 → 서버 로그만, 클라이언트에는 generic 응답
6. 청크 병렬 처리 검토 (Upstage rate limit 확인 필요)

## 배포 상태

- CT306 `192.168.29.22` — rust-backend healthy (2026-06-17 확인)
- `.env`: `UPSTAGE_API_KEY` 설정됨 (gitignored)
- `docker-compose.yml`: `UPSTAGE_API_KEY=${UPSTAGE_API_KEY:-}` 환경변수 주입

---

*작성: 2026-06-17*
