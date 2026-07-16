//! Ultragoal G004 (WP3) retrieval-cascade red-team QA.
//!
//! Adversarial, contract-derived integration tests against the *public* broker
//! surface only (`broker::{run, Verdict, CascadeError, MinkyClient, MinkyError,
//! BrokerConfig, Storage}`). The intent is to break the approved WP3 contract,
//! not to confirm the happy path:
//!
//!   1. malicious minky responses (broken JSON / non-finite similarity) are
//!      handled without panic;
//!   2. verdict boundary transitions are exact (config-driven thresholds:
//!      sufficient_top1=0.65, min_score=0.55, min_results=2);
//!   3. connection refused / read timeout maps to `Verdict::Offline`;
//!   4. the long-lived M2M key is sent in the `X-API-Key` header;
//!   5. a working-set hit short-circuits and never touches minky;
//!   6. the same `Storage` survives sequential cascade runs.

use broker::{run, BrokerConfig, CascadeError, MinkyClient, MinkyError, Storage, Verdict};
use std::io::Write;
use std::net::TcpListener;
use std::time::Duration;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Harness (public-API only)
// ---------------------------------------------------------------------------

/// Real "now" so freshly-written temp files fall inside the working-set window.
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

fn write_key(dir: &TempDir, contents: &str) -> std::path::PathBuf {
    let p = dir.path().join(".minky-key");
    let mut f = std::fs::File::create(&p).unwrap();
    // Trailing newline verifies the trim in MinkyClient::new.
    writeln!(f, "{contents}").unwrap();
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
    let key_path = write_key(&keydir, "test-key");

    let mut config = BrokerConfig::default();
    config.working_set.memory_root = memory.path().to_path_buf();
    // No pins by default so tests control membership precisely.
    config.working_set.always_include = Vec::new();
    config.minky.base_url = base_url.to_string();
    config.minky.api_key_file = key_path;
    config.minky.timeout_secs = 1;

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
fn envelope(results: &[(f64, &str)]) -> serde_json::Value {
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

async fn mount_semantic(server: &MockServer, results: &[(f64, &str)]) {
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(envelope(results)))
        .mount(server)
        .await;
}

const Q: &str = "kubernetes deployment strategy";

// ---------------------------------------------------------------------------
// 1. Malicious minky responses — safe handling, never panic
// ---------------------------------------------------------------------------

/// M-01: a 200 with a syntactically broken JSON body must surface as a decode
/// error (Result::Err), never a panic or hang.
#[tokio::test]
async fn m01_broken_json_body_is_decode_error_not_panic() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string("{ this is not ]json[ at all"),
        )
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let err = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap_err();
    assert!(
        matches!(err, CascadeError::Minky(MinkyError::Decode(_))),
        "broken JSON must decode-fail, got {err:?}"
    );
}

/// M-02: a non-standard `NaN` similarity literal is invalid JSON and must be
/// rejected at decode time — NaN can never enter the verdict math.
#[tokio::test]
async fn m02_nan_similarity_literal_rejected_at_decode() {
    let server = MockServer::start().await;
    // Raw body: `NaN` is not valid JSON, serde_json must refuse it.
    let body = r#"{"success":true,"data":{"results":[{"document_id":"d","similarity":NaN}],"total":1,"query":"q"}}"#;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(body),
        )
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let err = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap_err();
    assert!(
        matches!(err, CascadeError::Minky(MinkyError::Decode(_))),
        "NaN literal must decode-fail, got {err:?}"
    );
}

/// M-03: an extreme finite similarity (`1e39` overflows f32 to +inf on decode)
/// must not panic; the verdict math still resolves deterministically.
#[tokio::test]
async fn m03_overflowing_similarity_does_not_panic() {
    let server = MockServer::start().await;
    mount_semantic(&server, &[(1e39, "doc-a"), (1e39, "doc-b")]).await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    // Must return Ok, never panic. +inf >= sufficient_top1 with hit_count=2 >=
    // min_results is deterministically Sufficient — pin the exact verdict
    // (L2 fix, 2026-07-11 Fable Redteam review) instead of accepting any of
    // the three variants, which could not detect a threshold sign-error
    // regression.
    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.hit_count, 2);
    assert_eq!(
        out.verdict,
        Verdict::Sufficient,
        "+inf top1 with 2 hits >= min_results must be Sufficient, got {:?}",
        out.verdict
    );
    assert!(out.scores.iter().all(|s| s.is_infinite()));
}

// ---------------------------------------------------------------------------
// 2. Verdict boundary transitions (config-driven thresholds)
// ---------------------------------------------------------------------------

/// V-01: top1 == sufficient_top1 (0.65) with min_results hits -> Sufficient.
#[tokio::test]
async fn v01_top1_equals_sufficient_is_sufficient() {
    let server = MockServer::start().await;
    mount_semantic(&server, &[(0.65, "a"), (0.65, "b")]).await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Sufficient);
}

/// V-02: top1 == min_score (0.55): not ask-user, below sufficient -> Insufficient.
#[tokio::test]
async fn v02_top1_equals_min_score_is_insufficient() {
    let server = MockServer::start().await;
    mount_semantic(&server, &[(0.55, "a"), (0.55, "b")]).await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Insufficient);
}

/// V-03: top1 == 0.549 (just under min_score) -> AskUser.
#[tokio::test]
async fn v03_top1_just_below_min_score_is_ask_user() {
    let server = MockServer::start().await;
    mount_semantic(&server, &[(0.549, "a"), (0.5, "b")]).await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::AskUser);
}

/// V-04: enough score but only one hit (< min_results 2) -> Insufficient, not
/// Sufficient. Guards the AND in the sufficiency rule.
#[tokio::test]
async fn v04_high_top1_single_hit_is_insufficient() {
    let server = MockServer::start().await;
    mount_semantic(&server, &[(0.99, "a")]).await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Insufficient);
    assert_eq!(out.hit_count, 1);
}

