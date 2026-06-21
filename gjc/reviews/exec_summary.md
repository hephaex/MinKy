# Minky Cross-Domain Executive Summary

**Scope:** Six independent review passes — architecture, frontend, backend/rust, quality, red-team, and web-ui-ux.

**Raw file sizes:**
- architect.md: 9,202 bytes
- frontend.md: 7,350 bytes
- quality.md: 28,516 bytes
- red-team.md: 20,411 bytes
- rust.md: 31,393 bytes
- web-ui-ux.md: 26,463 bytes

---

## Executive Verdict

The codebase is structurally sound but carries **one CRITICAL security vulnerability that requires immediate remediation**:

1. Stored XSS via unsanitized HTML in `PlotTextarea`

Alongside this, **one ARCHITECT-LEVEL layering violation** (business logic in UI component + route-level embedding orchestration) compounds operational risk. The frontend tree widget has a strategic failure in keyboard accessibility, and the test suite has three canonical components with zero coverage.

Total issue taxonomy across all domains: **~50 issues** categorized below. Full detail in six source files.

---

## Critical Findings

| # | Domain | Issue | Source File |
|---|--------|-------|-------------|
|| CR-4 | Red-Team | Stored XSS: `PlotTextarea` inserts unsanitized HTML into DOM. Only `<script>` tags are stripped; `onclick`, `onerror`, `javascript:` URLs all pass through | `frontend_source_map.rs:529` in red-team.md (§1) |
---

## High Findings

| # | Domain | Issue | Source File |
|---|--------|-------|-------------|
| HI-1 | Frontend | TreeView folder row click triggers `onToggle` (toggle) instead of `onNodeClick` (navigate) — navigation into tag/year folders effectively disabled | `TreeView.js:ar-122ks` in frontend.md + architect.md |
| HI-2 | Backend/Rust | VaultWatcher uses `Arc<tokio::sync::Mutex<...>>` — every read acquires a write lock; concurrent `watch_status` requests block each other | `lib.rs:37` in rust.md (R2.2b) |
| HI-3 | Backend/Rust | No startup config validation — `database_url` can be unset; pool creation only fails at first query runtime | `lib.rs:44-71` in rust.md (R5.2a) |
| HI-4 | Red-Team | GPM token hash derives by lowercasing and stripping `sk-live-` / `sk-test-` prefix — loses ~8 bytes entropy, case-insensitive collision | `personal_access_tokens.rs:70` in red-team.md (§7) |
| HI-5 | Red-Team | GPM import endpoint relies solely on `SameSite=Strict` cookie for CSRF defense; no explicit token — relaxing SameSite for legacy iframe embedding re-enables CSRF | `sync.rs:128` in red-team.md (§9) |
| HI-6 | Architecture | `SearchBar.jsx` ask handler contains direct business logic (search history enqueue, RAG pipeline call) inside a UI component | `SearchBar.jsx:L154-162` in architect.md |
| HI-7 | Architecture | `enqueue_for_embedding()` defined in `routes/vault.rs` but reaches directly into `state.embedding_service` — transport layer reaching into business internals | `vault.rs:635-645` in architect.md |
| HI-8 | Quality | `AppError` has no `OffensiveContent` variant despite content field carrying `offensive: bool` and `offensive_reasons` — all offensive content errors collapse to generic `Validation` | `error.rs:10-41` in quality.md |

---

## Medium Findings

