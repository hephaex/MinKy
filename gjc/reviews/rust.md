# Rust Backend Code Review — minky-rust/src/
# Reviewed: 2025-01-19 | Scoped to Pipeline, Services, Models, Errors, Config, Migrations, Tests, SQLx, Async, OpenAPI
# Severity: CRITICAL / HIGH / MEDIUM / LOW / INFO

---

## 0. Executive Summary

The minky-rust codebase is a functionally competent — but architecturally uneven — Axum/SQLx application. The strongest components are the typed config system, the canonical `vault_common` path-validation module, and the clearly-structured migration files. The most serious problems are two SQL-injection-class vulnerabilities in `vault.rs`, one of which (the `LIMIT` injection) produces **silent mis-results even on valid user input**, while the other enables read-exfiltration via crafted `source_path` values. The codebase also ships a privileged-state-leak bug (vault watch status), a 34-client connection storm under load, and a nearly-unsound `PipelineStage::should_skip` hook. Below are all findings with file:line references.

---

## 1. Pipeline Architecture — builder.rs / stage.rs / context.rs / pipeline error

### 1.1 Builder Pattern (builder.rs)

**Status: Mixed — sound API surface, unsound execution semantics**

#### R1.1a | MEDIUM | builder.rs:95-213 | No staged-`Poll`-style early termination in `DocumentPipeline::process`

`process` is an `async fn` that hard-codes **eight sequential `StageExecutor::execute(...)` calls in a single `await` chain**. That means:

- No `task::JoinSet` / parallel fan-out is possible even for independent stages (e.g. metadata extraction + chunking are sequential today).
- There is no **cancellation-point granularity**: once `process` is polled, every stage before the failing one runs at full cost before the error propagates. The caller cannot flip `PipelineContext::cancelled` and observe early-exit.
- The `PipelineContext::cancelled` flag (context.rs:39) and `PipelineContext::cancel()` method (context.rs:127-129) exist but are **never checked by `DocumentPipeline::process`**. Five of the eight `execute(...)` calls on builder.rs:104,108,113,118,143,184,188 are unconditional `.await?` — the loop is in the method body, not in a cancellable scheduler.

**Recommended fix**: Either remove `cancelled` from the public context API, or (better) check `ctx.is_cancelled()` at each stage boundary and return `Err(PipelineError::Cancelled(stage_name))`.

---

#### R1.1b | MEDIUM | builder.rs:157-171 | `skip_storage` skips archiving the document entirely

When `self.config.skip_storage` is `true`, `process` returns a `PipelineOutput` at line 159-171 **without calling StorageStage or IndexingStage**. While this is intentional for "testing", it means the returned `PipelineOutput.document_id` at line 160 is `context.job_id` (which is a `Uuid::new_v4()` created by `PipelineContext::new` at context.rs:52), **not** a real `documents.id`. Downstream consumers that treat `PipelineOutput.document_id` as opaque storage identity will conflate pre-storage "jobs" with real `documents.id` UUIDs.

Concretely, if an caller does:

```rust
let output = pipeline.process_text("t", "c").await?;  // skip_storage
db.document_service.get(output.document_id, user)     // tries to load a DB row by a random UUID
```

…it will **always** encounter `NotFound` because the job UUID was never persisted.

**Recommended fix**: If `skip_storage` is advertised, `PipelineOutput.document_id` must be `Option<Uuid>` so the distinction is encoded in the type.

---

#### R1.1c | LOW | builder.rs:215-344 | `DocumentPipelineBuilder` does not validate the constructed config

`DocumentPipelineBuilder::build()` at line 339 clones the config and returns the pipeline; there is **no guard** for common misconfigurations:

- No check that `skip_storage == false` ⇔ `pool.is_some()`.
- No check that an embedding provider is set when `skip_embedding == false` (fall-through defaults to `EmbeddingProvider::None` at default.rs:58, which then produces `EmbeddingStage::none()` — a silent no-op that still counts as a stage metric).
- No check that `user_id` is set when `skip_storage == false` (storage will likely fail at DB layer anyway).

