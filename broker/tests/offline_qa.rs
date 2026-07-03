//! Ultragoal G005 (WP4) offline-degradation red-team QA.
//!
//! Adversarial, contract-derived tests whose intent is to *break* the approved
//! offline-degradation contract (`broker/src/cascade.rs` stage 3 +
//! `broker/src/minky_client.rs` failure taxonomy), not to confirm the happy
//! path. Everything exercises the public API only
//! (`broker::{run, Verdict}`, `broker::config::*`,
//! `broker::minky_client::MinkyClient`, `broker::storage::Storage`).
//!
//! Contract under test (approved plan WP3/WP4):
//! - A connection failure/timeout (`MinkyError::Unreachable`) maps to
//!   `Verdict::Offline` with `offline == true`.
//! - The offline outcome carries NO fabricated evidence: `scores` empty,
//!   `evidence` empty, `hit_count == 0`, `working_set_matches` empty.
//! - The cascade never panics on hostile transport conditions (tiny timeout,
//!   closed port, blackholed address, malformed base_url).
//! - Repeated runs are stateless: the same unreachable endpoint yields the
//!   same Offline verdict every time, with no state pollution.

use broker::config::BrokerConfig;
use broker::minky_client::MinkyClient;
use broker::storage::Storage;
use broker::{run, Verdict};

use std::net::TcpListener;
use tempfile::TempDir;

/// Evaluation time = real "now" so the (empty) working set is windowed sanely.
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

/// Build a harness with an EMPTY memory root and no pins, so stage 1 (working
/// set) always misses and the cascade proceeds to the minky/offline path.
/// `timeout_secs` is caller-controlled to exercise the tiny-timeout attack.
fn harness(base_url: &str, timeout_secs: u64) -> Harness {
    let memory = TempDir::new().unwrap();
    let keydir = TempDir::new().unwrap();
    let key_path = keydir.path().join(".minky-key");
    std::fs::write(&key_path, "test-key\n").unwrap();

    let mut config = BrokerConfig::default();
    config.working_set.memory_root = memory.path().to_path_buf();
    config.working_set.always_include = Vec::new();
    config.minky.base_url = base_url.to_string();
    config.minky.api_key_file = key_path;
    config.minky.timeout_secs = timeout_secs;

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

/// Bind then drop a listener to obtain a guaranteed-closed local port
/// (connection is refused immediately, regardless of timeout).
fn closed_port_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// Assert an outcome is the *honest* offline verdict: Offline, flagged offline,
/// and carrying zero fabricated evidence of any kind.
fn assert_honest_offline(out: &broker::CascadeOutcome) {
    assert_eq!(out.verdict, Verdict::Offline, "verdict must be Offline");
    assert!(out.offline, "offline flag must be set");
    assert!(out.scores.is_empty(), "offline must expose no scores");
    assert!(out.evidence.is_empty(), "offline must expose no evidence");
    assert_eq!(out.hit_count, 0, "offline hit_count must be 0");
    assert!(
        out.working_set_matches.is_empty(),
        "offline must expose no working-set matches"
    );
}

// ---------------------------------------------------------------------------
// Case 1: minimum-viable timeout (1s) still degrades cleanly, never panics.
// ---------------------------------------------------------------------------

/// O-01a: the smallest legal timeout (1s) against a closed port returns Offline
/// (connection refused is instant) and never panics.
#[tokio::test]
async fn o01a_min_timeout_closed_port_is_offline() {
    let mut h = harness(&closed_port_url(), 1);
    let minky = client(&h.config);

    let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
        .await
        .expect("offline degradation must not error");

    assert_honest_offline(&out);
}

/// O-01b: the smallest legal timeout (1s) against a blackholed, non-routable
/// TEST-NET-1 address (RFC 5737 `192.0.2.1`) actually exercises the *timeout*
/// branch (not just refused) and still degrades to Offline without panicking.
#[tokio::test]
async fn o01b_min_timeout_blackhole_is_offline() {
    // 192.0.2.1 is reserved for documentation and is not routable, so the
    // connect attempt hangs until the 1s connect_timeout fires -> Unreachable.
    let mut h = harness("http://192.0.2.1:8001", 1);
    let minky = client(&h.config);

    let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts())
        .await
        .expect("timeout must degrade, not error");

    assert_honest_offline(&out);
}

