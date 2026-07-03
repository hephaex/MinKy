//! Ultragoal G001 (WP0) completion-gate red-team QA.
//!
//! These are adversarial, contract-derived tests: the intent is to *break* the
//! approved contract, not to confirm the happy path. Everything here exercises
//! the public API only (`broker::BrokerConfig`, `broker::Storage`).
//!
//! Approved contract under test (WP0 decision (a) + WP0 schema):
//! - Thresholds externalized in `broker.toml`; defaults
//!   `min_score=0.55`, `sufficient_top1=0.65`, `min_results=2`, `top_k=8`,
//!   working-set `window_days=30`, `always_include=[IDENTITY.md, SELF.md,
//!   episodes/_recent.md]` (3 pinned files).
//! - Validation: scores in `[0.0, 1.0]`, `sufficient_top1 >= min_score`,
//!   `min_results >= 1`, `top_k >= min_results`, `window_days >= 1`,
//!   `minky.base_url` non-empty.
//! - `broker.sqlite`: `knowledge_item(path PK, first_seen, last_accessed,
//!   in_working_set, always_include)` + `access_log`; migrations are
//!   dry-runnable and idempotent; the access clock never runs backward (MAX).

use broker::config::{BrokerConfig, ConfigError};
use broker::storage::Storage;

// ---------------------------------------------------------------------------
// 1. TOML boundary / adversarial parsing
// ---------------------------------------------------------------------------

/// A-01: score boundary values 0.0 and 1.0 are *inside* [0,1] and must be
/// accepted. min_score=1.0 requires sufficient_top1=1.0 to satisfy ordering.
#[test]
fn a01_score_bounds_zero_and_one_accepted() {
    let lo = BrokerConfig::from_toml_str("[verdict]\nmin_score = 0.0\n")
        .expect("min_score=0.0 must be accepted");
    assert_eq!(lo.verdict.min_score, 0.0);

    let hi = BrokerConfig::from_toml_str(
        "[verdict]\nmin_score = 1.0\nsufficient_top1 = 1.0\n",
    )
    .expect("min_score=1.0 with sufficient_top1=1.0 must be accepted");
    assert_eq!(hi.verdict.min_score, 1.0);
    assert_eq!(hi.verdict.sufficient_top1, 1.0);
}