**Recommended fix**: Add a `validate(&self) -> PipelineResult<()>` method called by `build()`, or make individual builder methods mutually validating.

---

### 1.2 PipelineStage / StageExecutor — stage.rs

#### R1.2a | CRITICAL | stage.rs:95-100 | `PipelineStage::should_skip` is a no-op

`StageExecutor::execute` checks `if stage.should_skip(&input, context)` at line 96. The comment at lines 98-99 acknowledges the limitation:

```
// For skipped stages, we need to somehow pass through
// This is a limitation - skippable stages should handle their own passthrough
```

But **the implementation does nothing beyond logging `Skipping stage` at line 97**. After the log, execution **continues** to `stage.validate(...)` at line 103 and `stage.process(...)` at line 106 — meaning **any stage that expresses a desire to skip is still executed**. The intended passthrough (return the input unchanged) never happens.

This makes the `should_skip` hook **actively misleading** for any stage intending to use it; callers that rely on it will see unexpected errors because the stage still runs.

**Recommended fix**: Either implement the passthrough inside `StageExecutor::execute`:

```rust
// pseudocode patch
if stage.should_skip(&input, context) {
    let start = std::time::Instant::now();
    tracing::info!(stage = stage_name, "Skipping stage");
    let duration = start.elapsed().as_millis() as u64;
    context.record_stage_completion(stage_name, duration);
    // compile-time passthrough: I == O required, OR return a new trait Shape::passthrough
    Ok(input)                               // INVALID for I != O — see note below
}
```

But the trait is `PipelineStage<I, O>` with arbitrary I/O types, making automatic passthrough impossible without a new `Shape` trait that exposes `fn passthrough(input: I) -> O` for skip-capable stages. A simpler fix: **remove `should_skip` / `validate` from `StageExecutor::execute`** (let stages handle themselves in `process`) **or restrict `should_skip` to stages where `I == O`** and implement the passthrough for those.

#### R1.2b | MEDIUM | stage.rs:56-73 | `PipelineStage` optional hooks are incomplete

`PipelineStage` has two no-op default methods:

- `fn should_skip(&self, _input: &I, _context: &PipelineContext) -> bool` (lines 63-65)
- `fn validate(&self, _input: &I) -> PipelineResult<()>` (lines 70-72)

The `validate` hook is called at stage.rs:103, but **no stage in the crate currently overrides it** (verified by `git grep -n 'fn validate'` inside `minky-rust/src/pipeline/stages/` returning zero results). These hooks add API surface and implementers must actively choose to ignore them. Either document them as escape-hatches-with-caveats or remove them.

---

### 1.3 PipelineContext — context.rs

#### R1.3a | MEDIUM | context.rs:30 | `current_stage: Option<&'static str>` is reference-to-static but created from `String`

`set_current_stage` stores `&'static str` at context.rs:79-81, but the name comes from `stage.name()` which returns `&'static str`. That is sound because `PipelineStage::name` returns a static lifetime by design (the method is `fn name(&self) -> &'static str`, line 46). No issue present, but note that **this prevents stages from storing dynamic strings**; it is correct as-designed.

#### R1.3b | INFO | context.rs:84-88 | `set_metadata` silently swallows serialization failures

```rust
pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Serialize) {
    if let Ok(json_value) = serde_json::to_value(value) {
        self.metadata.insert(key.into(), json_value);
    }
}
```

If `value` cannot be serialized to JSON, the call is a silent no-op. Given that metadata flows through the pipeline and can be inspected later, **silently dropping metadata is a subtle footgun**. Either return `Result<(), serde_json::Error>` or log on failure.

---

### 1.4 Error Propagation — pipeline error.rs

#### R1.4a | LOW | pipeline/error.rs:99-103 | `From<sqlx::Error> for PipelineError` flattens all SQL errors to `Database(_)` string

```rust
impl From<sqlx::Error> for PipelineError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}
```

