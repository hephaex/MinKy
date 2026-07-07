//! Ultragoal G006 — LLMWiki (`generate_wiki`) acceptance QA.
//!
//! End-to-end integration tests that drive the *public* wiki-generation surface
//! (`broker::wiki::generate` and the MCP-level `broker::mcp_server::*` tools)
//! against synthetic mock servers: a wiremock minky backend for the retrieval
//! cascade evidence and a wiremock OpenAI-compatible synthesis endpoint for the
//! LLM body. Nothing here touches a real network service or the developer's
//! broker state — every filesystem root is an isolated `TempDir`.
//!
//! The eight LLMWiki acceptance criteria are covered as follows:
//!   - AC1 (tool call -> markdown + path): `ac1_generate_writes_markdown_and_returns_path`,
//!     `ac1b_generate_wiki_tool_returns_path`
//!   - AC2 (paragraph footnotes): `ac2_document_contains_footnote_marker`
//!   - AC3 (unverified banner): `ac3_document_contains_unverified_banner`
//!   - AC4 (re-request preserves + versions): `ac4_rerequest_preserves_old_and_adds_new_version`
//!   - AC5 (Obsidian-compatible structure): `ac5_document_is_obsidian_compatible`
//!   - AC6 (minky offline -> no file): `ac6_minky_offline_writes_nothing_and_errors`
//!   - AC7 (synth offline -> no file): `ac7_synth_offline_writes_nothing_and_errors`
//!   - AC8 (self-reference prevention): `ac8_generated_wiki_excluded_from_working_set`
//!     (unix, symlink e2e), `ac8_output_dir_inside_memory_root_refused`

use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use broker::config::BrokerConfig;
use broker::mcp_server;
use broker::minky_client::MinkyClient;
use broker::storage::Storage;
use broker::synth_client::SynthClient;
use broker::wiki::{self, WikiError};

use tempfile::TempDir;
use wiremock::matchers::{method, path as req_path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Real "now" so freshly-written memory files land inside the working-set
/// access window (a far-future constant would soft-demote everything).
fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// A guaranteed-closed loopback URL: bind an ephemeral port then drop the
/// listener, so any connection attempt is refused immediately.
fn closed_loopback_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// A minky `/api/search/semantic` success envelope from `(similarity, title)`
/// pairs, matching the shape the real backend returns.
fn semantic_envelope(results: &[(f32, &str)]) -> serde_json::Value {
    let arr: Vec<_> = results
        .iter()
        .enumerate()
        .map(|(i, (sim, title))| {
            serde_json::json!({
                "document_id": format!("00000000-0000-0000-0000-{:012}", i),
                "chunk_id": null,
                "chunk_text": format!("evidence snippet for {title}"),
                "similarity": sim,
                "document_title": title,
            })
        })
        .collect();
    serde_json::json!({
        "success": true,
        "data": { "results": arr, "total": arr.len(), "query": "q" }
    })
}

/// Start a live minky mock returning a fixed set of semantic hits.
async fn start_minky_mock(results: &[(f32, &str)]) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(req_path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(semantic_envelope(results)))
        .mount(&server)
        .await;
    server
}

/// Start a live synthesis mock returning `content` as the chat-completion body.
async fn start_synth_mock(content: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(req_path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [ { "message": { "content": content } } ]
        })))
        .mount(&server)
        .await;
    server
}

/// Owns every temp dir + config + storage for a single test, so they outlive
/// the mock servers and clients that borrow their paths.
struct Harness {
    _memory: TempDir,
    _output: TempDir,
    _keys: TempDir,
    config: BrokerConfig,
    storage: Storage,
    output_dir: PathBuf,
    memory_root: PathBuf,
}

impl Harness {
    /// Build an isolated harness. `minky_url`/`synth_url` are wired into the
    /// config; synthesis is enabled with short timeouts. The memory root and
    /// output dir are independent temp locations, so the output is always
    /// self-reference-safe unless a test deliberately overrides it.
    fn new(minky_url: &str, synth_url: &str) -> Self {
        let memory = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let keys = TempDir::new().unwrap();

        let key_path = keys.path().join(".api-key");
        let mut f = std::fs::File::create(&key_path).unwrap();
        writeln!(f, "test-key").unwrap();

        let memory_root = memory.path().to_path_buf();
        let output_dir = output.path().to_path_buf();

        let mut config = BrokerConfig::default();
        config.working_set.memory_root = memory_root.clone();
        // No pins: each test controls working-set membership precisely and the
        // cascade always falls through to the (mocked) minky backend.
        config.working_set.always_include = Vec::new();
        config.minky.base_url = minky_url.to_string();
        config.minky.api_key_file = key_path.clone();
        config.minky.timeout_secs = 2;
        config.synthesis.enable = true;
        config.synthesis.base_url = synth_url.to_string();
        config.synthesis.api_key_file = key_path;
        config.synthesis.timeout_secs = 2;
        config.wiki.output_dir = output_dir.clone();

        Harness {
            _memory: memory,
            _output: output,
            _keys: keys,
            config,
            storage: Storage::open_in_memory().unwrap(),
            output_dir,
            memory_root,
        }
    }

