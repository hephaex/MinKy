//! Retrieval cascade — the broker's read-path decision procedure.
//!
//! Three stages (approved plan WP3):
//!   1. **Working set** — token-match the query against the short-term set
//!      (filename + first N KB of content). Any hit short-circuits to
//!      `Sufficient`; the matched items are re-accessed (recall) and returned
//!      as evidence. No similarity score exists at this stage, so `scores` is
//!      empty and the reason is spelled out.
//!   2. **minky semantic search** — only when the working set misses. Hits are
//!      converted to vendor-neutral [`Evidence`] (snippet + source, never a
//!      synthesized answer). The verdict is decided purely from the
//!      externalized thresholds in [`crate::config::VerdictConfig`]. Any hit
//!      that corresponds to a local memory file is recalled.
//!   3. **degradation** — a connection failure/timeout ([`MinkyError::Unreachable`])
//!      maps to `Verdict::Offline`; real HTTP errors propagate as `Err`.
//!
//! Verdict rules (thresholds are config-driven; nothing is hard-coded):
//!   - `sufficient`   = `hits >= min_results` AND `top1 >= sufficient_top1`
//!   - `ask-user`     = `hits == 0` OR `top1 < min_score`
//!   - `insufficient` = everything in between (evidence returned, no confidence)

use crate::config::BrokerConfig;
use crate::minky_client::{MinkyClient, MinkyError};
use crate::storage::{Storage, StorageError};
use crate::working_set::{WorkingSet, WorkingSetError, WorkingSetSnapshot};
use std::path::Path;

/// How much of each working-set file to scan for token matches. Not a verdict
/// threshold — a plain I/O bound, so it stays a local constant.
const CONTENT_SCAN_BYTES: usize = 8 * 1024;

/// Access-source tags written to the audit log.
const SOURCE_WORKING_SET: &str = "working-set";
const SOURCE_RECALL: &str = "recall";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Enough high-confidence evidence to proceed without asking.
    Sufficient,
    /// Some evidence, but below the confidence bar — surface it, flag doubt.
    Insufficient,
    /// No usable evidence — the caller should ask the user.
    AskUser,
    /// Long-term memory is unreachable; proceed on the working set alone.
    Offline,
}

/// A single piece of returned evidence — snippet plus its source. The broker
/// never synthesizes an answer; it hands back sources only (vendor-neutral).
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub title: String,
    pub snippet: String,
    pub source_id: String,
    pub similarity: f32,
}