| # | Domain | Issue | Source File |
|---|--------|-------|-------------|
| ME-1 | Web-UI/UX | Tree roving `tabIndex` pattern is broken — `ArrowDown`/`ArrowUp` moves `focusedId` state but not DOM focus; focus jump happens via async `useEffect` causing flicker/loss | `TreeView.js` in web-ui-ux.md (§1, A1) |
| ME-2 | Web-UI/UX | `Enter` and `Space` both call `handleClick` on folders — WAI-ARIA tree pattern expects Enter=activate, Space=toggle | `TreeView.js` in web-ui-ux.md (§1, A2) |
| ME-3 | Web-UI/UX | `aria-level` missing on treeitems; `aria-label` on chevron is Korean-only (`접기`/`펼치기`); `aria-selected` only set on documents | `TreeView.js` in web-ui-ux.md (§1, A3-A7) |
| ME-4 | Frontend | `handleClick` on folder row toggles instead of triggering `onNodeClick` — the fix (adding `else if (onNodeClick)`) only covers leaf folders; expandable folders still toggle, never navigate | `TreeView.js` in quality.md §1 |
| ME-5 | Frontend | `UserEvent` present in package but half the tests still use `fireEvent` — inconsistency; no `userEvent` migration completed | `frontend/src/components/*.test.*` in frontend.md |
| ME-6 | Frontend | No `React.lazy()` coded evidence; large components (KnowledgeGraph 478 lines, CollaborativeEditor 430 lines) not lazy-loaded | multiple components in frontend.md |
| ME-7 | Quality | `DocumentCard.test.js` and `SearchBar.test.jsx` **do not exist on disk** — zero test coverage for two primary user-facing components | quality.md §3 |
| ME-8 | Quality | AI service routes (agents, ml, korean, skills, sync, harness, ocr) return raw `(StatusCode, String)` tuples instead of `AppError::into_response` structured JSON — frontend `success: false` envelope absent on these error paths | quality.md §4 + rust.md |
| ME-9 | Quality | `PipelineContext::cancelled` flag exists and has `cancel()` method but is never checked by `DocumentPipeline::process` — cancellation silently ignored | rust.md (R1.1a) |
| ME-10 | Web-UI/UX | `extract_title` in `srt_to_narrative.py` splits on **first** `.NA.` occurrence — `"2026-04.Special Report.NA.SBS.ko.srt"` → `"2026-04"` only; case-sensitive `.ko` suffix misses `.KO`; missing codes (`.fr`, `.de`, `.es`) | `scripts/srt_to_narrative.py` in quality.md §2 |
| ME-11 | Quality | `PipelineStage` validates hook exists but no stage overrides it — unused API surface | rust.md (R1.2b) |
| ME-12 | Backend/Rust | `skip_storage` mode returns `context.job_id` (random UUID) as `PipelineOutput.document_id`; downstream consumers treat as real `documents.id` → always `NotFound` | builder.rs:157-171 in rust.md (R1.1b) |
| ME-13 | Architecture | `lib.rs:44-177` (`create_app`) wires config, OAuth, upload, chat, search, vault watcher, workspace CRUD in one function — no `AppRouter` builder; testability limited | architect.md |

---

## Low Findings