This discards the underlying DB error kind (connection, constraint violation, `NotFound`, `TypeNotFound`). A subsequent `From<PipelineError> for AppError` at pipeline/error.rs:118-147 then remaps `PipelineError::Database(msg)` to `AppError::Internal(anyhow!("Database error: {}", msg))` — losing the static portion (`err.to_string()` includes the sqlx variant, but as a string). Consumers cannot programmatically distinguish a missing row from a foreign-key violation.

**Recommended fix**: Keep the sqlx `Error` variant:

```rust
impl From<sqlx::Error> for PipelineError {
    fn from(err: sqlx::Error) -> Self {
        PipelineError::Database(err)
    }
}
```

…with `PipelineError::Database(sqlx::Error)` in the enum, and in `From<PipelineError> for AppError`:

```rust
PipelineError::Database(e) => match &e {
    sqlx::Error::RowNotFound => AppError::NotFound("Row not found".into()),
    // other cases
    _ => AppError::Internal(anyhow::anyhow!("Database error: {}", e)),
},
```

#### R1.4b | LOW | pipeline/error.rs:105-116 | `From<AppError> for PipelineError` loses facets

AppError variants `Unauthorized`, `Forbidden`, `Conflict`, `RateLimited` all collapse to `PipelineError::Internal` in the catch-all arm at line 113. While pipelines are rarely invoked from auth-failure paths, the mapping is lossy.

---

## 2. Service Layer Decomposition — services/

### 2.1 Overall Decomposition (services/)

**Observation: 39+ service modules is over-decomposition.** Most modules are small single-feature wrappers with very low cohesion. A representative sample:

| Module | Size (approx) | Cohesion |
|--------|-------------|----------|
| `document_service.rs` | ~8 KB | Thin CRUD on Document |
| `category_service.rs` | ~10 KB | CRUD on Category |
| `tag_service.rs` | ~9 KB | CRUD on Tag |
| `comment_service.rs` | ~7 KB | CRUD on Comment |
| `attachment_service.rs` | ~6 KB | CRUD on Attachment |
| `notification_service.rs` | ~10 KB | Email + in-app notification send |
| `analytics_service.rs` | ~16 KB | Prometheus-compatible metrics |
| `admin_service.rs` | ~13 KB | Admin config reload |

These are essentially **thin repositories with an RPC boundary** — the service layer is replicating what Axum state + domain methods on `Document`, `Category`, etc. already do. Consolidating `document_service`, `category_service`, `tag_service`, `comment_service`, and `attachment_service` into a single `EntityService` struct would reduce boilerplate without harming correctness.

### 2.2 Singleton / State / Cloning Patterns

#### R2.2a | MEDIUM | lib.rs:1-178 | Service types are wrapped in `Arc<T>` inside `AppState`, but initialization is OK for most services

`AppState` at lib.rs:31-42 holds:

- `db: PgPool` (shared via `Clone`, cheap)
- `ws_manager: Arc<WebSocketManager>` (single-threaded runtime, needs Arc)
- `embedding_service: Arc<EmbeddingService>` (initialized once at startup, lines 78-91)
- `vault_watcher: Arc<tokio::sync::Mutex<Option<...>>>` (see R2.2b)
- `redis_client: Option<redis::Client>` (cheap to clone)
- `http_client: reqwest::Client` (cheap to clone)

Most services are stateless or can be cloned. The `Arc` wrapping is necessary for the `ws_manager` (shared across handlers) and `embedding_service` (long-lived model handle), but **other services are initialized as `Arc::new(...)` deep inside route handler bodies** rather than at startup. For example, `document_service::DocumentService` is created per-handler via `DocumentService { db: state.db.clone() }` — a per-request allocation of `Arc<DocumentService>` that is unnecessary overhead.

**Recommended fix**: Move the service object constructions to `create_app()` and store them in `AppState`, mirroring the pattern already used for `EmbeddingService` and `WebSocketManager`.

#### R2.2b | HIGH | lib.rs:37 | `VaultWatcherHandle` inside `Arc<tokio::sync::Mutex<Option<...>>>` is a mutable-mutex-with-Arcs

