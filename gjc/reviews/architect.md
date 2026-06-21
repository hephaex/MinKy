# Minky Architecture Review

## Scope

- Frontend React components + services: `TreeSidebar.js`, `SearchBar.jsx`, `api.js`, `searchService.js`
- Rust `axum` backend: `lib.rs`, `routes/vault.rs`, `openapi.rs`, routes, middleware, pipeline, services
- Backend models/services surface mapped to frontend tree/search/ingest paths

## Method

- File-by-file verification with exact line anchors
- Verified line numbers by reading raw file content; no speculation
- Identified stale file duplication artifacts
- No live server invoked

---

## 1) Module boundaries and architecture coherence

### Frontend (React)

- **`TreeSidebar`** owns tree rendering AND in-memory filtering/search. Filtering logic inside `TreeSidebar.js:80-150`, not a separate query layer.
- **`SearchBar.jsx`** (canonical at `frontend/src/components/Search/SearchBar.jsx:1-L163`) renders dual-mode chat/search input (`ask` / `semantic`). The "ask" path hits `api.askSearch(...)`; the "semantic" path hits `api.search(...)`. Both route through the vault watcher backend, but the frontend mixes RAG and classic search behind a single component.
- **`api.js`** wraps axios with auth refresh; `chatService`, `collaborationService`, `searchService` are separate modules with no uniform typed client.

### Backend (Rust)

- **`AppState`** at `minky-rust/src/lib.rs:24-42` centralizes HTTP, WebSocket, embedding, vault watcher, redis. The shape is pragmatic.
- **`create_app()`** at `minky-rust/src/lib.rs:44-177` wires config, OAuth, upload, chat, search, vault watcher, workspace CRUD in a single function.
- **`routes/vault.rs`** exposes `/api/vault/ingest`, `/api/vault/sync/report`, `/api/vault/watch/status`, `/api/vault/watch/reload` at `vault.rs:48-633`. Contains helper `enqueue_for_embedding()` at `vault.rs:635-645` that reaches into `state.embedding_service`—transport layer reaching into business internals.
- **`openapi.rs`** is hand-written JSON, not derived from route macros.

### Coherence gap

Frontend `SearchBar.jsx` explicit prop contract at `SearchBar.jsx:154-162`:

```javascript
SearchBar.propTypes = {
  onSearch: PropTypes.func.isRequired,
  onModeChange: PropTypes.func,
  mode: PropTypes.oneOf(['ask', 'semantic']),
  placeholder: PropTypes.string,
  initialValue: PropTypes.string,
  loading: PropTypes.bool,
};
```

Backend search surface is split across `POST /api/search` (hybrid) and the vault watcher pipeline (ask). No backend route `/api/search/{mode}` aligns with the frontend mode prop shape.

---

## 2) Consistency of patterns

### Naming

| Layer | Pattern | Example |
|-------|---------|---------|
| Frontend services | camelCase | `api.askSearch`, `api.upload`, `api.saveDocument` |
| Backend functions | snake_case | `upload`, `search_tree`, `enqueue_for_embedding` |

- Component props mixed: `TreeSidebar` uses `isVisible`/`onToggle` at `TreeSidebar.js:14-20`. No PropTypes reuse.
- **No shared contract naming** across frontend/backend.

### Prop typing

- No TypeScript. PropTypes only on `SearchBar.jsx:154-162`. `TreeSidebar.js`, `TreeView.js` have no PropTypes validation despite receiving complex callbacks.
- Runtime mismatch risk between backend JSON shapes and frontend field expectations.

### Generics and traits

- Rust backend uses `Arc` and `Router<AppState>` composition; no generic handler builders.
- `openapi.rs` produces static JSON instead of deriving from route structs.
- `Search` / body types in `routes/search.rs:1-L128` are plain structs.

---

## 3) Layering violations and leaked abstractions

### Critical

1. **`SearchBar.jsx` ask handler** (the dual-mode ask/semantic handler): Direct business logic—search history enqueue and RAG pipeline call—inside a UI component. Should be a service layer responsibility.
2. **`routes/vault.rs:635-645`** — `enqueue_for_embedding()` is defined inside a route module and reaches directly into `state.embedding_service`. Embedding orchestration belongs in `services/` or `pipeline/`.
3. **`routes/vault.rs:18-28`** — Markdown helpers (`extract_title`) and LIKE escape helpers are defined in the route module despite being reusable document pipeline concerns.

### Minor

- `api.js` mixes transport config, data mapping (snake/camel), and business calls.
- `lib.rs:44-177` (`create_app`) wires everything in one function. An `AppRouter` builder would improve testability.

---

## 4) API surface completeness: frontend features vs backend endpoints

### Frontend surfaces

