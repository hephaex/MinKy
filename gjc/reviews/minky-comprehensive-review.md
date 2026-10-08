# minky Comprehensive Review

**Date:** 2026-07-01
**Scope:** minky as hub-and-spoke central knowledge store — 3 in-repo sections (trust boundary/security, core, search) re-verified as corpus delta against `gjc/reviews/{red-team,exec_summary,rust,quality,frontend,architect,web-ui-ux}.md`; 3 external-seam sections (Gaeyang, Baram, Casterhub) as minky-side ticket + wire/auth contract specs.
**Method:** Every claim below is re-verified against the current tree (`find`/`search`/`read`, reproduction-level evidence for P0/P1, symbol+context for P2/P3) — not carried over from the prior corpus. Full re-verification trails: `/tmp/ultragoal-artifacts/g001-phase1a-ledger.md` (security/seam), `/tmp/ultragoal-artifacts/g002-phase1b-ledger.md` (remainder).
**Non-goals:** No code implementation (diagnosis/design/tickets only); no infra/deployment deep security review; no internal code review of Gaeyang/Baram/Casterhub (minky-boundary/contract only).

---

## 0. Top Summary — P0 Immediate Actions (re-verified current-tree findings only)

**No stale corpus P0 survives re-verification** — the prior flagship P0 (`stored-XSS frontend_source_map.rs:529`) and every other CRITICAL/HIGH finding in `red-team.md` cite the `routes/gpm/` subtree, which is **entirely absent from the current tree** (confirmed via `find`/`search`, zero matches). Likewise, 2 of the 3 CRITICAL findings in `rust.md` (SQL-injection framings) are unfounded on re-verification (dead code / bounded-integer false positives — see §2). **One corpus-inherited CRITICAL does survive re-verification as a live P0** (MK-S2-001, `rust.md` R1.2a — confirmed still exploitable via a real production override), and is included below alongside the two net-new findings this review discovered.

| Ticket | Severity | One-line |
|---|---|---|
| MK-S1-001 | **P0** (net-new) | No global auth layer exists; `POST /api/pipeline/ingest` and `/ingest/url` accept zero authentication of any kind. |
| MK-S1-002 | **P0** (net-new) | `BYPASS_AUTH=true` short-circuits `AuthUser`/`OptionalAuthUser`/`AdminUser` to a hardcoded admin identity for every caller, with no further gating. |
| MK-S2-001 | **P0** (corpus-inherited, re-verified live) | `PipelineStage::should_skip` is a true no-op — a stage's skip intent is silently ignored and the stage runs anyway; confirmed live via `pipeline/stages/analysis.rs`'s real override. |

**Immediate action:** put an explicit auth requirement on the pipeline ingest routes (MK-S1-001), treat `BYPASS_AUTH` as a deployment-time hard-fail outside single-operator/internal contexts (MK-S1-002), and implement the `should_skip` passthrough or remove the hook (MK-S2-001) before any externally-reachable deployment.

| Section | P0 count | Total tickets | Design/contract included? |
|---|---|---|---|
| 1. Trust Boundary and Security | 2 | 7 | — |
| 2. minky Core | 1 | 5 | — |
| 3. Search (3-path) | 0 | 3 | — |
| 4. Gaeyang Seam + Contract | 0 | 2 | Yes |
| 5. Baram Seam + Contract | 0 | 2 | Yes |
| 6. Casterhub Seam + Contract | 0 | 2 | Yes |

---

## 1. Trust Boundary and Security

Untrusted content enters minky from two source seams (Baram news, Casterhub fintech data) and lands in a shared pgvector/RAG index that Gaeyang consumes. This is the primary risk frame — placed first, drives overall severity — and its backbone is **net-new current-tree findings**, not the inherited (and now confirmed-absent) corpus P0.

### Red-Team Delta (corpus re-verification)

`gjc/reviews/red-team.md` has 11 findings total. **7 of 11** — every CRITICAL/HIGH/MEDIUM vulnerability finding (stored-XSS, script-stripper, plaintext-token, trust-data-leak, token-hash-entropy, `trust_level` mass-assignment, GPM-import CSRF-reliance) — cite a `routes/gpm/...` path. `find`/`search` confirm the entire `gpm/` subtree, `frontend_source_map.rs`, `personal_access_tokens.rs`, and `source_map` are **absent from the current tree**. All 7 are reclassified **{stale}**; `exec_summary.md`'s CR-4/HI-4/HI-5 are the same 3 items, likewise stale. None are ticketed or promoted.

