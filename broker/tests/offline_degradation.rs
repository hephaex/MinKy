//! Ultragoal G005 (WP4) offline graceful-degradation integration tests.
//!
//! Approved contract under test (pending-approval.md WP4):
//!   "timeout/connect 실패 감지 추가: 미도달 시 작업집합만으로 진행 +
//!    verdict.reason=offline + 사용자 고지 메시지 포함. 검증: 미도달 URL 강제 통합 테스트"
//!
//! These exercise the *public* cascade surface only (`broker::run`,
//! `broker::Verdict`, `broker::config::BrokerConfig`, `broker::storage::Storage`,
//! `broker::minky_client::MinkyClient`) against genuinely unreachable endpoints,
//! plus one recovery round-trip through a live wiremock server. The intent is to
//! prove the offline-degradation path end to end, not to inspect internals.

use std::io::Write;
use std::net::TcpListener;
use std::time::{Duration, Instant};

use broker::config::BrokerConfig;
use broker::minky_client::MinkyClient;
use broker::storage::Storage;
use broker::{run, Verdict};

use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Real "now" so freshly-written temp files fall inside the working-set window
/// (a far-future constant would demote every file).
fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Owns the temp dirs so they outlive the config/storage for a whole test.
struct Env {
    _memory: TempDir,
    _keyfile: TempDir,
    config: BrokerConfig,
    storage: Storage,
}

/// Build an environment whose memory root contains `files` (rel_path -> content),
/// pointed at `base_url`, with the given connect/read `timeout_secs`.
fn env(base_url: &str, timeout_secs: u64, files: &[(&str, &str)]) -> Env {
    let memory = TempDir::new().unwrap();
    for (rel, content) in files {
        let full = memory.path().join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&full, content).unwrap();
    }

    let keydir = TempDir::new().unwrap();
    let key_path = keydir.path().join(".minky-key");
    let mut f = std::fs::File::create(&key_path).unwrap();
    writeln!(f, "test-key").unwrap();

    let mut config = BrokerConfig::default();
    config.working_set.memory_root = memory.path().to_path_buf();
    // No pins so each test controls working-set membership precisely.
    config.working_set.always_include = Vec::new();
    config.minky.base_url = base_url.to_string();
    config.minky.api_key_file = key_path;
    config.minky.timeout_secs = timeout_secs;

    Env {
        _memory: memory,
        _keyfile: keydir,
        config,
        storage: Storage::open_in_memory().unwrap(),
    }
}

/// A guaranteed-closed loopback port: bind then immediately drop the listener.
/// Connecting here yields a prompt `connection refused`.
fn closed_loopback_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// `(similarity, document_title)` pairs -> a minky semantic-search envelope.
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

// ---------------------------------------------------------------------------
// 1. connect-refused: a bound-then-dropped loopback port is immediately closed.
// ---------------------------------------------------------------------------

/// O-01: a guaranteed-closed port maps to `Verdict::Offline`, sets
/// `offline == true`, and the user-facing reason literally contains the
/// "proceeding on the working set alone" notice.
#[tokio::test]
async fn o01_connect_refused_maps_to_offline_with_notice() {
    let mut e = env(&closed_loopback_url(), 2, &[]);
    let minky = MinkyClient::new(&e.config.minky).unwrap();

    let out = run("kubernetes deploy", &e.config, &mut e.storage, &minky, now_ts())
        .await
        .expect("offline degradation must be Ok, never Err");

    assert_eq!(out.verdict, Verdict::Offline, "closed port must degrade to Offline");
    assert!(out.offline, "offline flag must be set");
    assert!(
        out.reason.contains("proceeding on the working set alone"),
        "reason must carry the working-set notice, got: {:?}",
        out.reason
    );
    // No evidence and no working-set matches when nothing matched locally.
    assert_eq!(out.hit_count, 0);
    assert!(out.evidence.is_empty());
    assert!(out.working_set_matches.is_empty());
}

// ---------------------------------------------------------------------------
// 2. Unreachable address + short timeout: the connect stage must terminate
//    the whole cascade in a bounded time and still map to Offline.
// ---------------------------------------------------------------------------