| # | Domain | Issue | Source File |
|---|--------|-------|-------------|
| LO-1 | Frontend | No SWR / React Query — dashboard and bulk search/knowledge pages rely on raw per-page fetch with no caching, retries, or deduping | `frontend/package.json:47` in frontend.md |
| LO-2 | Frontend | One CSS file per component — no shared design tokens; duplicated hover colors (`#e9ecef`) across `MLAnalytics.css`, `TreeView.css` | demo in frontend.md |
| LO-3 | Frontend | Large monolithic components not decomposed — 478-line KnowledgeGraph, 430-line CollaborativeEditor, 204-line MLAnalytics | frontend.md |
| LO-4 | Backend/Rust | `set_metadata()` silently swallows serialization failures — metadata silently lost | context.rs:84-88 in rust.md (R1.3b) |
| LO-5 | Backend/Rust | `From<sqlx::Error> for PipelineError` flattens all error kinds to string; `From<AppError> for PipelineError` maps all auth variants to `Internal` | pipeline/error.rs in rust.md (R1.4a, R1.4b) |
| LO-6 | Backend/Rust | `DocumentPipelineBuilder::build()` does not validate config preconditions (`skip_storage` ↔ `pool`, `skip_embedding` ↔ embedding provider) | builder.rs:215-344 in rust.md (R1.1c) |
| LO-7 | Backend/Rust | `list()` in document_service uses unclamped `page`/`limit` for offset; pathological `page=999999999` produces ~10B row offset | document_service.rs:28 in rust.md (R8.3b) |
| LO-8 | Architecture | Stale files at `frontend/src/components/SearchBar.js` (50-line stub), `SearchBar.test.js` (old pattern), `SearchBar.css` (shared, duplicated) — all alongside canonical `Search/SearchBar.jsx` | architect.md |
| LO-9 | Backend/Rust | `Document` derives `Serialize` (correct for DB read model) but has no `Deserialize` guard — risk of accidental deserialization surface being opened | models/document.rs:7 in quality.md |
| LO-10 | Red-Team | Script stripper (`strip_scripts`) removes `<script>` tags but does not handle `onerror`, event handlers, `javascript:` URLs, or entity-encoded payloads; applies only to Discord import path, not direct JSON import | `links_dates.rs:68` in red-team.md (§2) |
| LO-11 | Frontend | TreeView `useMemo` filter mutates `count` field — number shows filtered count during search instead of original document count; undocumented semantic mutation | TreeSidebar.js in web-ui-ux.md |
| LO-12 | Frontend | Loading state renders plain text Korean `"로딩 중..."` with no spinner; error state has no retry button | TreeSidebar.js in web-ui-ux.md |
| LO-13 | Quality | `TreeNode.handleKeyDown` ArrowRight calls `onFocusChange` but ArrowLeft does not — keyboard asymmetry for collapsed folders | quality.md §1 |
| LO-14 | Frontend | `ChatInput.jsx`: `if (!trimmed || disabled) return; onSend(trimmed);` — this early-return pattern is correct but should be documented for consistency | quality.md §1 |

---

## Good Patterns (Confirmed)

| Domain | Pattern | Evidence |
|--------|---------|----------|
| Frontend | PropTypes usage consistent across 15+ components; `TreeView` uses extracted `nodeShape` pattern for recursive type reuse | frontend.md, quality.md |
| Frontend | Top-level `ErrorBoundary` in `App.js` wraps entire Routes tree — correct pattern | quality.md |
| Frontend | `isDocument` / `hasChildren` tree branching is correct; defensive `if (callback)` guards used consistently | quality.md |
|| Backend | SQLx macros used everywhere — all queries are parameterized correctly | rust.md §8.3 |
| Backend | `AppError` enum with `#[derive(Error)]`, `#[from]` conversions, and complete `IntoResponse` — well-designed | rust.md §4, quality.md §4 |
| Backend | Config system uses `config = "0.13"` + `dotenvy` with `.env` non-fatal fallback — correct | rust.md §5 |
| Backend | Migration files are forward-only; no destructive ALTERs; 16 migrations audited | rust.md §6 |
| Backend | `SameSite=Strict; HttpOnly` on auth cookies — correct boundary by default | red-team.md (§4) |
| Red-Team | `vault_common` module correctly validates paths via prefix check; fallback canonicalize examined and deemed safe | red-team.md (§5) |
| Architecture | Rust module separation (`routes/`, `models/`, `services/`, `pipeline/`) is clean and well-separated | architect.md |

---

## Cross-Domain Callouts

### 1. TreeView is reviewed in four separate passes
- **Frontend review** (frontend.md): folder row click semantics, monolithic component size, test coverage gaps
- **Architectural review** (architect.md): `handleClick` business logic in UI component, stale file duplication
- **Quality review** (quality.md): original bug fix verification (`onNodeClick` fallback is correct), test gap on leaf-folder click, ArrowRight/ArrowLeft keyboard asymmetry
- **Web-UI/UX review** (web-ui-ux.md): ARIA tree pattern violations — Enter duplicates Space behavior, roving tabindex broken, `aria-level` missing, Korean-only screen reader labels