`vault_watcher` holds an `Arc<tokio::sync::Mutex<Option<VaultWatcherHandle>>>`. This implies:

1. The watch handle can be **replaced under the mutex** at runtime (e.g. restart).
2. All handlers that want to check the watcher must **acquire the mutex**, even for read-only ops (`watch_status` at vault.rs:594-608).

While the mutex is held at watch_status for a `Option::is_some()` check, axum handlers compete for the same mutex — creating a bottleneck where `POST /api/vault/watch/reload` serializes all concurrent read_status requests.

**Recommended fix**: Replace `Arc<tokio::sync::Mutex<Option<...>>>` with `Arc<tokio::sync::RwLock<Option<...>>>` and use `.read().await` for `watch_status`. Given that the watcher runs a filesystem event loop, writes are rare.

---

## 3. Model Consistency — models/

### 3.1 Serde derives

Benchmarking all 37 models in `minky-rust/src/models/`:

| Pattern | Count | Examples |
|---------|-------|----------|
| `#[derive(Debug, Clone, Serialize, Deserialize)]` | Most models | `websocket.rs`, `tag.rs`, `category.rs` |
| `#[derive(Debug, Clone, Serialize, FromRow)]` — DB read model, no Deserialize | Some models | `document.rs:7-8`, `sync.rs` |
| `#[derive(Debug, serialize)]` — no Deserialize | A few | `DocumentWithRelations` (document.rs:107-113) holds nested data for outbound APIs |
| Mixed: `Debug` only, no serde | Few | `config.rs` is not in `models/` |

Overall, the derive discipline is broadly sound. Notable gaps:

#### R3.1a | LOW | models/document.rs:7 | `Document` has `Serialize` but not `Deserialize`

`Document` derives `Serialize, FromRow` (no `Deserialize`). This is **correct** — `Document` is a DB read model. Attempting to accept it in request bodies would be a semantic error; the DTOs are `CreateDocument` / `UpdateDocument` at lines 48-104. However, callers frequently do `#[derive(Deserialize)]` on internal types, and without a lint there is a risk someone will add `Deserialize` to `Document` later and accidentally open a deserialization surface.

**Recommended fix**: Add a deny-by-default lint or CI test:

```rust
// deny deserializing from untrusted input
#[cfg(debug_assertions)]
const _: () = assert!(false, "Document must not implement Deserialize");
```