    fn minky(&self) -> MinkyClient {
        MinkyClient::new(&self.config.minky).unwrap()
    }

    fn synth(&self) -> SynthClient {
        SynthClient::new(&self.config.synthesis)
    }
}

/// Count regular files directly inside `dir` (0 if the dir does not exist).
fn files_in(dir: &Path) -> usize {
    match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count(),
        Err(_) => 0,
    }
}

/// Default synthesized body used by happy-path tests. Includes an in-paragraph
/// footnote marker in addition to the ones the renderer appends.
const SYNTH_BODY: &str =
    "러스트의 소유권은 메모리 안전성을 보장한다.[^1]\n\n대여 규칙은 이를 강화한다.[^2]";

// ---------------------------------------------------------------------------
// AC1 — tool call produces a markdown document and returns its path.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac1_generate_writes_markdown_and_returns_path() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    let mc = h.minky();
    let sc = h.synth();
    let path = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .expect("generate should succeed with live minky + synth");

    assert!(path.exists(), "returned path must point at a real file");
    assert!(path.starts_with(&h.output_dir), "file must live under output_dir");
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(!body.trim().is_empty(), "generated document must not be empty");
    assert_eq!(files_in(&h.output_dir), 1, "exactly one document written");
}

#[tokio::test]
async fn ac1b_generate_wiki_tool_returns_path() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    // Exercise the MCP tool layer, not just the wiki module directly.
    let mc = h.minky();
    let sc = h.synth();
    let result = mcp_server::generate_wiki_tool(
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        "rust ownership",
        "러스트 소유권",
        now_ts(),
    )
    .await
    .expect("generate_wiki_tool should succeed");

    let path = PathBuf::from(&result.file_path);
    assert!(path.exists(), "MCP tool must return a path to a real file");
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("# 러스트 소유권"), "document must carry the topic title");
}

// ---------------------------------------------------------------------------
// AC2 — every document carries at least one paragraph footnote marker.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac2_document_contains_footnote_marker() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    let mc = h.minky();
    let sc = h.synth();
    let path = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .unwrap();

    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("[^1]"), "document must contain a [^1] footnote marker");
}

// ---------------------------------------------------------------------------
// AC3 — every document carries the mandatory unverified banner.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac3_document_contains_unverified_banner() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    let mc = h.minky();
    let sc = h.synth();
    let path = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .unwrap();

    let body = std::fs::read_to_string(&path).unwrap();
    assert!(
        body.contains("AI 자동생성"),
        "document must carry the unverified 'AI 자동생성' banner"
    );
}

// ---------------------------------------------------------------------------
// AC4 — re-requesting the same topic preserves the old file and adds a new
//        version (never clobbers / overwrites).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac4_rerequest_preserves_old_and_adds_new_version() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    let now = now_ts();
    let mc = h.minky();
    let sc = h.synth();
    let first = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now,
    )
    .await
    .unwrap();

    // Different UTC second -> a distinct timestamped filename.
    let second = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now + 1,
    )
    .await
    .unwrap();

    assert!(first.exists(), "first version must survive the re-request");
    assert!(second.exists(), "second version must be written");
    assert_ne!(first, second, "re-request must produce a new version, not overwrite");
    assert_eq!(files_in(&h.output_dir), 2, "both versions must coexist");
}

// ---------------------------------------------------------------------------
// AC5 — the document is a valid, Obsidian-compatible markdown structure:
//        YAML front-matter + footnotes + a sources section.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac5_document_is_obsidian_compatible() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    let mc = h.minky();
    let sc = h.synth();
    let path = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .unwrap();

    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.starts_with("---\n"), "must open with YAML front-matter");
    // Front-matter must be a closed block (a second `---` fence after the open).
    assert!(
        body[4..].contains("\n---\n"),
        "YAML front-matter must be terminated by a closing fence"
    );
    assert!(body.contains("[^"), "must contain Obsidian footnote markers");
    assert!(body.contains("## 출처"), "must contain a sources section");
    assert!(
        body.contains("[^1]:"),
        "must contain a resolvable footnote definition"
    );
}