/// Full result of a cascade run.
#[derive(Debug, Clone, PartialEq)]
pub struct CascadeOutcome {
    pub verdict: Verdict,
    pub reason: String,
    pub scores: Vec<f32>,
    pub hit_count: usize,
    pub evidence: Vec<Evidence>,
    /// Local memory paths that were re-entered (working-set match or recall).
    pub working_set_matches: Vec<String>,
    pub offline: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum CascadeError {
    #[error("working set error: {0}")]
    WorkingSet(#[from] WorkingSetError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    /// Only real HTTP errors reach here; `Unreachable` is handled as `Offline`.
    #[error("minky error: {0}")]
    Minky(#[from] MinkyError),
}

/// Run the retrieval cascade for `query` at time `now` (epoch seconds).
pub async fn run(
    query: &str,
    config: &BrokerConfig,
    storage: &mut Storage,
    minky: &MinkyClient,
    now: i64,
) -> Result<CascadeOutcome, CascadeError> {
    // ---- Stage 1: working set ------------------------------------------------
    let snapshot: WorkingSetSnapshot = {
        let mut ws = WorkingSet::new(&config.working_set, storage)
            .with_exclude_dir(Some(config.wiki.output_dir.clone()));
        ws.evaluate(now)?
    };

    let tokens = tokenize(query);
    let mut ws_evidence = Vec::new();
    let mut ws_matches = Vec::new();

    for member in &snapshot.members {
        let path = config.working_set.memory_root.join(&member.rel_path);
        let content = read_head(&path);
        let name_hay = member.rel_path.to_lowercase();
        let content_hay = content.to_lowercase();
        let matched = tokens
            .iter()
            .any(|t| name_hay.contains(t) || content_hay.contains(t));
        if matched {
            ws_evidence.push(Evidence {
                title: member.rel_path.clone(),
                snippet: snippet_of(&content),
                source_id: member.rel_path.clone(),
                similarity: 0.0,
            });
            ws_matches.push(member.rel_path.clone());
        }
    }

    if !ws_matches.is_empty() {
        {
            let mut ws = WorkingSet::new(&config.working_set, storage);
            for rel_path in &ws_matches {
                ws.record_access(rel_path, now, SOURCE_WORKING_SET)?;
            }
        }
        return Ok(CascadeOutcome {
            verdict: Verdict::Sufficient,
            reason: format!(
                "working-set hit ({} item(s)); no similarity score at this stage",
                ws_matches.len()
            ),
            scores: Vec::new(),
            hit_count: ws_matches.len(),
            evidence: ws_evidence,
            working_set_matches: ws_matches,
            offline: false,
        });
    }

    // ---- Stage 2: minky semantic search -------------------------------------
    let hits = match minky.search_semantic(query, &config.verdict).await {
        Ok(hits) => hits,
        // ---- Stage 3a: offline degradation ----------------------------------
        Err(MinkyError::Unreachable(detail)) => {
            return Ok(CascadeOutcome {
                verdict: Verdict::Offline,
                reason: format!(
                    "long-term memory unreachable ({detail}); proceeding on the \
                     working set alone — long-term memory may be unanswered"
                ),
                scores: Vec::new(),
                hit_count: 0,
                evidence: Vec::new(),
                working_set_matches: Vec::new(),
                offline: true,
            });
        }
        // ---- Stage 3b: real HTTP error propagates ---------------------------
        Err(other) => return Err(CascadeError::Minky(other)),
    };

    let scores: Vec<f32> = hits.iter().map(|h| h.similarity).collect();
    let hit_count = hits.len();
    let top1 = scores.first().copied().unwrap_or(0.0);

    let evidence: Vec<Evidence> = hits
        .iter()
        .map(|h| Evidence {
            title: h
                .document_title
                .clone()
                .unwrap_or_else(|| h.document_id.clone()),
            snippet: h.chunk_text.clone().unwrap_or_default(),
            source_id: h.document_id.clone(),
            similarity: h.similarity,
        })
        .collect();

    // Recall: any minky hit that corresponds to a local memory file re-enters
    // the working set (external documents stay evidence-only).
    let local_paths: Vec<String> = snapshot
        .members
        .iter()
        .chain(snapshot.demoted.iter())
        .map(|i| i.rel_path.clone())
        .collect();
    let mut recall_matches = Vec::new();
    for hit in &hits {
        if let Some(title) = &hit.document_title {
            if let Some(rel_path) = match_local(title, &local_paths) {
                if !recall_matches.contains(&rel_path) {
                    recall_matches.push(rel_path);
                }
            }
        }
    }
    if !recall_matches.is_empty() {
        let mut ws = WorkingSet::new(&config.working_set, storage);
        for rel_path in &recall_matches {
            ws.record_access(rel_path, now, SOURCE_RECALL)?;
        }
    }

    // Similarity is inherently f32; compare thresholds in f32 space so a
    // configured `0.65` boundary matches a `0.65` similarity exactly (an f64
    // comparison would spuriously reject the round-tripped f32 value).
    let min_score = config.verdict.min_score as f32;
    let sufficient_top1 = config.verdict.sufficient_top1 as f32;
    let min_results = config.verdict.min_results as usize;

    let (verdict, reason) = if hit_count == 0 || top1 < min_score {
        (
            Verdict::AskUser,
            format!(
                "top-1 similarity {top1:.4} below min_score {min_score} \
                 (or no results): ask the user"
            ),
        )
    } else if hit_count >= min_results && top1 >= sufficient_top1 {
        (
            Verdict::Sufficient,
            format!(
                "top-1 {top1:.4} >= sufficient_top1 {sufficient_top1} with \
                 {hit_count} hit(s) >= min_results {min_results}"
            ),
        )
    } else {
        (
            Verdict::Insufficient,
            format!(
                "evidence returned but not confident: top-1 {top1:.4} in \
                 [{min_score}, {sufficient_top1}) or hits {hit_count} < \
                 min_results {min_results}"
            ),
        )
    };

    Ok(CascadeOutcome {
        verdict,
        reason,
        scores,
        hit_count,
        evidence,
        working_set_matches: recall_matches,
        offline: false,
    })
}

/// Split a query into lowercase alphanumeric tokens (length >= 2).
fn tokenize(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Read up to [`CONTENT_SCAN_BYTES`] of a file, lossily decoded as UTF-8.
fn read_head(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => {
            let n = bytes.len().min(CONTENT_SCAN_BYTES);
            String::from_utf8_lossy(&bytes[..n]).into_owned()
        }
        Err(_) => String::new(),
    }
}

/// A short single-line snippet for evidence display.
fn snippet_of(content: &str) -> String {
    content
        .trim()
        .chars()
        .take(200)
        .collect::<String>()
        .replace('\n', " ")
}

/// Return the local rel_path a minky `document_title` corresponds to, if any:
/// an exact rel_path match, or a case-insensitive file-stem match.
fn match_local(title: &str, local_paths: &[String]) -> Option<String> {
    let title_lc = title.to_lowercase();
    let title_stem = file_stem_lc(&title_lc);
    for rel in local_paths {
        let rel_lc = rel.to_lowercase();
        if rel_lc == title_lc {
            return Some(rel.clone());
        }
        if !title_stem.is_empty() && file_stem_lc(&rel_lc) == title_stem {
            return Some(rel.clone());
        }
    }
    None
}

/// Lowercase file stem of a `/`-separated path: last path segment without its
/// final extension.
fn file_stem_lc(path_lc: &str) -> String {
    let name = path_lc.rsplit('/').next().unwrap_or(path_lc);
    match name.rsplit_once('.') {
        Some((stem, _ext)) if !stem.is_empty() => stem.to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrokerConfig;
    use crate::minky_client::MinkyClient;
    use crate::storage::Storage;
    use std::io::Write;
    use std::net::TcpListener;
    use tempfile::TempDir;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Evaluation time = real "now" so freshly-written temp files fall inside
    /// the working-set window (a far-future constant would demote them all).
    fn now_ts() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    struct Harness {
        _memory: TempDir,
        _keyfile: TempDir,
        config: BrokerConfig,
        storage: Storage,
    }

    fn write_key(dir: &TempDir) -> std::path::PathBuf {
        let p = dir.path().join(".minky-key");
        let mut f = std::fs::File::create(&p).unwrap();
        // Trailing newline verifies the trim in MinkyClient::new.
        writeln!(f, "test-key").unwrap();
        p
    }

    /// Build a harness whose memory root contains `files` (rel_path -> content).
    fn harness(base_url: &str, files: &[(&str, &str)]) -> Harness {
        let memory = TempDir::new().unwrap();
        for (rel, content) in files {
            let full = memory.path().join(rel);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full, content).unwrap();
        }
        let keydir = TempDir::new().unwrap();
        let key_path = write_key(&keydir);

        let mut config = BrokerConfig::default();
        config.working_set.memory_root = memory.path().to_path_buf();
        // No pins by default so tests control membership precisely.
        config.working_set.always_include = Vec::new();
        config.minky.base_url = base_url.to_string();
        config.minky.api_key_file = key_path;
        config.minky.timeout_secs = 2;

        let storage = Storage::open_in_memory().unwrap();
        Harness {
            _memory: memory,
            _keyfile: keydir,
            config,
            storage,
        }
    }

    fn client(config: &BrokerConfig) -> MinkyClient {
        MinkyClient::new(&config.minky).unwrap()
    }

    /// `(similarity, document_title)` pairs -> a minky semantic envelope.
    fn envelope(results: &[(f32, &str)]) -> serde_json::Value {
        let arr: Vec<_> = results
            .iter()
            .enumerate()
            .map(|(i, (sim, title))| {
                serde_json::json!({
                    "document_id": format!("00000000-0000-0000-0000-{:012}", i),
                    "chunk_id": null,
                    "chunk_text": format!("snippet for {title}"),
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

    async fn mount_semantic(server: &MockServer, results: &[(f32, &str)]) {
        Mock::given(method("POST"))
            .and(path("/api/search/semantic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope(results)))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn sufficient_two_hits() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.8, "doc-a"), (0.7, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Sufficient);
        assert_eq!(out.hit_count, 2);
        assert_eq!(out.scores, vec![0.8, 0.7]);
        assert_eq!(out.evidence.len(), 2);
        assert!(!out.offline);
    }

    #[tokio::test]
    async fn insufficient_single_hit() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.6, "doc-a")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Insufficient);
        assert_eq!(out.hit_count, 1);
    }

    #[tokio::test]
    async fn ask_user_no_results() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::AskUser);
        assert_eq!(out.hit_count, 0);
    }

    #[tokio::test]
    async fn ask_user_low_top1() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.4, "doc-a"), (0.3, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::AskUser);
    }

    #[tokio::test]
    async fn boundary_top1_equals_sufficient_is_sufficient() {
        // top1 == sufficient_top1 (0.65) with min_results (2) hits -> sufficient.
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.65, "doc-a"), (0.65, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Sufficient);
    }

    #[tokio::test]
    async fn boundary_top1_equals_min_score_is_insufficient() {
        // top1 == min_score (0.55): not ask-user, below sufficient -> insufficient.
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.55, "doc-a"), (0.55, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Insufficient);
    }

    #[tokio::test]
    async fn boundary_top1_just_below_min_score_is_ask_user() {
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.5499, "doc-a"), (0.5, "doc-b")]).await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::AskUser);
    }

    #[tokio::test]
    async fn recall_reenters_local_document() {
        // Local file whose name does NOT contain the query tokens (so the
        // working set misses), but whose stem matches a minky hit title.
        let server = MockServer::start().await;
        mount_semantic(&server, &[(0.8, "rust.md"), (0.7, "external-doc")]).await;
        let mut h = harness(&server.uri(), &[("rust.md", "some notes about ownership")]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Sufficient);
        assert_eq!(out.working_set_matches, vec!["rust.md".to_string()]);
        // Recall wrote an access-log row and re-entered the working set.
        assert_eq!(h.storage.access_log_count("rust.md").unwrap(), 1);
        let item = h.storage.get_item("rust.md").unwrap().unwrap();
        assert!(item.in_working_set);
    }

    #[tokio::test]
    async fn sends_api_key_header() {
        let server = MockServer::start().await;
        // Mock only matches when X-API-Key == trimmed key; otherwise 404 -> Http err.
        Mock::given(method("POST"))
            .and(path("/api/search/semantic"))
            .and(header("X-API-Key", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "d")])))
            .expect(1)
            .mount(&server)
            .await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();
        // Reached minky and got a verdict -> the header matched.
        assert!(!out.offline);
    }

    #[tokio::test]
    async fn offline_when_endpoint_closed() {
        // Bind then drop a listener to obtain a guaranteed-closed port.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let base = format!("http://127.0.0.1:{port}");
        let mut h = harness(&base, &[]);
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Offline);
        assert!(out.offline);
    }

    #[tokio::test]
    async fn working_set_hit_short_circuits_without_calling_minky() {
        let server = MockServer::start().await;
        // expect(0): the cascade must not touch minky when the working set hits.
        Mock::given(method("POST"))
            .and(path("/api/search/semantic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "d")])))
            .expect(0)
            .mount(&server)
            .await;

        // File content contains the query token "kubernetes".
        let mut h = harness(
            &server.uri(),
            &[("ops.md", "notes on kubernetes clusters")],
        );
        let minky = client(&h.config);

        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap();

        assert_eq!(out.verdict, Verdict::Sufficient);
        assert!(out.scores.is_empty());
        assert_eq!(out.working_set_matches, vec!["ops.md".to_string()]);
        assert_eq!(h.storage.access_log_count("ops.md").unwrap(), 1);
        // MockServer verifies expect(0) on drop.
    }

    #[tokio::test]
    async fn http_error_propagates() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/search/semantic"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let mut h = harness(&server.uri(), &[]);
        let minky = client(&h.config);

        let err = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
            .await
            .unwrap_err();
        assert!(matches!(err, CascadeError::Minky(MinkyError::Http(500))));
    }
}