The remaining **4 of 11** `red-team.md` findings are LOW-safe and cite non-gpm, still-present files — re-verified separately, not discarded:
- **SQLi (§2)**: `services/tag_service.rs` (previously flagged "unverified") uses only parameterized `sqlx::query`/`query_as` with `$1`/`$2` binds. {resolved, closed safe}.
- **CSRF (§4)**: `SameSite=Strict; HttpOnly` cookie control moved from `middleware/auth.rs` to `routes/auth.rs` (`build_cookie`/`build_delete_cookie`) — still present, functionally equivalent. {resolved, citation-corrected}.
- **Path traversal (§5)**: `try_canonicalize` (`routes/vault.rs`) returns the original unmodified path string on canonicalize failure — confirmed non-dangerous, answers the corpus's own open question. {resolved}.
- **SSRF (§3)**: not re-walked this pass; no evidence found contradicting the original safe finding. Carried forward as {valid, not re-verified at reproduction level}.

### Net-new trust-boundary findings (this review)

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S1-001 | `minky-rust/src/routes/pipeline.rs` `ingest_document`, `ingest_from_url`; `minky-rust/src/lib.rs:205-209` (layer stack) | No global auth layer exists — `lib.rs`'s only `.layer()` calls are rate-limit/trace/compression/CORS. `ingest_document`/`ingest_from_url` handler signatures take zero auth extractor of any kind (verified by reading the full signatures), and `pipeline::router()` (`routes/mod.rs:77`) is nested with no wrapping middleware. Anyone reaching the HTTP port can ingest arbitrary documents into the shared index. | Add an explicit `AuthUser` (or dedicated ingest-service-token) extractor to both pipeline handlers; do not rely on a future global layer alone. | **P0** | `curl -X POST /api/pipeline/ingest` without credentials returns 401, not 200. | S |
| MK-S1-002 | `minky-rust/src/middleware/extractor.rs` `is_bypass_auth`, `bypass_admin_user` | When `BYPASS_AUTH=true`, all three extractors (`AuthUser`, `OptionalAuthUser`, `AdminUser`) short-circuit to a hardcoded `AuthUser{id:1,role:"admin"}` for every caller, no exceptions (direct read of all three `FromRequestParts` impls). Default is `false` (`.unwrap_or(false)`) — corpus framing of "vulnerable by default" is imprecise; the real risk is a deployment/env misconfiguration silently granting blanket unauthenticated admin. | Treat `BYPASS_AUTH=true` as a startup hard-fail unless an explicit `ALLOW_BYPASS_AUTH_UNSAFE=true` companion flag is also set; log a loud warning either way. | **P0** | Setting `BYPASS_AUTH=true` alone (without the companion flag) fails app startup with a clear error. | S |
| MK-S1-003 | `minky-rust/src/routes/rag.rs` `/search/ask`, `/search/semantic`, `/search/ask/stream`, history | All four RAG/search consume endpoints take `_auth_user: OptionalAuthUser` (underscore-prefixed — extracted but never checked). `OptionalAuthUser` never rejects; unauthenticated callers get a real answer. Compounds the untrusted-source→shared-index risk: no consumer-side gate either. | Require `AuthUser` (non-optional) on these four handlers, or add an explicit authorization check using the extracted `OptionalAuthUser` value. | **P1** | Unauthenticated `POST /search/ask` returns 401. | S |
| MK-S1-004 | `minky-rust/src/pipeline/content.rs` `NamedPipeStrategy::fallback_write` | `let _ = tx.send(CasterSyncMessage{..})` — a failed fallback send (closed/full channel) is silently discarded. If the primary named-pipe write also failed, this is the last resort; dropping it silently means data loss with no surfaced error. | Log the send error at `warn` level and increment a metrics counter; consider a bounded retry. | P2 | A forced-closed fallback channel produces a `warn!` log line, not silence. | S |
| MK-S1-005 | `minky-rust/migrations/*.sql` (all 21 present files, 001-016+018-022) | No `provenance`, `tenant_id`, or `source_id` column exists anywhere in the schema. `audit_log`'s `resource_id VARCHAR(100)` is generic, not per-row source-system/tenant attribution. Directly relevant to the untrusted-source trust boundary: nothing in the data model distinguishes a Baram-sourced row from a Casterhub-sourced row from a manual upload. | Add a `source_system` + `tenant_id` column (nullable, backfill-friendly) to `documents` in a new forward-only migration; thread it through the ingestion pipeline. | P2 | A new migration adds the columns with a CHECK/enum constraint; ingestion code populates them. | M |
| MK-S1-006 | `minky-rust/migrations/015_memory_category_seed.sql:7,21` | Hardcodes `user_id=1` (`WHERE u.id = 1`, `CROSS JOIN (SELECT id FROM users WHERE id = 1)`). Consistent with MK-S1-005 — no tenant column exists to do otherwise. Single-tenant seed-data assumption. | Document as an intentional single-operator assumption until MK-S1-005 lands, or parameterize the seed to run per-tenant. | P3 | Migration comment explicitly states the single-tenant assumption and links to MK-S1-005. | S |
| MK-S1-007 | `minky-rust/src/routes/security.rs` `list_api_keys`/`create_api_key`/`revoke_api_key`; `services/security_service.rs` | Corpus-correction, not a new problem: the `api_keys` table DOES have per-route `AuthUser` (strict, non-optional) enforcement plus query-level `WHERE user_id = $1` scoping. The "no validating middleware" framing does not reproduce — there is no *global* auth layer (see MK-S1-001), but this specific table is properly gated. | No action; documented so this is not double-ticketed against MK-S1-001. | — (resolved/documentation) | N/A | — |