/// A-02: 1.0000001 is strictly above the closed upper bound and must be
/// rejected as an Invalid (semantic) error, not silently clamped.
#[test]
fn a02_score_just_above_one_rejected() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmin_score = 1.0000001\n")
        .expect_err("min_score just above 1.0 must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-02b: sufficient_top1 just above 1.0 must also be rejected.
#[test]
fn a02b_sufficient_top1_above_one_rejected() {
    let err =
        BrokerConfig::from_toml_str("[verdict]\nsufficient_top1 = 1.0000001\n")
            .expect_err("sufficient_top1 above 1.0 must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-03: sufficient_top1 < min_score violates the cascade ordering invariant.
#[test]
fn a03_sufficient_top1_below_min_score_rejected() {
    let err = BrokerConfig::from_toml_str(
        "[verdict]\nmin_score = 0.8\nsufficient_top1 = 0.3\n",
    )
    .expect_err("sufficient_top1 < min_score must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-04: min_results = 0 makes "sufficient" reachable with zero hits.
#[test]
fn a04_min_results_zero_rejected() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmin_results = 0\n")
        .expect_err("min_results=0 must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-05: top_k < min_results can never satisfy the min_results requirement.
#[test]
fn a05_top_k_below_min_results_rejected() {
    let err = BrokerConfig::from_toml_str(
        "[verdict]\nmin_results = 5\ntop_k = 3\n",
    )
    .expect_err("top_k < min_results must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-06: negative float for a score. Negative parses as f64 but is out of
/// range -> Invalid (semantic rejection).
#[test]
fn a06_negative_score_rejected() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmin_score = -0.5\n")
        .expect_err("negative min_score must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

/// A-06b: negative integer for a u32 field must fail at parse/type layer.
#[test]
fn a06b_negative_integer_type_error() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmin_results = -1\n")
        .expect_err("negative min_results must be a type error");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}

/// A-07: string where a float is expected must fail at parse/type layer.
#[test]
fn a07_string_for_float_type_error() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmin_score = \"high\"\n")
        .expect_err("string for min_score must be a type error");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}

/// A-07b: string where an integer is expected must fail at parse/type layer.
#[test]
fn a07b_string_for_integer_type_error() {
    let err = BrokerConfig::from_toml_str("[verdict]\ntop_k = \"eight\"\n")
        .expect_err("string for top_k must be a type error");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}

/// A-08: unknown top-level section is rejected (deny_unknown_fields).
#[test]
fn a08_unknown_section_rejected() {
    let err = BrokerConfig::from_toml_str("[bogus_section]\nfoo = 1\n")
        .expect_err("unknown section must be rejected");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}

/// A-08b: unknown field inside a known section is rejected.
#[test]
fn a08b_unknown_field_rejected() {
    let err = BrokerConfig::from_toml_str("[verdict]\nmystery = 1\n")
        .expect_err("unknown field must be rejected");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}

/// A-09: empty minky.base_url would produce an unusable endpoint.
#[test]
fn a09_empty_base_url_rejected() {
    let err = BrokerConfig::from_toml_str("[minky]\nbase_url = \"\"\n")
        .expect_err("empty base_url must be rejected");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// 2. Broken TOML (syntax errors) — safe failure, never panic
// ---------------------------------------------------------------------------

/// A-10: an assortment of syntactically broken TOML must each return a Parse
/// error (Result::Err), never panic or hang.
#[test]
fn a10_broken_toml_fails_safely() {
    let broken_inputs = [
        "[verdict\nmin_score = 0.5\n",          // unterminated section header
        "[verdict]\nmin_score = \n",            // missing value
        "min_score = 0.5\n[verdict]",           // key before its table (bare key ok, but...)
        "[verdict]]]\n",                        // garbage brackets
        "= 0.5\n",                              // missing key
        "[verdict]\nmin_score 0.5\n",           // missing '='
        "\u{0}\u{1}\u{2}",                       // control-byte garbage
    ];
    for (i, raw) in broken_inputs.iter().enumerate() {
        let result = BrokerConfig::from_toml_str(raw);
        assert!(
            result.is_err(),
            "broken input #{i} should fail safely, got Ok: {raw:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. SQLite state store — migrations, integrity, clock, injection safety
// ---------------------------------------------------------------------------

/// S-01: reopening the on-disk DB is idempotent (no duplicate migrations,
/// no error, schema version stable at MIGRATIONS length via dry-run).
#[test]
fn s01_reopen_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broker.sqlite");

    let baseline = Storage::migration_dry_run().unwrap();
    assert!(baseline >= 1, "at least one migration must exist");

    for _ in 0..5 {
        let store = Storage::open(&path).expect("reopen must not error");
        // Dry-run resolves the fully-migrated version each time.
        assert_eq!(
            Storage::migration_dry_run().unwrap(),
            baseline,
            "schema version must be stable across reopens"
        );
        drop(store);
    }

    // Data survives reopen and no phantom items appear from re-migration.
    {
        let mut s = Storage::open(&path).unwrap();
        s.record_access("survivor.md", 42, "broker").unwrap();
    }
    let s = Storage::open(&path).unwrap();
    let item = s.get_item("survivor.md").unwrap().expect("row must survive");
    assert_eq!(item.last_accessed, Some(42));
    assert_eq!(s.all_items().unwrap().len(), 1, "no phantom rows");
}

/// S-02: 1000 accesses to one path -> exactly 1 knowledge_item row, 1000 log
/// rows, first_seen pinned to the first ts, last_accessed at the max ts.
#[test]
fn s02_bulk_access_consistency() {
    let mut store = Storage::open_in_memory().unwrap();
    let path = "episodes/hot.md";
    for ts in 1..=1000i64 {
        store.record_access(path, ts, "broker").unwrap();
    }
    assert_eq!(store.all_items().unwrap().len(), 1, "single item row");
    assert_eq!(store.access_log_count(path).unwrap(), 1000, "1000 log rows");

    let item = store.get_item(path).unwrap().unwrap();
    assert_eq!(item.first_seen, 1, "first_seen keeps the earliest ts");
    assert_eq!(item.last_accessed, Some(1000), "last_accessed is the max ts");
}

/// S-03: the access clock must never run backward, even under interleaved
/// out-of-order timestamps (adversarial ordering).
#[test]
fn s03_clock_never_runs_backward() {
    let mut store = Storage::open_in_memory().unwrap();
    let p = "a.md";
    // Deliberately hostile ordering: peak in the middle, then regress.
    let seq = [100, 500, 300, 900, 200, 899, 50];
    for ts in seq {
        store.record_access(p, ts, "broker").unwrap();
    }
    let item = store.get_item(p).unwrap().unwrap();
    assert_eq!(
        item.last_accessed,
        Some(900),
        "last_accessed must hold the maximum ts observed"
    );
    // Every observation is still logged (audit is append-only).
    assert_eq!(store.access_log_count(p).unwrap(), seq.len() as i64);
}

/// S-03b: even a single backward-then-forward pair keeps the max.
#[test]
fn s03b_backward_then_equal() {
    let mut store = Storage::open_in_memory().unwrap();
    store.record_access("x.md", 5_000, "broker").unwrap();
    store.record_access("x.md", 5_000, "broker").unwrap(); // equal ts
    store.record_access("x.md", 1, "broker").unwrap(); // backward
    let item = store.get_item("x.md").unwrap().unwrap();
    assert_eq!(item.last_accessed, Some(5_000));
}

/// S-04: hostile path strings — SQL injection payloads, unicode, embedded
/// quotes and control characters — are stored/retrieved verbatim and never
/// corrupt or drop the schema (parameterized queries).
#[test]
fn s04_hostile_paths_are_safe() {
    let mut store = Storage::open_in_memory().unwrap();
    let hostile = [
        "'; DROP TABLE knowledge_item; --",
        "'; DROP TABLE access_log; --",
        "path\" OR \"1\"=\"1",
        "1' OR '1'='1",
        "한글/에피소드_기록.md",
        "emoji/🔥🧠/notes.md",
        "with\ttab\nand\nnewline.md",
        "quotes'\"`.md",
        "back\\slash\\path.md",
        "; DELETE FROM schema_version; --",
        "] injected [section].md",
        "%00null%01.md",
    ];
    for (i, p) in hostile.iter().enumerate() {
        store
            .record_access(p, 1_000 + i as i64, "red-team")
            .unwrap_or_else(|e| panic!("record_access failed for {p:?}: {e:?}"));
    }

    // Schema intact: both tables still queryable, all rows present.
    let items = store.all_items().unwrap();
    assert_eq!(
        items.len(),
        hostile.len(),
        "every hostile path stored as its own row; schema not dropped"
    );

    // Round-trip: each hostile path retrieved verbatim with its own log entry.
    for (i, p) in hostile.iter().enumerate() {
        let item = store
            .get_item(p)
            .unwrap()
            .unwrap_or_else(|| panic!("missing row for {p:?}"));
        assert_eq!(&item.path, p, "path stored/retrieved verbatim");
        assert_eq!(item.last_accessed, Some(1_000 + i as i64));
        assert_eq!(store.access_log_count(p).unwrap(), 1, "one log row for {p:?}");
    }

    // The injection attempts did NOT drop tables: a fresh write still works.
    store.record_access("post-injection.md", 9_999, "broker").unwrap();
    assert!(store.get_item("post-injection.md").unwrap().is_some());
}

/// S-05: NULL last_accessed — set_membership creates a row without touching the
/// clock, so last_accessed reads back as None, and get_item/all_items handle
/// the NULL column without error.
#[test]
fn s05_null_last_accessed_handled() {
    let store = Storage::open_in_memory().unwrap();
    store.set_membership("IDENTITY.md", true, true, 100).unwrap();

    let item = store.get_item("IDENTITY.md").unwrap().unwrap();
    assert_eq!(item.last_accessed, None, "membership never sets the clock");
    assert!(item.in_working_set);
    assert!(item.always_include);

    // all_items must also decode the NULL column without panicking.
    let all = store.all_items().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].last_accessed, None);
    // No log rows because set_membership is not an access.
    assert_eq!(store.access_log_count("IDENTITY.md").unwrap(), 0);
}

/// S-05b: a membership row with NULL clock then receives an access; the clock
/// must move from NULL to the access ts (COALESCE(NULL,0) path).
#[test]
fn s05b_null_clock_then_access() {
    let mut store = Storage::open_in_memory().unwrap();
    store.set_membership("SELF.md", true, false, 100).unwrap();
    assert_eq!(store.get_item("SELF.md").unwrap().unwrap().last_accessed, None);
    store.record_access("SELF.md", 777, "broker").unwrap();
    assert_eq!(
        store.get_item("SELF.md").unwrap().unwrap().last_accessed,
        Some(777),
        "NULL clock advances on first access"
    );
}

// ---------------------------------------------------------------------------
// 4. Independent contract cross-check — defaults must equal approved values
// ---------------------------------------------------------------------------

/// C-01: Independently re-assert every approved default value. This is a
/// contract guard: if any default drifts from the approved plan (decision (a)
/// + WP0 spec), this test fails and the completion gate must block.
#[test]
fn c01_defaults_match_approved_contract() {
    let cfg = BrokerConfig::default();

    // Verdict thresholds (approved decision (a)).
    assert_eq!(cfg.verdict.min_score, 0.55, "approved min_score");
    assert_eq!(cfg.verdict.sufficient_top1, 0.65, "approved sufficient_top1");
    assert_eq!(cfg.verdict.min_results, 2, "approved min_results");
    assert_eq!(cfg.verdict.top_k, 8, "approved top_k");

    // Working-set policy (approved spec).
    assert_eq!(cfg.working_set.window_days, 30, "approved 1-month window");
    assert_eq!(
        cfg.working_set.always_include,
        vec![
            "IDENTITY.md".to_string(),
            "SELF.md".to_string(),
            "episodes/_recent.md".to_string()
        ],
        "approved 3 pinned files"
    );
    assert_eq!(
        cfg.working_set.always_include.len(),
        3,
        "exactly 3 always-include files pinned"
    );

    // The default config must itself validate.
    assert!(
        BrokerConfig::from_toml_str("").is_ok(),
        "empty TOML must round-trip to a valid default config"
    );
}

/// C-02: defaults are self-consistent w.r.t. the validation invariants (guards
/// against a default that would be rejected by its own validator).
#[test]
fn c02_defaults_satisfy_all_invariants() {
    let cfg = BrokerConfig::default();
    assert!((0.0..=1.0).contains(&cfg.verdict.min_score));
    assert!((0.0..=1.0).contains(&cfg.verdict.sufficient_top1));
    assert!(cfg.verdict.sufficient_top1 >= cfg.verdict.min_score);
    assert!(cfg.verdict.min_results >= 1);
    assert!(cfg.verdict.top_k >= cfg.verdict.min_results);
    assert!(cfg.working_set.window_days >= 1);
    assert!(!cfg.minky.base_url.is_empty());
}