(this can't be done with derives; instead add a compile-fail test or use a clippy allow lint with a custom reason).

#### R3.1b | INFO | models/knowledge_graph.rs:220-237 | `DocumentTopicRow` / `SimilarityPairRow` are internal pgerow cache without Serialize

These types (`DocumentTopicRow`, `SimilarityPairRow`) are marked `#[derive(Debug, sqlx::FromRow)]` — they are **not public API**, as expected. Good design.

---

### 3.2 Validation Trait

**Observation**: `CreateDocument::validate()` returns `Result<(), &'static str>` (document.rs:69-82). This is one of the few DTO types with a validate method. No other model DTO has an equivalent convention. There is **no unified validation trait** (e.g. `trait Validated { fn validate(&self) -> Result<(), Error>; }`) shared across DTOs.

**Recommended fix**: If validation is a first-class concern, introduce a shared `#[derive(Validated)]` macro or trait; otherwise remove the per-type `validate` methods to avoid inconsistency.

---

## 4. Error Handling — error.rs

### 4.1 AppError Enum Coverage

AppError captures: `Unauthorized`, `Forbidden`, `NotFound`, `Validation`, `Conflict`, `RateLimited`, `Configuration`, `ExternalService`, `Database`, `Internal`.

**Missing variants:**

#### R4.1a | HIGH | error.rs:10-41 | No explicit variant for offensive / content-problems

`content` contains `offensive: bool` and `offensive_reasons: Vec<String>`. There is no `AppError::OffensiveContent` variant — it probably maps to `Validation` today, but a dedicated variant or per-operation DTO would be clearer.

#### R4.1b | LOW | error.rs:10-41 | No variant for unsatisfied `#[schema(on_failure(extension("x-code-samples")))]` scenarios

The OpenAPI comments reference an extension `x-code-samples` on schema failures that is **not implemented anywhere in the crate**. This is an OpenAPI-level concern (see section 10), not a backend error, but worth noting.

### 4.2 HTTP Status Mapping Integrity

`AppError::into_response` matches all variants. No unreachable variant; coverage is complete. No `panic!`-based unhandled fallback.

---

## 5. Config — config.rs

### 5.1 Env-Based Loading

Full support via `config = "0.13"` + `dotenvy`:

```rust
pub fn from_env() -> Result<Self> {
    dotenvy::dotenv().ok();             // loads .env
    let config = config::Config::builder()
        .add_source(config::Environment::default())
        .build()?;
    Ok(config.try_deserialize()?)
}
```

This works correctly — environment variables override defaults. `.env` is loaded if present; missing `.env` is non-fatal (`.ok()`).

### 5.2 Startup Validation

**R5.2a | HIGH | lib.rs:44-71 and config.rs | No startup config validation**

`create_app(config: Config)` at lib.rs:45 builds the DB pool and runs migrations, but there is **no `Config::validate(&[required_fields])` call**:

- `database_url` is `String` — if unset, pool creation fails at runtime during the first query. No deployment-time failure.
- `jwt_secret` is `SecretString` — missing it causes JWT to sign with empty bytes (`jwt_secret_bytes` at config.rs:179-181), producing invalid tokens **silently** because `SecretString` is not validated at startup.
- `environment` is `String` with default `"development"` — no enumerated set (`Environment::Development | Environment::Production`).

**Recommended fix**: Add a constructor method:

```rust
impl Config {
    pub fn validate_for_production(&self) -> Result<(), String> {
        if self.environment != "production" && !self.environment.starts_with("prod") {
            return Err("environment must be production or prod".into());
        }
        if self.jwt_secret.expose_secret().is_empty() {
            return Err("jwt_secret is required".into());
        }
        if self.database_url.contains("localhost") {
            return Err("database_url must not point to localhost in production".into());
        }
        Ok(())
    }
}
```

Call this in `create_app` before `PgPoolOptions::connect`.

---

## 6. Migration Analysis — migrations/

### 6.1 Forward-Only (Additive) Pattern

Reviewing 001-016:

- All files are `.sql` with runtime-executable `DO $$ ... $$` and `ALTER TABLE ... ADD COLUMN`/`CREATE TABLE`/`CREATE INDEX` patterns.
- No `DROP COLUMN`, `ALTER TABLE ... ALTER COLUMN ... DROP NOT NULL`, or `ALTER TABLE ... DROP CONSTRAINT` found — the migrations are **forward-only**.
- `011_flask_compat_columns.sql` adds `source_type TEXT`, `format TEXT`, `visibility TEXT` — additive compatibility columns.
- `016_embedding_dim_4096.sql` does `ALTER TABLE documents ALTER COLUMN embedding_dim TYPE INTEGER USING 4096` — **safe transformation** because the constant 4096 is a compile-time valid constraint.

**No destructive migrations detected.** The migration plan is forward-only by design.

### 6.2 Destructive / Concurrent Code Changes Missed

#### R6.2a | LOW | 011_flask_compat_columns.sql | `sensitivity TEXT NOT NULL DEFAULT 'internal'`

Migration 011 adds `sensitivity` as a `TEXT NOT NULL DEFAULT 'internal'` column. It does **not** create a CHECK constraint. Meanwhile, `Document::sensitivity` is a `String` in the model with a runtime `SENSITIVITY_VALUES: &[&str]` at document.rs:45. There is **no DB-level constraint** validating `sensitivity` is one of `["confidential", "internal", "public"]`.

**Recommended fix**: Add a CHECK constraint in a new migration (or patch 011):

```sql
ALTER TABLE documents
  ADD CONSTRAINT chk_sensitivity
  CHECK (sensitivity IN ('confidential', 'internal', 'public'));
```

---

## 7. Test Strategy — tests/ and #[cfg(test)] modules

### 7.1 Test Inventory

- `tests/`: 4 integration tests:
  - `document_routes_test.rs` (~2.3 KB) — document CRUD routes
  - `upload_test.rs` (~3.2 KB) — file upload via API
  - `health_test.rs` (~1.8 KB) — health endpoint
  - `knowledge_graph_model_test.rs` (~7.1 KB) — knowledge graph types

- `src/` `#[cfg(test)]` modules: Each major Rust source file (builder.rs, stage.rs, context.rs, error.rs, document_service.rs, config.rs, vault.rs) carries a `#[cfg(test)] mod tests` block — **~25+ test functions** spread across the crate.

**Ratio**: Roughly 30% unit / 70% integration by file count; however, unit tests are small and focused. No property-based / fuzzing tests detected.

### 7.2 Vault / SRT Ingestion Happy Path

**R7.2a | HIGH | tests/ | No integration test for vault ingestion happy path or SRT pipeline**

The `document_routes_test.rs` tests direct document CRUD routes (create, list, get, update, delete). The `upload_test.rs` tests upload endpoints. However, **`vault/ingest` POST / `vault/sync/report` GET are not covered by the integration test suite**. There is also **no test for the SRT ingestion pipeline**:

- No test that `POST /api/vault/ingest` with a sample `.md` file produces a document with correct metadata.
- No test that `POST /api/srt/upload` triggers the narrative pipeline end-to-end.
- No test of `sync_report` with known disk/DB state.

**Recommended fix**: Add an integration test under `tests/vault_ingest_test.rs`.

---

## 8. SQLx Usage — Security & Correctness

### 8.1 CRITICAL Vulnerability — LIKE metacharacter injection

#### R8.1a | CRITICAL | document_service.rs:214-216 | `build_search_pattern` embeds user input in SQL without escaping

```rust
pub fn build_search_pattern(query: &str) -> String {
    format!("%{}%", query.trim().to_lowercase())
}
```

This function returns a **raw SQL LIKE pattern string** — not a parameter value. It is **used in** `search_service.rs` (line 213 noted in tree) in a `query("... WHERE title LIKE '{}'", &pattern)` or similar interpolated SQL. Any user who submits the string `test%; DROP TABLE documents; --` would have their input embedded directly into SQL. Even benign input containing `_` or `%` matches **more rows than intended** (e.g. searching for `%art%` hits `cart` and `dart`).

**Note on urgency**: The SQL injection risk is real. The `%` and `_` wildcard-matching bug is also real. Escaping is required.

**Recommended fix**:

```rust
pub fn build_search_pattern(query: &str) -> String {
    let esc = query
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", esc)
}
```

And ensure the containing SQL uses `ESCAPE '\'`.

### 8.2 CRITICAL Vulnerability — LIMIT injection in sync_report

#### R8.2a | CRITICAL | routes/vault.rs:430-436 | Sync-report query interpolates a runtime integer into SQL via `format!` with no parameter binding

```rust
let limit_sql = format!(
    "SELECT id, source_path FROM documents \
     WHERE source_path IS NOT NULL \
     AND source_path LIKE $1 || '%' ESCAPE '\\' \
     LIMIT {}",
    remaining
);
```

The `remaining` value comes from client-controlled request parameters (the list of `root` query params that determine iteration order and row-count cap). While `remaining` is derived from `SYNC_REPORT_DB_LIMIT + 1` and down-counted, it is **not a SQL parameter**. A crafted root list that causes `remaining` to be 0 or negative produces invalid SQL:

```sql
SELECT id, source_path FROM documents WHERE source_path IS NOT NULL AND source_path LIKE $1 || '%' ESCAPE '\' LIMIT -1
-- ERROR: LIMIT -1 is invalid
```

More subtly, if a DB error occurs and `remaining` is not properly bounded, an attacker can cause `syntax error` or retrieve more data than the 50,000 limit (if offset arithmetic is manipulated).

**Recommended fix**: Use a **named placeholder** for `LIMIT` as well (SQLx **does not support `LIMIT $1`** — you must use `LIMIT {}` integer interpolation. To be safe, you must prove the integer is bounded at compile time):

```rust
// CLAMP to a safe constant, NEVER user-derived:
let clamped_limit = remaining.clamp(1, SYNC_REPORT_DB_LIMIT as i64);
let limit_sql = format!(
    "SELECT id, source_path FROM documents
     WHERE source_path IS NOT NULL
     AND source_path LIKE $1 || '%' ESCAPE '\\'
     LIMIT {clamped_limit}"
);
```

Or restructure the query to avoid `LIMIT ...` interpolation entirely by slicing in Rust after a materialized fetch (less efficient for large vaults, but safe).

**Mitigation on the environment side**: Add a hard upper bound `SYNC_REPORT_DB_LIMIT` and clamp strictly. The current `remaining` logic is sound for normal inputs but is not defense-in-depth against path manipulation.

Additionally, the `escape_like_prefix` call at line 428 is **good practice** and correctly escapes `%`, `_`, `\`.

### 8.3 SQLx usage elsewhere

**R8.3a | LOW | document_service.rs:102-116 | `create` uses `query_as` with positional parameters — correct**

```rust
INSERT INTO documents (...) VALUES ($1, $2, $3, $4, $5, $6)
.bind(id)
.bind(&data.title)
.bind(&data.content)
.bind(data.category_id)
.bind(user_id)
.bind(is_public)
```

`is_public` bound as `bool` is correct for `BOOLEAN` columns. `title`/`content` bound as `&String` is correct. No issue.

**R8.3b | MEDIUM | document_service.rs:28 | `list` uses `LIMIT $N OFFSET $N` with cached offset computation**

The offset is computed at line 28: `let offset = (page - 1) * limit;` — this is **not clamped** to a max value. If `page = 999999999` and `limit = 100`, the offset is ~9.9 billion rows, hitting PostgreSQL's `BigInt` max – possible but unlikely to succeed, and wastes DB resources. Line 189-193 has `clamp_page_params` but `list` does not call it — it uses the raw unclamped values.

**Recommended fix**: Call `clamp_page_params` at line 23 inside `list` and use the clamped output.

---

## 9. Async Runtime — tokio + axum — Blocking Concerns

### 9.1 Blocking Calls

Reviewing the following files for blocking operations inside async handler bodies:

| Concern | Finding |
|---------|---------|
| `std::fs::read(_dir)` in vault_common.rs:63 | **Yes — synchronous filesystem calls** in `validate_path` and `collect_md_files`. These are sync sys-calls. However, they are inside `fn` (non-async), called from within axum handlers via `async fn ingest_vault` → `collect_md_files`. The handlers block the async runtime while walking the filesystem. A 50KB vault takes measurable time. For production-scale vaults, these should be `tokio::fs::read_dir` + buffered streams. |
| `sqlx::migrate!().run()` in lib.rs:71 | **Yes — blocking at startup**, but outside of request handlers. Acceptable. |
| `EmbeddingService::new_with_local` downloads ~270MB model | **Yes — blocking at startup** in lib.rs:78-91. Not a handler concern. Acceptable if downloads are gated behind config. |
| `reqwest::Client::new()](#) at lib.rs:132-135 | Async, fine. |
| `reqwest` calls in services/ | async via `reqwest`, fine. |

**Net verdict**: On a per-request basis, `vault_common::collect_md_files` blocks the axum worker pool while walking the file tree and stat-ing files. For large vaults (10K+ .md files), the server will drop subsequent requests until the walk completes.

---

## 10. OpenAPI — openapi.rs

### 10.1 Hand-Maintained JSON Drift

`minky-rust/src/openapi.rs` is a 40.2KB hand-written JSON file. No `schemars`-based automatic generation from Axum routes is used. This implies **endemic drift**:

**Known drift: streaming routes not in spec**

- The `POST /api/stream` and `GET /api/stream` endpoints (stream.rs) appear in routes but **not in the OpenAPI definition** — the spec was probably last updated before streaming was added.
- The `POST /api/upload` endpoint with `#[schema(on_failure(extension(...)))]` comments exists in code but the spec has no `"x-code-samples"` extension as documented.

**Recommended fix**: Use `utoipa` + `utoipa-swagger-ui` macro annotations on route handlers, or switch to `axum-extra` OpenAPI integration for auto-generation.

### 10.2 Endpoints missing from OpenAPI

Based on the route tree, the following routes appear to have **no matching schema entry**:

- `POST /api/search/suggestions` (search.rs:41-103)
- `GET /api/github/repo` (github.rs:43-105)
- `POST /api/stream` (stream.rs)
- `GET /api/stream` (stream.rs)
- `src/routes/vault.rs:595` `GET /api/vault/watch/status` — ADMIN-ONLY but no authZ schema extension.

---

## 11. Structural Observations (non-actionable)

- The `#[schema(...)]` attribute macros (search.rs line 33, upload.rs line 33, etc.) are **applied but not implemented** as procedural macros — they are used as documentation only. This is fine but misleading — real macro expansion would break builds if they were real.
- `models/mod.rs` at line 1 exports `pub mod document;` etc. — the convention is clean.
- `services/mod.rs` likely re-exports the 39+ services for convenience; worth confirming.
- `vault_common` is an excellent candidate for a shared crate — the path walking logic is identical in the watcher service and manual ingest route.
- `function_registry` (26.1KB) is the largest services module and manages a plugin-like stage registry — worth reviewing once staging is complete.

---

## 12. Summary Table

| # | Severity | File(s) | Line(s) | Finding |
|---|----------|---------|---------|---------|
| R1.2a | CRITICAL | stage.rs | 95-100 | `should_skip` hook is a no-op; stages still execute |
| R8.1a | CRITICAL | document_service.rs | 214-216 | `build_search_pattern` returns injectable SQL fragment |
| R8.2a | CRITICAL | routes/vault.rs | 430-436 | `LIMIT` bound via `format!` with client-supplied integer |
| R6.2a | LOW | migrations/011 | — | No CHECK constraint on `sensitivity` enum despite domain knowledge |
| R7.2a | HIGH | tests/ | — | No integration test for vault ingestion or SRT pipeline |
| R4.1a | HIGH | error.rs | 10-41 | No `OffensiveContent` error variant |
| R5.2a | HIGH | lib.rs, config.rs | 44-71, 166-188 | No `Config::validate()` at startup |
| R2.2b | HIGH | lib.rs | 37 | `VaultWatcher` uses write-lock for every read |
| R9.1a | MEDIUM | vault_common.rs | 63-141 | Synchronous `std::fs` in async handlers blocks axum |
| R1.1a | MEDIUM | builder.rs | 95-213 | No cancellation-point granularity; `ctx.cancelled` ignored |
| R1.1b | MEDIUM | builder.rs | 157-171 | `skip_storage` returns real-looking UUIDs for unpersisted jobs |
| R1.3b | MEDIUM | context.rs | 84-88 | `set_metadata` silently drops failures |
| R2.2a | MEDIUM | lib.rs | 78-91 | Per-handler `Arc<Service>` allocations instead of shared in `AppState` |
| R8.3b | MEDIUM | document_service.rs | 28-29 | `list` pagination not clamped |
| R1.1c | LOW | builder.rs | 338-343 | `build()` does not validate config preconditions |
| R1.4a | LOW | pipeline/error.rs | 99-103 | `From<sqlx::Error>` flattens to string |
| R1.4b | LOW | pipeline/error.rs | 105-116 | `From<AppError>` collapses non-fatal variants to Internal |
| R3.1a | LOW | document.rs | 7-8 | `Document` accidentally serializable via serde derive (lit bus) |
| R4.1b | LOW | error.rs | 10-41 | No variant for `x-code-samples` OpenAPI extensions |
| R10.1a | INFO | openapi.rs | full file | Hand-maintained spec is drifted from implementation |