Corpus red-team.md summary table (superseded — retained only to show what changed):

| # | Concern (original) | Original Sev | Current status |
|---|---|---|---|
| 1 | HTML injection via PlotTextarea | CRITICAL | **stale** — `gpm/` subtree absent |
| 6 | Credentials plaintext in DB | CRITICAL | **stale** — file absent |
| 10 | Trust-level privilege escalation | HIGH | **stale** — file absent |
| 8 | GPM trust data leakage | HIGH | **stale** — file absent |
| 2 | Script stripper deficiency | MEDIUM | **stale** — file absent |
| 9 | Token hash entropy loss | MEDIUM | **stale** — file absent |
| 11 | GPM import CSRF reliance | MEDIUM | **stale** — file absent |
| 3 | SQL injection | LOW-safe | **resolved** — re-verified against `tag_service.rs` |
| 4 | SSRF | LOW-safe | valid, not re-walked this pass |
| 5 | CSRF (SameSite) | LOW-safe | **resolved** — control moved to `routes/auth.rs`, still present |
| 7 (path traversal) | Path traversal | LOW-safe | **resolved** — `try_canonicalize` confirmed non-dangerous |

---

## 2. minky Core

Delta-only tickets from re-verification of `rust.md` (backend) and `quality.md` (cross-domain).

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S2-001 | `minky-rust/src/pipeline/stage.rs` `StageExecutor::execute` (~L101-105) | `should_skip` is a true no-op: when a stage's `should_skip` returns `true`, execution logs "Skipping stage" and then unconditionally proceeds to `validate()`/`process()` anyway — no passthrough. Confirmed live, not theoretical: `pipeline/stages/analysis.rs` genuinely overrides `should_skip`, so a real stage's skip intent is silently ignored today. | Implement an early-return passthrough in `execute()` for `I == O` stages, or remove `should_skip` from the trait and let stages self-gate inside `process()`. | **P0** (was CRITICAL in corpus, confirmed live) | A stage with `should_skip() == true` does not call `process()`; its output equals its input unchanged. | M |
| MK-S2-002 | `minky-rust/src/lib.rs:37-38`; `routes/vault.rs` `watch_status`, `watch_reload` | `vault_watcher` is `Arc<tokio::sync::Mutex<Option<VaultWatcherHandle>>>`. Read-only `watch_status` acquires the same exclusive lock as `watch_reload`'s write, serializing concurrent status checks against reloads. | Replace with `Arc<tokio::sync::RwLock<...>>`; use `.read().await` in `watch_status`. | **P1** (HIGH in corpus, confirmed) | Concurrent `watch_status` calls do not block on an in-flight `watch_reload`. | S |
| MK-S2-003 | `minky-rust/src/config.rs`, `lib.rs:46-72` | No `Config::validate()` at startup. `database_url` unset fails only at first query; `jwt_secret` (a `SecretString`) unvalidated at startup silently signs tokens with empty bytes if unset. | Add `Config::validate_for_production()` called from `create_app()` before pool connect. | **P1** (HIGH in corpus, confirmed) | Starting the app with `jwt_secret` unset fails fast with a clear error, not a silent bad-token issue at first request. | S |
| MK-S2-004 | `minky-rust/tests/` (4 files total) | No integration test exercises `/api/vault/ingest`, `/api/vault/sync/report`, or the SRT ingestion pipeline end-to-end. Confirmed: `search` for `vault|ingest|srt|sync_report` across `tests/` returns zero matches. | Add `tests/vault_ingest_test.rs` covering the ingest→pipeline→embed happy path with an in-memory/test DB. | **P1** (HIGH in corpus, confirmed) | New test file exercises at least one full ingest round-trip. | M |
| MK-S2-005 | `minky-rust/src/services/vault_common.rs` `collect_md_files`; called from `routes/vault.rs` `sync_report`/`ingest_vault` | Synchronous `std::fs::read_dir`/`symlink_metadata` runs inside `async fn` handlers, blocking the tokio worker pool during large-vault directory walks. | Move to `tokio::fs::read_dir` + buffered streaming, or `spawn_blocking`. | P2 (MEDIUM in corpus, confirmed) | A 10K-file vault walk does not visibly stall unrelated concurrent requests (measured or code-reviewed for `spawn_blocking` wrapping). | M |