// ---------------------------------------------------------------------------
// AC6 — minky unreachable: hard-refuse, write nothing, MinkyOffline error.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac6_minky_offline_writes_nothing_and_errors() {
    // minky on a closed port; synth is live but must never be reached.
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&closed_loopback_url(), &synth.uri());

    let mc = h.minky();
    let sc = h.synth();
    let err = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .expect_err("unreachable minky must fail");

    assert!(matches!(err, WikiError::MinkyOffline), "got {err:?}");
    assert_eq!(files_in(&h.output_dir), 0, "no document may be written when offline");
}

// ---------------------------------------------------------------------------
// AC7 — synthesis endpoint unreachable: hard-refuse, write nothing,
//        SynthOffline error. minky must be live with evidence so the cascade
//        reaches the synthesis stage (an offline minky would short-circuit
//        first with MinkyOffline).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ac7_synth_offline_writes_nothing_and_errors() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let mut h = Harness::new(&minky.uri(), &closed_loopback_url());

    let mc = h.minky();
    let sc = h.synth();
    let err = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .expect_err("unreachable synthesis endpoint must fail");

    assert!(matches!(err, WikiError::SynthOffline(_)), "got {err:?}");
    assert_eq!(
        files_in(&h.output_dir),
        0,
        "no document may be written when synthesis is offline"
    );
}

// ---------------------------------------------------------------------------
// AC8 — self-reference prevention.
//
// (AC8.1: `resolve_safe_output_dir` is already covered by config unit tests
//  (G002); not re-verified here.)
//
// AC8.2: end-to-end proof that a generated wiki file reachable *through* the
//        memory root (via a symlink) is excluded from the working-set scan, so
//        the broker never re-ingests its own output as evidence.
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn ac8_generated_wiki_excluded_from_working_set() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    // A legitimate memory file: proves the scan actually runs and returns
    // real members. Its content does not match the query, so the cascade
    // falls through to the (mocked) minky backend for evidence.
    std::fs::write(h.memory_root.join("note.md"), "무관한 요리 관련 메모").unwrap();

    // A symlink inside the memory root pointing at the wiki output dir: without
    // the self-reference guard the generated page would be discovered here.
    std::os::unix::fs::symlink(&h.output_dir, h.memory_root.join("wiki-link")).unwrap();

    let now = now_ts();
    let mc = h.minky();
    let sc = h.synth();
    let path = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now,
    )
    .await
    .unwrap();

    let wiki_name = path.file_name().unwrap().to_string_lossy().into_owned();

    let snapshot = mcp_server::load_working_set_tool(&h.config, &mut h.storage, now).unwrap();

    let appears = snapshot
        .members
        .iter()
        .chain(snapshot.demoted.iter())
        .any(|item| item.rel_path.contains(&wiki_name) || item.rel_path.contains(".md-"));
    assert!(
        !appears,
        "generated wiki '{wiki_name}' must NOT appear in the working set (self-reference guard); \
         members={:?} demoted={:?}",
        snapshot.members, snapshot.demoted
    );

    // Sanity: the legitimate memory file IS visible, proving the scan is live
    // and the exclusion above is meaningful rather than an empty scan.
    let sees_note = snapshot
        .members
        .iter()
        .chain(snapshot.demoted.iter())
        .any(|item| item.rel_path == "note.md");
    assert!(sees_note, "the ordinary memory file must be scanned normally");
}

// AC8.3: a misconfigured output dir *inside* the memory root is refused with
//        UnsafeOutputDir and no document is written.
#[tokio::test]
async fn ac8_output_dir_inside_memory_root_refused() {
    let minky = start_minky_mock(&[(0.91, "ownership.md"), (0.72, "borrowing.md")]).await;
    let synth = start_synth_mock(SYNTH_BODY).await;
    let mut h = Harness::new(&minky.uri(), &synth.uri());

    // Operator error: point the wiki output at a path under the memory root.
    let unsafe_dir = h.memory_root.join("gyeol-wiki");
    h.config.wiki.output_dir = unsafe_dir.clone();

    let mc = h.minky();
    let sc = h.synth();
    let err = wiki::generate(
        "rust ownership",
        "러스트 소유권",
        &h.config,
        &mut h.storage,
        &mc,
        &sc,
        now_ts(),
    )
    .await
    .expect_err("output dir inside memory root must be refused");

    assert!(matches!(err, WikiError::UnsafeOutputDir(_)), "got {err:?}");
    assert!(
        !unsafe_dir.exists(),
        "no document (and no output dir) may be created when the path is unsafe"
    );
}