**Convergence point:** The TreeView component is the most-reviewed surface in the codebase with the widest disagreement — four passes found four distinct classes of issues (behavior, architecture, test, accessibility). All four domains agree the component needs refactor. Priority fixes: split click semantics (HI-1), roving tabindex fix (ME-1), ARIA completeness (ME-3), double-click guard (ME-5).

### 2. Vault watcher has architectural and security dimensions
- **Red-Team** (§8, §9): Vault token plaintext storage is a CRITICAL credential exposure (CR-5); token hash entropy is HIGH (HI-4); GPM import CSRF reliance is MEDIUM (HI-5)
- **Architecture review**: `enqueue_for_embedding()` buried in compute-layer, reaches into `state.embedding_service` — transport layer coupling into pipeline internals (architect.md)
- **Rust review**: VaultWatcher uses Mutex for reads — acquisition bottleneck; vault watch status leaks privileged state without authZ schema extension

**Coverage gap:** `tag_service.rs` (mentioned in rust.md §8.3 note) was not audited for raw SQL — potential injection vector not yet ruled out.

### 3. Frontend error handling has a backend dependency gap
The frontend (via `api.js`) expects all error responses to have `{"success": false, "error": "..."}` shape. Eight backend route modules (`agents`, `ml`, `korean`, `skills`, `sync`, `harness`, `ocr`, `timeline`) bypass `AppError::into_response` and return raw `(StatusCode, String)` tuples. When these endpoints error, frontend parsing crashes. This is reported independently in quality.md §4 and architect.md (naming inconsistency), but neither notes the cross-domain crash risk explicitly.

---

## Unverified / Pending

| Item | Status | Domain |
|------|--------|--------|
| `tag_service.rs` SQL injection audit | **NOT verified** — review flagged as pending | Rust |
| `try_canonicalize` in `vault_common` fallback | Design note only — needs manual inspection | Red-Team |
| `x-code-samples` OpenAPI `on_failure` extension | Confirmed not implemented — documented as known gap | Backend |
| Frontend `DocumentCard.test.js` / `SearchBar.test.jsx` file paths | Review could not confirm exact canonical paths; flagged as missing in quality.md | Quality |

---

## Authoritative Sources

- **architect.md** — Module boundaries, layering violations, API cascade gaps
- **frontend.md** — Component structure, test consistency, TreeView click semantics
- **quality.md** — Bug fix verification, `extract_title` functional correctness, error handling parity, test quality
- **red-team.md** — XSS, credential storage, trust-level escalation, CSRF, SSRF, path traversal, SQL injection review
- **rust.md** — Pipeline specs, staged execution semantics, SQLx security review, service decomposition, config validation
- **web-ui-ux.md** — ARIA tree semantics, keyboard interaction patterns, visual consistency, loading/error UX, TreeSidebar data-fetching patterns

---

## Recommendations

### Immediate (fix before next deployment)
1. **CR-4**: Add backend HTML sanitizer for PlotTextarea input
### Near-Term (within next sprint)
4. **HI-1**: Separate chevron click (toggle) from folder row click (navigate) in TreeView; add double-click guard
5. **HI-2**: Switch `vault_watcher` to `RwLock`
6. **ME-1**: Fix roving tabindex — sync DOM focus in keydown handler imperative style, not `useEffect`
7. **ME-3**: Complete ARIA tree: add `aria-level`, localize chevron labels, set `aria-selected` on all treeitems, add `aria-label` to tree root
8. **ME-7**: Create `DocumentCard.test.js` and `SearchBar.test.jsx`

### Backlog
9. **HI-3**: Add `Config::validate()` at startup
10. **ME-8**: Migrate all eight AI service routes to `AppResult<Json<...>>` / `AppError` structured responses
11. **LO-1**: Introduce SWR or React Query on dashboard/list pages