### Corpus corrections — not tickets (documented so they are not re-raised)

- **`build_search_pattern` "CRITICAL SQL injection"** (`document_service.rs:214-216`, corpus R8.1a): the function is unescaped but has **zero production callers** — only its own unit tests and a benchmark reference it. No SQL interpolation site exists anywhere. Not a live vulnerability; do not ticket. If the function is later wired up, escaping must be added at that time.
- **`sync_report` "CRITICAL LIMIT injection"** (`routes/vault.rs`, corpus R8.2a): `remaining` is a server-side `usize` derived from the constant `SYNC_REPORT_DB_LIMIT` (50,000), only ever decremented via `saturating_sub` (never negative), on an `AdminUser`-gated route with no client-controlled numeric input. Interpolating a bounded non-negative integer cannot inject SQL. Not a live vulnerability.
- **"Missing `AppError::OffensiveContent` variant"** (corpus R4.1a, HIGH): the premised `content.offensive`/`offensive_reasons` fields do not exist anywhere in the Rust backend (zero matches). No such variant is warranted; the corpus finding described a feature that isn't present.
- **"No CHECK constraint on `sensitivity`"** (corpus R6.2a): a CHECK constraint DOES exist — added by migration `012_document_sensitivity.sql` (the corpus cited migration 011, which does not touch `sensitivity` at all), type `VARCHAR(20)` constrained to `('confidential','internal','public')`. Resolved.
- **DocumentCard.test.js / SearchBar.test.js / SearchBar.test.jsx / FileUpload.test.js "MISSING"** (quality.md, frontend.md): all four files **exist on disk with substantive, real assertions** (196, 73, 215, and confirmed-present lines respectively). This was independently confirmed by the review leader and by two separate re-verification passes. The only genuine, still-open test gap in this area is the leaf-folder-click (`onNodeClick`) scenario in `TreeView.test.js` — see the frontend note below.
- **`DRAMA_EXTENSIONS` dead constant** and **`openapi.rs` unused `Router` import**: both stale — the constant no longer exists in `scripts/srt_to_narrative.py`, and `Router` is used at `router()`.

### Frontend-adjacent findings (documented for completeness; frontend implementation is out of primary scope)