// ---------------------------------------------------------------------------
// Case 2: repeated runs against the same unreachable URL are consistent and
// leave no state pollution.
// ---------------------------------------------------------------------------

/// O-02: three sequential runs against one closed endpoint, reusing the same
/// storage, each return an identical honest-Offline outcome. No access-log
/// rows and no knowledge_item rows are written by the offline path (the
/// verdict must not fabricate recall/membership state).
#[tokio::test]
async fn o02_repeated_runs_consistent_no_state_pollution() {
    let mut h = harness(&closed_port_url(), 1);
    let minky = client(&h.config);
    let t = now_ts();

    for i in 0..3 {
        let out = run("kubernetes deploy", &h.config, &mut h.storage, &minky, t)
            .await
            .unwrap_or_else(|e| panic!("run #{i} must not error: {e:?}"));
        assert_honest_offline(&out);
    }

    // State pollution guard: offline degradation writes nothing to storage.
    assert!(
        h.storage.all_items().unwrap().is_empty(),
        "offline path must not create knowledge_item rows"
    );
    assert_eq!(
        h.storage.access_log_count("kubernetes deploy").unwrap(),
        0,
        "offline path must not write access-log rows"
    );
}

// ---------------------------------------------------------------------------
// Case 3: offline outcome fields are precisely empty/zero (no false evidence).
// ---------------------------------------------------------------------------

/// O-03: when offline == true, every evidence-bearing field is exactly its
/// empty/zero identity. This is asserted independently of the shared helper so
/// a regression that (e.g.) leaks a stale score is caught with an explicit,
/// field-by-field message.
#[tokio::test]
async fn o03_offline_carries_no_false_evidence() {
    let mut h = harness(&closed_port_url(), 1);
    let minky = client(&h.config);

    let out = run("anything at all", &h.config, &mut h.storage, &minky, now_ts())
        .await
        .unwrap();

    assert!(out.offline);
    assert_eq!(out.verdict, Verdict::Offline);
    assert_eq!(out.scores, Vec::<f32>::new(), "scores must be empty");
    assert_eq!(out.evidence.len(), 0, "evidence must be empty");
    assert_eq!(out.hit_count, 0, "hit_count must be 0");
    assert_eq!(
        out.working_set_matches,
        Vec::<String>::new(),
        "working_set_matches must be empty"
    );
    // The reason string must exist and describe the degradation (non-empty),
    // but must never claim confidence.
    assert!(!out.reason.is_empty(), "offline reason must be explained");
}

// ---------------------------------------------------------------------------
// Case 4: malformed base_url (empty / whitespace-only) never panics.
// ---------------------------------------------------------------------------

/// O-04a: an empty base_url yields an unusable relative endpoint. The client
/// builds, and `run` must degrade to Offline (send() fails to a transport
/// error) OR return a clean Err — but MUST NOT panic.
#[tokio::test]
async fn o04a_empty_base_url_no_panic() {
    let mut h = harness("", 1);
    let minky = client(&h.config);

    let result = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts()).await;

    match result {
        Ok(out) => assert_honest_offline(&out),
        Err(e) => {
            // A clean, explained error is acceptable; a panic is not.
            let _ = format!("{e:?}");
        }
    }
}

/// O-04b: a whitespace-only base_url ("   ") is likewise unusable. Same
/// contract: Offline or clean Err, never a panic.
#[tokio::test]
async fn o04b_whitespace_base_url_no_panic() {
    let mut h = harness("   ", 1);
    let minky = client(&h.config);

    let result = run("kubernetes deploy", &h.config, &mut h.storage, &minky, now_ts()).await;

    match result {
        Ok(out) => assert_honest_offline(&out),
        Err(e) => {
            let _ = format!("{e:?}");
        }
    }
}