// ---------------------------------------------------------------------------
// 3. Degradation — Unreachable / 5xx -> Offline; 4xx still a hard Err
// ---------------------------------------------------------------------------

/// D-01: a guaranteed-closed port (bind then drop) -> connection refused ->
/// Verdict::Offline (not an Err).
#[tokio::test]
async fn d01_closed_port_maps_to_offline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let base = format!("http://127.0.0.1:{port}");
    let mut h = harness(&base, &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Offline);
    assert!(out.offline);
    assert_eq!(out.hit_count, 0);
    assert!(out.evidence.is_empty());
}

/// D-02: a response delayed past the configured timeout (1s) triggers the read
/// timeout, which is treated as Unreachable -> Offline.
#[tokio::test]
async fn d02_response_delay_past_timeout_maps_to_offline() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(envelope(&[(0.9, "a"), (0.9, "b")]))
                .set_delay(Duration::from_secs(3)),
        )
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config); // timeout_secs = 1

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Offline);
    assert!(out.offline);
}

/// D-03: a 5xx status means the backend itself is broken (crash, dependency
/// outage, overload) — it degrades to Offline exactly like Unreachable,
/// rather than taking the whole cascade down with a hard Err.
#[tokio::test]
async fn d03_http_500_degrades_to_offline() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Offline);
    assert!(out.offline);
    assert!(out.reason.contains("500"));
}

/// D-03b: a 502 (bad gateway) is also a server-side failure and degrades the
/// same way as 500.
#[tokio::test]
async fn d03b_http_502_degrades_to_offline() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Offline);
    assert!(out.offline);
    assert!(out.reason.contains("502"));
}

/// D-03c: a 4xx status (e.g. bad/expired API key) is a genuine client-side
/// error and must still propagate as Err — it must not be swallowed into a
/// quiet Offline verdict, since that would hide auth/config misconfiguration.
#[tokio::test]
async fn d03c_http_401_still_propagates_as_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let err = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        CascadeError::Minky(MinkyError::Http(401))
    ));
}

// ---------------------------------------------------------------------------
// 4. Auth — X-API-Key header transmission
// ---------------------------------------------------------------------------

/// A-01: the mock only answers when X-API-Key == trimmed key AND is hit exactly
/// once. A verdict returning proves the header was sent correctly.
#[tokio::test]
async fn a01_sends_trimmed_api_key_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .and(header("X-API-Key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "a"), (0.8, "b")])))
        .expect(1)
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config);

    let out = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert!(!out.offline);
    assert_eq!(out.verdict, Verdict::Sufficient);
    // MockServer verifies expect(1) on drop.
}

/// A-02: a wrong key must NOT match the header matcher -> mock 404 -> Http err.
/// Proves the matcher is real and the key is actually being compared.
#[tokio::test]
async fn a02_wrong_key_fails_header_match() {
    let server = MockServer::start().await;
    // Only a request with the *wrong* key would fall through to this default;
    // the correct-key matcher is intentionally absent so any request 404s.
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .and(header("X-API-Key", "some-other-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "a")])))
        .mount(&server)
        .await;
    let mut h = harness(&server.uri(), &[]);
    let minky = client(&h.config); // sends "test-key", never matches -> 404

    let err = run(Q, &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap_err();
    assert!(
        matches!(err, CascadeError::Minky(MinkyError::Http(404))),
        "unmatched key should 404, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// 5. Working-set short-circuit — minky must NOT be called
// ---------------------------------------------------------------------------

/// W-01: a working-set hit short-circuits to Sufficient and the minky endpoint
/// is never touched (expect(0) verified on drop).
#[tokio::test]
async fn w01_working_set_hit_skips_minky() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "d")])))
        .expect(0)
        .mount(&server)
        .await;

    let mut h = harness(&server.uri(), &[("ops.md", "notes on kubernetes clusters")]);
    let minky = client(&h.config);

    let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();
    assert_eq!(out.verdict, Verdict::Sufficient);
    assert!(out.scores.is_empty(), "no similarity score at working-set stage");
    assert_eq!(out.working_set_matches, vec!["ops.md".to_string()]);
    assert_eq!(h.storage.access_log_count("ops.md").unwrap(), 1);
    // MockServer verifies expect(0) on drop.
}

// ---------------------------------------------------------------------------
// 6. Sequential runs on a shared Storage
// ---------------------------------------------------------------------------

/// C-01: two sequential cascade runs against the same Storage both succeed and
/// the access clock/log accumulate correctly (no lock/state corruption).
#[tokio::test]
async fn c01_sequential_runs_share_storage_safely() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/search/semantic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(envelope(&[(0.9, "d")])))
        .expect(0)
        .mount(&server)
        .await;

    let mut h = harness(&server.uri(), &[("ops.md", "notes on kubernetes clusters")]);
    let minky = client(&h.config);

    let t1 = now_ts();
    let out1 = run("kubernetes deploy", &h.config, &mut h.storage, &minky, t1)
        .await
        .unwrap();
    assert_eq!(out1.verdict, Verdict::Sufficient);

    let t2 = t1 + 10;
    let out2 = run("kubernetes deploy", &h.config, &mut h.storage, &minky, t2)
        .await
        .unwrap();
    assert_eq!(out2.verdict, Verdict::Sufficient);

    // Two runs -> two access-log rows; clock advanced to the later ts.
    assert_eq!(h.storage.access_log_count("ops.md").unwrap(), 2);
    let item = h.storage.get_item("ops.md").unwrap().unwrap();
    assert_eq!(item.last_accessed, Some(t2));
    assert!(item.in_working_set);
}