- **Backend error-envelope split (still real, unchanged):** 8 route modules (`agents.rs`, `ml.rs`, `korean.rs`, `skills.rs`, `sync.rs`, `harness.rs`, `ocr.rs`, `timeline.rs`) return raw `Result<Json<...>, (StatusCode, String)>` instead of `AppResult<Json<...>>`, producing a plain-string error body instead of the `{"success": false, "error": "..."}` envelope the frontend expects elsewhere. Not re-ticketed with a full schema here (frontend/API-contract work), flagged for a follow-up ticket if backend error-contract consistency becomes in-scope.
- **`srt_to_narrative.py` `extract_title`** still has the first-occurrence `.NA.` split bug and the case-sensitive/incomplete language-suffix regex, exactly as previously reported — genuinely unfixed.
- **TreeView folder-click contradiction, resolved:** `frontend.md` described an open HIGH defect (shared inline toggle handler, navigation disabled); current code shows `handleClick`/`handleArrowClick` as separate functions with the `onNodeClick` fallback correctly wired for leaf folders — `quality.md`'s "fix already landed" framing is accurate; `frontend.md`'s framing is stale. The *remaining* behavior (folders-with-children always toggle, never navigate) is confirmed to be an intentional design tradeoff, accurately described by `web-ui-ux.md` as a UX recommendation, not a code defect.
- **`architect.md` is the most decayed corpus file** re-verified this pass: 3 of 5 spot-checked claims are stale (SearchBar business logic already extracted to a pure-presentational component; `api.getDocumentTree`/`GET /documents/tree` path actually aligns; the entire workspace-settings feature it describes — frontend and backend — no longer exists in the tree). One claim needs correction rather than deletion: the root `SearchBar.js`/`.test.js`/`.css` files are **not** unused dead code as claimed — `pages/DocumentList.js` and `pages/TagList.js` both import the root stub live; any consolidation must account for those two consumers.

---

## 3. Search (3-path)

minky's search surface is three parallel implementations, not one: OpenSearch lexical (`services/search_service.rs`), pgvector semantic (`rag.rs`/`embedder.rs`), and a `hybrid.rs` combiner.

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S3-001 | `minky-rust/src/routes/search.rs` (semantic handler, dead-code comment ~L93-97) | The semantic-search handler path is marked as currently dead code in its own comment ("when wired up it should extract OptionalAuthUser..."); OpenSearch-semantic is a parallel, unwired path alongside the pgvector semantic path in `rag.rs`. Two semantic implementations exist with no documented authoritative-path decision. | Decide and document which semantic path (OpenSearch-semantic vs pgvector/rag.rs) is authoritative; either wire up or delete the dead OpenSearch-semantic handler; add a dedup/sync note if both are intentionally retained. | P2 | A short ADR comment or doc states which semantic path is authoritative; the non-authoritative path is either removed or explicitly marked experimental. | M |
| MK-S3-002 | `minky-rust/src/services/document_service.rs` `list()` (~L28) | Pagination `offset = (page - 1) * limit` is computed without clamping; an existing `clamp_page_params`/`calc_offset` helper exists but `list()` does not call it. | Route `list()` through the existing clamp helper. | P2 | `page=999999999` no longer produces an unclamped multi-billion offset bind. | S |
| MK-S3-003 | `minky-rust/src/routes/rag.rs` (see MK-S1-003) | Unauthenticated consume paths on the RAG/semantic-search surface (cross-referenced from §1; not duplicated as a separate ticket — fix under MK-S1-003). | — | — (see MK-S1-003) | — | — |

---

## 4. Gaeyang Seam + Contract

Gaeyang is the consumer: an LLM agent/assistant using minky as its long-term memory (notes/document) retrieval backend.

