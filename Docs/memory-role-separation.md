# Memory System Role Separation

This document defines the boundaries between minky's three local memory/knowledge
systems, and the cold-storage eligibility of each, per the C4 goal of the
minky-gyeol memory integration plan.

## The three systems

### 1. gyeol `.memory/` — identity/episodic long-term memory (SOUL)

- **What it is**: the gyeol project's long-lived, identity-bearing memory —
  episodic session logs, semantic/domain knowledge, relationship/preference
  ("bond") state, and periodic reflections. This is the durable "self" memory
  that persists across sessions and projects.
- **Storage today**: local files under `~/Simon/Gaeyang/.memory/...` (episodes,
  semantics, bonds, reflections), one-way synced into minky via the external
  `~/.config/gyeol/scripts/sync-memory-to-minky.sh` script (Sprint 8), landing
  as `documents` rows categorized under the `memory` category tree (slugs
  `memory-episode`, `memory-semantic`, `memory-bond`, `memory-reflection`,
  `memory-reference` — see migration `015_memory_category_seed.sql`).
- **Cold-eligibility**: **cold-eligible**. This is exactly the kind of
  long-lived, infrequently-updated, semantically rich content that benefits
  from being embedded and made searchable in a durable backend rather than
  living only in flat files.

### 2. CLI Agent Auto-Memory — ephemeral per-project state

- **What it is**: the CLI agent's (Claude Code / GJC) per-project working memory/state,
  scoped to a single coding session or project context.
- **Storage today**: managed entirely by the CLI agent itself, outside minky.
- **Cold-eligibility**: **explicitly EXCLUDED from the cold backend.** This
  memory is ephemeral and project-scoped by design; promoting it into a
  durable, cross-session, cross-LLM-vendor backend like minky would violate
  its intended lifecycle (it should not outlive the project/session it
  belongs to, and it is not meant to be shared across different LLM
  vendors/tools). minky-gyeol integration must never ingest, embed, or
  otherwise persist Auto-Memory content.

### 3. `~/.gjc/references/` — technical reference corpus

- **What it is**: a corpus of technical reference material (architecture
  notes, past review reports, ADR-style documents, operational patterns) —
  not personal/episodic memory, but durable technical knowledge.
- **Storage today**: flat markdown/text files under `~/.gjc/references/`.
  Ingestion into minky (via the vault/DocumentPipeline File source) attributes
  these documents by `source_path` under a references-root prefix,
  deliberately kept **disjoint** from the gyeol `memory-*` category
  attribution — a references-ingested document must never carry a `memory-*`
  category, and a gyeol-synced document must never carry a references
  `source_path`. This is what lets minky determine "is this a gyeol memory
  document or a references document?" purely from existing schema (no new
  `source_system` column).
- **Cold-eligibility**: **cold-eligible**, for the same reason as gyeol
  memory — durable, rarely-changing technical knowledge that benefits from
  being embedded and searchable.

## Cold-eligibility matrix

| System                          | Cold-eligible | Attribution in minky                                  | Lifecycle                          |
|----------------------------------|:-------------:|---------------------------------------------------------|-------------------------------------|
| gyeol `.memory/` (SOUL)          | ✅ Yes        | `documents.category_id` in the `memory-*` category set  | Durable, cross-session, cross-tool  |
| CLI Agent Auto-Memory            | ❌ **No**     | N/A — never ingested into minky                          | Ephemeral, project/session-scoped   |
| `~/.gjc/references/` corpus      | ✅ Yes        | `documents.source_path` under the references-root prefix | Durable, rarely-changing            |
| Everything else (documents not matching either predicate above) | ❌ No (excluded from MCP scope) | Neither predicate | N/A |

## Relationship to the MCP source-scope filter

The MCP federation surface (`services::mcp_server`) enforces this exact
boundary at query time: a document is visible over MCP if and only if it
matches the gyeol memory-* category predicate OR the references source_path
predicate. Documents matching neither — which by construction includes
anything derived from CLI Agent Auto-Memory, since that system is never
ingested into minky at all — are excluded from every MCP tool's results
(`minky_search`, `minky_vsearch`, `minky_deep_search`, `minky_get`,
`minky_multi_get`). See `services::mcp_server::MCP_SOURCE_SCOPE_SQL` for the
concrete SQL predicate and `services::mcp_server::references_root_prefix` for
the (currently unconfigured-by-default, fail-safe-empty) references-root
prefix, which is intentionally deferred to the still-blocked C0 corpus
ingestion goal to define concretely.

## Explicitly out of scope for this document / the C4 MCP surface

Per the original deep-interview spec, the following remain intentionally
deferred and are NOT addressed by this document or the MCP handler:

- Automatic hot/cold promotion policy (when/how content moves between
  "hot" per-session state and this "cold" durable backend).
- A full bidirectional sync/conflict engine between gyeol and minky.
- Any MCP-extension completion criteria beyond what is stated in the C4 goal
  itself (the six `minky_*` tools, source-scope filtering, and this
  role-separation document).
- The external Python `minky-mcp.py` server and CLI Agent Auto-Memory are
  explicitly out of scope for the Rust MCP surface implemented in
  `services::mcp_server` / `routes::mcp`.