/// O-02: RFC 5737 TEST-NET-1 (`192.0.2.1`) is a documented non-routable address.
/// With a 1s connect/read timeout the cascade must return `Offline` and, most
/// importantly, must not hang: it terminates well within a small multiple of the
/// configured timeout (whether via a genuine connect timeout or a prompt
/// "no route to host" — both are the Unreachable trigger).
#[tokio::test]
async fn o02_unreachable_address_bounded_by_timeout() {
    let mut e = env("http://192.0.2.1:8001", 1, &[]);
    let minky = MinkyClient::new(&e.config.minky).unwrap();

    let start = Instant::now();
    let out = run("kubernetes deploy", &e.config, &mut e.storage, &minky, now_ts())
        .await
        .expect("unreachable address must degrade to Ok(Offline)");
    let elapsed = start.elapsed();

    assert_eq!(out.verdict, Verdict::Offline, "unreachable IP must degrade to Offline");
    assert!(out.offline);
    // Bounded termination: a 1s timeout must not let the run hang. Generous
    // upper bound (4s) covers scheduler/CI jitter while still catching a hang.
    assert!(
        elapsed < Duration::from_secs(4),
        "cascade must terminate promptly under a 1s timeout; took {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Offline + working set: a local hit short-circuits before minky is ever
//    contacted (so the closed endpoint is irrelevant); a miss degrades.
// ---------------------------------------------------------------------------

/// O-03a: with a matching working-set item the cascade is `Sufficient` and never
/// touches the (closed) minky endpoint — proven by the fact that a guaranteed
/// unreachable base_url does not produce Offline here.
#[tokio::test]
async fn o03a_working_set_hit_short_circuits_even_when_offline() {
    let mut e = env(
        &closed_loopback_url(),
        2,
        &[("ops.md", "notes on kubernetes clusters")],
    );
    let minky = MinkyClient::new(&e.config.minky).unwrap();

    let out = run("kubernetes deploy", &e.config, &mut e.storage, &minky, now_ts())
        .await
        .unwrap();

    assert_eq!(
        out.verdict,
        Verdict::Sufficient,
        "a working-set hit must short-circuit before the offline path"
    );
    assert!(!out.offline, "a short-circuited run is not offline");
    assert_eq!(out.working_set_matches, vec!["ops.md".to_string()]);
    // The minky endpoint was never contacted (it is closed), yet no offline.
    assert!(out.scores.is_empty());
}

/// O-03b: same closed endpoint but the working set has no matching item -> the
/// cascade reaches the unreachable backend and degrades to Offline.
#[tokio::test]
async fn o03b_working_set_miss_degrades_to_offline() {
    let mut e = env(
        &closed_loopback_url(),
        2,
        &[("unrelated.md", "grocery list and weekend plans")],
    );
    let minky = MinkyClient::new(&e.config.minky).unwrap();

    let out = run("kubernetes deploy", &e.config, &mut e.storage, &minky, now_ts())
        .await
        .unwrap();

    assert_eq!(out.verdict, Verdict::Offline);
    assert!(out.offline);
    assert!(out.working_set_matches.is_empty());
}

// ---------------------------------------------------------------------------
// 4. Recovery: an offline run must not contaminate a subsequent healthy run
//    over the same storage.
// ---------------------------------------------------------------------------

/// O-04: first hit a closed endpoint (-> Offline), then repoint base_url at a
/// live wiremock server and run again over the *same* storage. The second run
/// must recover cleanly to a normal verdict (`Sufficient` here), proving the
/// offline state is per-call and never sticky.
#[tokio::test]
async fn o04_recovery_after_offline_is_not_sticky() {
    // Phase 1: closed endpoint -> Offline.
    let mut e = env(&closed_loopback_url(), 2, &[]);
    let offline_client = MinkyClient::new(&e.config.minky).unwrap();
    let first = run("kubernetes deploy", &e.config, &mut e.storage, &offline_client, now_ts())
        .await
        .unwrap();
    assert_eq!(first.verdict, Verdict::Offline, "phase 1 must be Offline");
    assert!(first.offline);

    // Phase 2: swap in a live server and rebuild the client from the new config.
    let server = MockServer::start().await;
    mount_semantic(&server, &[(0.8, "doc-a"), (0.7, "doc-b")]).await;
    e.config.minky.base_url = server.uri();
    let online_client = MinkyClient::new(&e.config.minky).unwrap();

    let second = run("kubernetes deploy", &e.config, &mut e.storage, &online_client, now_ts())
        .await
        .unwrap();

    assert_eq!(
        second.verdict,
        Verdict::Sufficient,
        "recovery run must reach the live backend and produce a normal verdict"
    );
    assert!(!second.offline, "recovered run must clear the offline flag");
    assert_eq!(second.hit_count, 2);
    assert_eq!(second.scores, vec![0.8, 0.7]);
}

/// O-04b: recovery to an empty result set also returns a clean, non-offline
/// verdict (`AskUser`) — the offline flag does not linger as a false positive.
#[tokio::test]
async fn o04b_recovery_to_ask_user_is_not_offline() {
    let mut e = env(&closed_loopback_url(), 2, &[]);
    let offline_client = MinkyClient::new(&e.config.minky).unwrap();
    let first = run("kubernetes deploy", &e.config, &mut e.storage, &offline_client, now_ts())
        .await
        .unwrap();
    assert_eq!(first.verdict, Verdict::Offline);

    let server = MockServer::start().await;
    mount_semantic(&server, &[]).await;
    e.config.minky.base_url = server.uri();
    let online_client = MinkyClient::new(&e.config.minky).unwrap();

    let second = run("kubernetes deploy", &e.config, &mut e.storage, &online_client, now_ts())
        .await
        .unwrap();

    assert_eq!(second.verdict, Verdict::AskUser);
    assert!(!second.offline);
    assert_eq!(second.hit_count, 0);
}

// ---------------------------------------------------------------------------
// 5. Notice completeness: the Offline reason must be a genuine, informative
//    user notice (non-empty, mentions both the fault and the fallback).
// ---------------------------------------------------------------------------

/// O-05: the Offline verdict's `reason` is non-empty and contains BOTH
/// "unreachable" (what went wrong) and "working set" (what the broker does
/// about it), so the caller can explain the situation to the user.
#[tokio::test]
async fn o05_offline_reason_is_a_complete_user_notice() {
    let mut e = env(&closed_loopback_url(), 2, &[]);
    let minky = MinkyClient::new(&e.config.minky).unwrap();

    let out = run("kubernetes deploy", &e.config, &mut e.storage, &minky, now_ts())
        .await
        .unwrap();

    assert_eq!(out.verdict, Verdict::Offline);
    assert!(!out.reason.trim().is_empty(), "offline reason must not be empty");
    assert!(
        out.reason.contains("unreachable"),
        "reason must name the fault (unreachable), got: {:?}",
        out.reason
    );
    assert!(
        out.reason.contains("working set"),
        "reason must name the fallback (working set), got: {:?}",
        out.reason
    );
}