**minky-seam tickets (real symbols):**

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S4-001 | `minky-rust/src/routes/rag.rs` `/search/ask`, `/search/ask/stream` (SSE), `/search/semantic` | Consumer-facing read endpoints are unauthenticated in practice (`OptionalAuthUser`, never gated — see MK-S1-003). Gaeyang as a trusted consumer has no way to distinguish itself from an anonymous caller today. | Same fix as MK-S1-003; additionally consider a service-to-service auth token specifically for Gaeyang once MK-S1-003 lands. | P1 (tracks MK-S1-003) | — | — |
| MK-S4-002 | `minky-rust/migrations/015_memory_category_seed.sql`; write path for `memory_category` (episode/semantic/bond/reflection/reference) | The memory-category taxonomy is seeded but the write/tag contract for how Gaeyang writes back into these categories is not documented anywhere in code comments or the migration. | Add a short contract doc (or code comment block) describing which category each write-path targets and the expected tagging convention. | P2 | A doc comment or README section enumerates the 5 categories and their intended write source. | S |

**Wire/auth contract (minky-side; OPEN where undecidable from this repo alone):**
- **Read:** `POST /search/ask` (sync), `POST /search/ask/stream` (SSE), `POST /search/semantic` — all currently reachable without authentication (see MK-S1-003/MK-S4-001; fix pending).
- **Write:** via `memory_category` seed taxonomy (migration 015) — exact write endpoint/schema for Gaeyang pushing new memories is **OPEN**: no route in the current tree is visibly dedicated to "Gaeyang writes a memory row"; this may be a generic document/category write reused by multiple consumers, but that reuse is not documented.
- **Auth token format:** **OPEN** — no service-to-service credential scheme for Gaeyang specifically exists in the current tree; general `AuthUser` JWT is the only mechanism observed.

---

## 5. Baram Seam + Contract

Baram is the source: a news-crawler pushing content into minky for storage/embedding/indexing.

**minky-seam tickets:**

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S5-001 | `minky-rust/src/routes/pipeline.rs` `ingest_document` (`POST /api/pipeline/ingest`), `ingest_from_url` (`POST /api/pipeline/ingest/url`) | No authentication of any kind on the ingest routes Baram would use to push content (tracks MK-S1-001 — this is the same defect, viewed from the source-seam side). | Same fix as MK-S1-001, framed here as a source-authentication requirement so Baram-originated content is provenance-tagged, not just "some caller". | P0 (tracks MK-S1-001) | — | — |
| MK-S5-002 | `minky-rust/src/routes/pipeline.rs` `router()`; `embedding_queue` (migration 014) | No source-identity/provenance is recorded on ingested documents (tracks MK-S1-005 — no `source_system`/`tenant_id` column exists). A document ingested via `/api/pipeline/ingest` cannot currently be distinguished as Baram-sourced vs. manually uploaded. | Same fix as MK-S1-005, applied specifically to the pipeline ingest write path once the columns exist. | P2 (tracks MK-S1-005) | — | — |