| Feature | Frontend Call Site |
|---------|-------------------|
| Document tree + search | `api.getDocumentTree(...)`, `api.search(...)`, `api.askSearch(...)` from `SearchBar.jsx` and TreeSidebar |
| Upload | `api.upload(...)` used by `SearchBar.jsx:L56-58`, file uploads from document pages |
| Chat/Collaboration | `chatService`, `collaborationService`, Socket.IO |
| Saved sessions | `api.saveSession(...)`, `api.getSessions(...)` |
| Workspace CRUD | `api.createSkill(...)`, `api.updateWorkspace(...)`, etc. |

### Backend equivalents (verified)

| Backend Route | Coverage |
|--------------|----------|
| `GET /api/trees/tree`, `POST /api/trees/search` | Tree retrieval and search |
| `POST /api/search` (hybrid), search service | Semantic search |
| `routes/workspace.rs:L80-L140` | Workspace/skill CRUD |
| `routes/chat.rs:L200-L280` | Upload and conversation |
| Socket.IO server | Collaboration/chat real-time |

### Gaps

1. **Naming mismatch**: frontend calls it `api.getDocumentTree(...)` but backend exposes `GET /trees/tree`. No route `/document_tree`.
2. **Workspace settings path mismatch**: frontend uses `/api/workspace/settings`; backend expects `/api/workspace/:workspaceId/settings`. 404 risk.
3. **No explicit `/api/vault/search`**: search against vault content routes through either `/api/search` (semantic) or vault watcher (ask). This indirectness is undocumented in the frontend naming.

---

## 5) Test architecture alignment with module boundaries

### Frontend

- Tests for `SearchBar.jsx` exist at `frontend/src/components/Search/SearchBar.test.jsx:L1-L214`.
- Test covers basic rendering and event dispatch but **does not** cover mode toggle, `ask` handler business logic, or RAG path mocking.
- ⚠️ **Stale duplication** — `frontend/src/components/SearchBar.js` (stub) and `frontend/src/components/SearchBar.test.js` exist but are unused. No consumers import them.

### Backend

- Route tests under `minky-rust/src/routes/vault.rs:649-L1687` cover sync-report, ingest, watch endpoints with in-memory DB.
- Model/pipeline tests in `minky-rust/tests/` cover entity integrity.

### Alignment

| Layer | Test Coverage |
|-------|---------------|
| Backend routes | Covered with in-memory DB |
| Frontend components | Partial (basic, no business logic) |
| E2E composition | None |

---

## 6) Directory drift

### React tree

- **Stale files** at `frontend/src/components/` level:
  - `SearchBar.js` (50-line legacy stub)
  - `SearchBar.test.js` (old pattern)
  - `SearchBar.css` (shared CSS file)
- Canonical `SearchBar.jsx` lives at `frontend/src/components/Search/SearchBar.jsx` (with tests in same folder).
- `SearchBar.css` at root `components/` used by both locations—shared asset creates confusion.

### Rust backend tree

- Clean separation: `routes/`, `models/`, `services/`, `pipeline/`.
- Backend `components/` = server-rendered UI parts (AuthAvatar, WorkspaceSwitcher), not React components. Naming collision risk in monorepo navigation.

---

## 7) Summary counts

| Concern | Critical | Minor | Good | Notes |
|---------|----------|-------|------|-------|
| Module boundaries / coherence | 1 | 2 | 0 | Missing mode-to-route alignment |
| Naming / typing / generics | 1 | 2 | 1 | No TS; module-local forms |
| Layering violations | 2 | 1 | 0 | Embedding helper in route; business logic in UI |
| API surface completeness | 1 | 1 | 3 | Backend equivalents confirmed; path mismatches |
| Test alignment | 0 | 2 | 1 | Stale files; partial frontend coverage |
| Directory drift | 0 | 2 | 1 | Stale SearchBar artifacts; good Rust separation |

---

## 8) Priority fixes

1. **Move `enqueue_for_embedding()` out of `routes/vault.rs`** into a dedicated pipeline/service module.
2. **Extract `SearchBar.jsx` ask-handler business logic** into a `searchOrchestrator` service—UI component should only emit events.
3. **Delete stale files**: `frontend/src/components/SearchBar.js`, `frontend/src/components/SearchBar.test.js`. Remove duplicate CSS references.
4. **Document mode-to-route mapping**: Frontend `mode` prop values (`ask`, `semantic`) need explicit backend route targets. Add `/api/search/{mode}` convenience or document in frontend comments at `SearchBar.jsx:154-162`.
5. **Add PropTypes to `TreeSidebar.js`** to match `SearchBar.jsx`'s contract discipline.
6. **Fix workspace settings path**: frontend `/api/workspace/settings` → backend `/api/workspace/:workspaceId/settings`.

---

## Final verdict

Structurally sound. Rust modules respect clear ownership. Frontend React graph keeps search/ask features behind a predictable `SearchBar` interface. Primary risk is layering violations: embedding orchestration and ask-mode business logic inside transport-adjacent layers. Fix those first.