**Wire/auth contract (minky-side):**
- **Push wire:** `POST /api/pipeline/ingest` (direct content) and `POST /api/pipeline/ingest/url` (fetch-and-ingest) — Markdown-only push; minky owns embedding generation via `embedding_queue` (confirmed: minky is the sole embedding owner per migration 014's queue design, not the source).
- **Auth:** currently none enforced (MK-S5-001/MK-S1-001) — pending fix.
- **Source identity/signing:** **OPEN** — no signing scheme or source-identity header is defined anywhere in the current pipeline route code; this must be decided jointly with the Baram team once MK-S5-001 is addressed.

---

## 6. Casterhub Seam + Contract

Casterhub is a fintech/market-data source, carrying additional regulatory/PII/audit weight.

**minky-seam tickets:**

| Ticket | File · Symbol | Problem | Proposed change | Severity | Acceptance | Effort |
|---|---|---|---|---|---|---|
| MK-S6-001 | `minky-rust/src/pipeline/content.rs` `CasterSyncMessage`, `NamedPipeStrategy`, `FallbackPipeline` (see MK-S1-004) | The Casterhub-specific sync-message fallback path silently swallows send errors (tracks MK-S1-004 — same defect, Casterhub-specific because `CasterSyncMessage` is the Casterhub sync payload type). For fintech data, a silently dropped sync message is a bigger integrity concern than for general content. | Same fix as MK-S1-004; for Casterhub specifically, consider making fallback-send failures a hard error rather than a warning, given the regulatory context. | P2 (tracks MK-S1-004, elevated framing for fintech data) | — | — |
| MK-S6-002 | `minky-rust/migrations/020_content_hash_dedup.sql` | Content-hash dedup exists (migration 020) but no documented policy for what happens when Casterhub sends a corrected/amended version of previously-ingested fintech data with the same or a different hash. | Document the dedup/amendment policy explicitly (does a new hash create a new row, and if so how do consumers know the old row is superseded?). | P2 | A short doc/comment describes the amendment-handling behavior. | S |

**Wire/auth contract (minky-side, + fintech/PII regulatory note):**
- **Push wire:** shares the `pipeline.rs` + `pipeline/content.rs` (`CasterSyncMessage`/`NamedPipeStrategy`/`FallbackPipeline`) infrastructure with Baram; dedup via `content_hash` (migration 020).
- **Auth:** currently none enforced (tracks MK-S1-001/MK-S5-001).
- **PII classification authority:** **OPEN** — no column, flag, or documented policy in the current schema classifies which fields of Casterhub-sourced content are PII-sensitive vs. public market data. This must be decided with input from whoever owns Casterhub's data-classification policy; minky's schema currently has no hook for it (tracks MK-S1-005's missing provenance/tenant work — a PII-classification flag would naturally extend that same migration).
- **Retention policy:** **OPEN** — no retention/TTL mechanism exists in the current schema for any content, Casterhub-sourced or otherwise.

---

## Appendix A: Corpus Delta Ledger (audited N = 80)

Full per-item ledgers with file:line evidence are preserved as durable ultragoal artifacts:
- **Phase 1a (security/seam, 18 items audited, 7 net-new promoted):** `/tmp/ultragoal-artifacts/g001-phase1a-ledger.md`
- **Phase 1b (remainder, 62 items audited):** `/tmp/ultragoal-artifacts/g002-phase1b-ledger.md`

**Summary by source file:**

| Source file | Items audited | Valid (ticketed above) | Resolved/stale (corpus-was-wrong or already fixed) | Unverifiable-via-static-analysis |
|---|---|---|---|---|
| `red-team.md` | 11 | 0 promoted as-is (4 re-verified safe, folded into §1) | 7 (gpm-scoped, absent) | 0 |
| `exec_summary.md` (CR-4/HI-4/HI-5 subset) | 3 (subset of the 11 above, not additional) | — | 3 (same gpm items) | 0 |
| net-new (this review, §1) | 7 | 7 (MK-S1-001..007) | — | 0 |
| `rust.md` | 24 | 19 | 5 (R1.2b, R4.1a, R6.2a, R8.1a, R8.2a) | 0 |
| `quality.md` | 20 | 9 valid + 3 lower-priority/partial | 4 resolved (missing-test claims) + 2 stale (dead constant, unused import) | 2 (§8 git-history narrative, temp_commit) |
| `frontend.md` | 5 | 3 | 2 (stale/contradicted by current code) | 0 |
| `architect.md` | 5 | 1 valid + 1 valid-partial | 3 (stale — feature removed or already fixed) | 0 |
| `web-ui-ux.md` | 8 | 8 | 0 | 0 |
| **Total** | **80** (18 Phase 1a + 62 Phase 1b) | — | — | 2 |

**Headline corrections from re-verification (most consequential):**
1. The corpus's flagship P0 (stored-XSS) and every other `red-team.md` CRITICAL/HIGH finding cite code (`routes/gpm/`) that does not exist in the current tree.
2. 2 of `rust.md`'s 3 original CRITICAL findings (SQL injection via `build_search_pattern`; LIMIT injection in `sync_report`) are unfounded — dead code and a bounded server-side integer, respectively.
3. 4 "missing test file" claims across `quality.md`/`frontend.md` (DocumentCard, SearchBar ×2, FileUpload) are stale — all four files exist with substantive tests.
4. `architect.md` is the most decayed corpus file audited — 3 of 5 spot-checked claims describe code/features no longer present.
5. The three surviving P0s are: two **net-new** findings this review discovered (no global auth layer / unauthenticated pipeline ingest; a `BYPASS_AUTH` footgun — neither was in any prior corpus file) plus one **corpus-inherited CRITICAL confirmed still live** (`should_skip` no-op, MK-S2-001, `rust.md` R1.2a).
