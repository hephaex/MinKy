//! Ultragoal G003 (WP2) working-set red-team QA.
//!
//! Adversarial, contract-derived tests whose intent is to *break* the approved
//! working-set contract (`broker/src/working_set.rs`), not to confirm the happy
//! path. Everything exercises the public API only
//! (`broker::config::WorkingSetConfig`, `broker::storage::Storage`,
//! `broker::working_set::{WorkingSet, WorkingSetError}`).
//!
//! Contract under test (spec + plan WP2):
//! - Working set = always-include pins + items accessed within a 1-month window.
//! - Access clock = max(broker last_accessed, file mtime).
//! - Demotion is soft/lazy; files are never deleted or moved.
//! - Any recorded access re-enters the item (recall).

use broker::config::WorkingSetConfig;
use broker::storage::Storage;
use broker::working_set::WorkingSet;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use filetime::{set_file_mtime, FileTime};

const NOW: i64 = 1_800_000_000;
const DAY: i64 = 86_400;

/// Build an empty memory root plus a config with no pins by default.
fn empty_env(pins: Vec<String>, window_days: u32) -> (tempfile::TempDir, WorkingSetConfig, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let config = WorkingSetConfig {
        memory_root: dir.path().to_path_buf(),
        window_days,
        always_include: pins,
    };
    let storage = Storage::open_in_memory().unwrap();
    (dir, config, storage)
}

/// Write a markdown file at `rel` (relative to root) with the given mtime.
fn write_md(root: &Path, rel: &str, mtime: i64, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, body).unwrap();
    set_file_mtime(&path, FileTime::from_unix_time(mtime, 0)).unwrap();
}

/// Snapshot every file under `root` as (rel_path, content-hash), recursively,
/// including symlinks-as-entries but not following directory-symlink cycles.
fn hash_tree(root: &Path) -> (usize, u64) {
    let mut entries: Vec<(String, u64)> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = std::collections::HashSet::new();
    while let Some(dir) = stack.pop() {
        // canonicalize guards against directory-symlink cycles in the *harness*.
        let key = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
        if !visited.insert(key) {
            continue;
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() {
                // record the symlink target string but do not traverse it
                let target = std::fs::read_link(&path).unwrap_or_default();
                let mut h = DefaultHasher::new();
                target.hash(&mut h);
                entries.push((path.to_string_lossy().into_owned(), h.finish()));
            } else if meta.is_dir() {
                stack.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap_or_default();
                let mut h = DefaultHasher::new();
                bytes.hash(&mut h);
                entries.push((path.to_string_lossy().into_owned(), h.finish()));
            }
        }
    }
    entries.sort();
    let count = entries.len();
    let mut h = DefaultHasher::new();
    entries.hash(&mut h);
    (count, h.finish())
}

// ---------------------------------------------------------------------------
// 1. Symlink cycle — evaluate() must terminate (timeout-guarded).
// ---------------------------------------------------------------------------

/// W-01: a directory-symlink cycle inside memory_root must not cause an
/// infinite loop or stack blow-up. Guarded by a wall-clock timeout: if
/// evaluate() does not return within the budget, the underlying traversal is
/// non-terminating and this is reported as a BLOCKER via a failing assertion.
///
/// The evaluation runs on a detached worker thread; if it hangs it will spin a
/// core but is killed when the test process exits (no join, bounded memory:
/// the cycle directory contains no `*.md` so the result vec never grows).
#[test]
#[cfg(unix)]
fn w01_symlink_cycle_terminates_within_timeout() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // A real, discoverable file so the happy path has something to collect.
    write_md(&root, "IDENTITY.md", NOW - 3_600, "# id");

    // Build the cycle: root/loopdir/back -> loopdir (self-referential dir).
    let loopdir = root.join("loopdir");
    std::fs::create_dir_all(&loopdir).unwrap();
    symlink(&loopdir, loopdir.join("back")).unwrap();

    let config = WorkingSetConfig {
        memory_root: root.clone(),
        window_days: 30,
        always_include: vec!["IDENTITY.md".to_string()],
    };

    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let mut storage = Storage::open_in_memory().unwrap();
        let mut ws = WorkingSet::new(&config, &mut storage);
        // Catch a panic (e.g. stack overflow / unwrap) as a distinct signal
        // from a hang, and describe the terminating outcome.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ws.evaluate(NOW)));
        let msg = match res {
            Ok(Ok(snap)) => format!(
                "OK(returned snapshot: {} members, {} demoted)",
                snap.members.len(),
                snap.demoted.len()
            ),
            Ok(Err(e)) => format!("OK(terminated with error, no hang/panic: {e})"),
            Err(_) => "PANIC".to_string(),
        };
        let _ = tx.send(msg);
    });

    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(msg) if msg == "PANIC" => panic!(
            "BLOCKER(W-01): evaluate() panicked on a directory-symlink cycle. \
             Repro: memory_root/loopdir/back -> loopdir; call evaluate(now)."
        ),
        Ok(msg) => {
            // Terminated (either a snapshot or a surfaced error): contract of
            // "no infinite loop / no stack blow-up" is upheld.
            eprintln!("W-01 symlink-cycle outcome: {msg}");
        }
        Err(mpsc::RecvTimeoutError::Timeout) => panic!(
            "BLOCKER(W-01): evaluate() did NOT terminate within 5s on a \
             directory-symlink cycle (would indicate an infinite loop in \
             collect_markdown_files: path.is_dir() follows symlinks). \
             Repro: mkdir memory_root/loopdir; ln -s memory_root/loopdir \
             memory_root/loopdir/back; call WorkingSet::evaluate(now)."
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("BLOCKER(W-01): worker thread died unexpectedly")
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Window boundary precision — effective_access == cutoff is inclusive.
// ---------------------------------------------------------------------------

/// W-02: the window predicate is `effective_access >= cutoff`. A file whose
/// clock lands exactly on the cutoff is IN; one second earlier is OUT.
#[test]
fn w02_window_boundary_is_inclusive_at_cutoff() {
    let (dir, config, mut storage) = empty_env(vec![], 30);
    let root = dir.path();
    let cutoff = NOW - 30 * DAY;

    write_md(root, "on_cutoff.md", cutoff, "# on");
    write_md(root, "before_cutoff.md", cutoff - 1, "# before");

    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();

    let on = snap
        .members
        .iter()
        .find(|i| i.rel_path == "on_cutoff.md")
        .expect("mtime == cutoff must be a member (>= is inclusive)");
    assert_eq!(on.effective_access, cutoff);
    assert!(on.in_working_set && !on.pinned);

    let before = snap
        .demoted
        .iter()
        .find(|i| i.rel_path == "before_cutoff.md")
        .expect("mtime == cutoff-1 must be demoted (strictly outside)");
    assert_eq!(before.effective_access, cutoff - 1);
    assert!(!before.in_working_set);
}

// ---------------------------------------------------------------------------
// 3. Future mtime — kept, no panic, clock respected.
// ---------------------------------------------------------------------------

/// W-03: a file with a future mtime (now + 1 day, e.g. clock skew) is well
/// within the window; it stays a member and nothing panics or overflows.
#[test]
fn w03_future_mtime_is_kept_without_panic() {
    let (dir, config, mut storage) = empty_env(vec![], 30);
    let root = dir.path();
    let future = NOW + DAY;
    write_md(root, "future.md", future, "# future");

    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();

    let item = snap
        .members
        .iter()
        .find(|i| i.rel_path == "future.md")
        .expect("future-mtime file must remain a member");
    assert_eq!(item.effective_access, future, "clock reflects the future mtime");
    assert!(item.in_working_set);
}

// ---------------------------------------------------------------------------
// 4. Unicode / whitespace / deep nesting path safety.
// ---------------------------------------------------------------------------

/// W-04: paths with unicode, spaces, and 10 levels of nesting are collected
/// safely with `/` separators and remain addressable through the whole cycle.
#[test]
fn w04_unicode_whitespace_deep_paths_are_safe() {
    let (dir, config, mut storage) = empty_env(vec![], 30);
    let root = dir.path();

    // 10-level nested path with unicode + spaces at each level.
    let segments = [
        "레벨 1", "niveau 2", "уровень 3", "レベル 4", "ζ 5",
        "level six", "층 7", "מפלס 8", "स्तर 9", "θ 10",
    ];
    let rel = format!("{}/기록 note.md", segments.join("/"));
    write_md(root, &rel, NOW - 2 * DAY, "# deep");

    // Also a plain-space top-level file and a unicode filename.
    write_md(root, "a file with spaces.md", NOW - DAY, "# spaces");
    write_md(root, "메모리.md", NOW - DAY, "# unicode name");

    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();

    let paths: Vec<&str> = snap.members.iter().map(|i| i.rel_path.as_str()).collect();
    assert!(paths.contains(&rel.as_str()), "deep unicode path collected: got {paths:?}");
    assert!(paths.contains(&"a file with spaces.md"));
    assert!(paths.contains(&"메모리.md"));
    // Separators are forward slashes only.
    assert!(rel.contains('/') && !rel.contains('\\'));

    // Recall a deep unicode path round-trips through storage without error.
    ws.record_access(&rel, NOW, "recall").unwrap();
    let stored = storage.get_item(&rel).unwrap().expect("deep path stored verbatim");
    assert_eq!(stored.path, rel);
}

// ---------------------------------------------------------------------------
// 5. Empty memory_root — empty snapshot, no error.
// ---------------------------------------------------------------------------

/// W-05: a valid but empty memory_root (zero `*.md`) yields an empty snapshot,
/// not an error and not a panic.
#[test]
fn w05_empty_memory_root_yields_empty_snapshot() {
    let (_dir, config, mut storage) = empty_env(vec![], 30);
    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();
    assert!(snap.members.is_empty(), "no files -> no members");
    assert!(snap.demoted.is_empty(), "no files -> no demoted items");
}

// ---------------------------------------------------------------------------
// 6. Pin listed in always_include but the file does not exist.
// ---------------------------------------------------------------------------

/// W-06: `always_include` names a file that is absent from disk. Behavior:
/// `collect_markdown_files` only enumerates real files, so a missing pin is
/// simply never materialized in the snapshot — no panic, no synthetic entry.
/// This test documents that observed behavior (a phantom pin is silently
/// absent rather than surfaced).
#[test]
fn w06_missing_pin_file_does_not_panic() {
    let (dir, config, mut storage) =
        empty_env(vec!["IDENTITY.md".to_string(), "ghost/PIN.md".to_string()], 30);
    let root = dir.path();
    // Only one of the two pins actually exists on disk.
    write_md(root, "IDENTITY.md", NOW - 500 * DAY, "# id ancient");

    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();

    // Existing pin is present and pinned despite an ancient mtime.
    let id = snap
        .members
        .iter()
        .find(|i| i.rel_path == "IDENTITY.md")
        .expect("existing pin is a member");
    assert!(id.pinned && id.in_working_set);

    // Missing pin is absent everywhere (documented behavior).
    let all_paths: Vec<&str> = snap
        .members
        .iter()
        .chain(snap.demoted.iter())
        .map(|i| i.rel_path.as_str())
        .collect();
    assert!(
        !all_paths.contains(&"ghost/PIN.md"),
        "a phantom pin is silently absent, not synthesized: {all_paths:?}"
    );
    // Storage was never told about the phantom pin either.
    assert!(storage.get_item("ghost/PIN.md").unwrap().is_none());
}

// ---------------------------------------------------------------------------
// 7. No-deletion invariant across the full cycle.
// ---------------------------------------------------------------------------

/// W-07: file count and content hashes are invariant across
/// evaluate -> record_access -> re-evaluate. The broker never touches files.
#[test]
fn w07_no_deletion_invariant_over_full_cycle() {
    let (dir, config, mut storage) = empty_env(vec!["IDENTITY.md".to_string()], 30);
    let root = dir.path();

    write_md(root, "IDENTITY.md", NOW - 400 * DAY, "# id");
    write_md(root, "fresh.md", NOW - DAY, "# fresh");
    write_md(root, "stale.md", NOW - 120 * DAY, "# stale");
    write_md(root, "nested/deep.md", NOW - 5 * DAY, "# deep");

    let before = hash_tree(root);

    {
        let mut ws = WorkingSet::new(&config, &mut storage);
        ws.evaluate(NOW).unwrap();
    }
    let mid = hash_tree(root);
    assert_eq!(before, mid, "evaluate() must not modify any file");

    {
        let mut ws = WorkingSet::new(&config, &mut storage);
        // Recall a demoted item.
        ws.record_access("stale.md", NOW, "recall").unwrap();
    }
    let after_recall = hash_tree(root);
    assert_eq!(before, after_recall, "record_access() must not touch files");

    {
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        // recalled item is back in the set (recall re-entry).
        assert!(snap.members.iter().any(|i| i.rel_path == "stale.md"));
    }
    let after = hash_tree(root);
    assert_eq!(before, after, "re-evaluation must not delete or alter files");
    assert_eq!(after.0, 4, "exactly the 4 authored files remain");
}

// ---------------------------------------------------------------------------
// 8. Idempotency — same `now`, twice, same snapshot.
// ---------------------------------------------------------------------------

/// W-08: two consecutive evaluate() calls at the same `now` over an unchanged
/// tree produce byte-for-byte equal snapshots (deterministic ordering, no
/// hidden state drift).
#[test]
fn w08_evaluate_is_idempotent_for_same_now() {
    let (dir, config, mut storage) = empty_env(vec!["IDENTITY.md".to_string()], 30);
    let root = dir.path();
    write_md(root, "IDENTITY.md", NOW - 400 * DAY, "# id");
    write_md(root, "a.md", NOW - DAY, "# a");
    write_md(root, "b.md", NOW - 2 * DAY, "# b");
    write_md(root, "old.md", NOW - 90 * DAY, "# old");

    let mut ws = WorkingSet::new(&config, &mut storage);
    let first = ws.evaluate(NOW).unwrap();
    let second = ws.evaluate(NOW).unwrap();
    assert_eq!(first, second, "evaluate must be idempotent for identical inputs");
}

// ---------------------------------------------------------------------------
// 9. Bulk 1000 files — performance sanity + correctness.
// ---------------------------------------------------------------------------

/// W-09: 1000 files across many subdirectories evaluate within a few seconds
/// and split correctly into fresh members and stale demoted items.
#[test]
fn w09_bulk_thousand_files_is_fast_and_consistent() {
    let (dir, config, mut storage) = empty_env(vec![], 30);
    let root = dir.path();

    let total = 1000usize;
    let mut expected_members = 0usize;
    let mut expected_demoted = 0usize;
    for i in 0..total {
        // Half fresh (in window), half stale (out of window), spread across dirs.
        let (age_days, fresh) = if i % 2 == 0 { (i as i64 % 29, true) } else { (40 + (i as i64 % 300), false) };
        if fresh {
            expected_members += 1;
        } else {
            expected_demoted += 1;
        }
        let rel = format!("d{:02}/note_{:04}.md", i % 20, i);
        write_md(root, &rel, NOW - age_days * DAY, "# bulk");
    }

    let start = Instant::now();
    let mut ws = WorkingSet::new(&config, &mut storage);
    let snap = ws.evaluate(NOW).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(snap.members.len(), expected_members, "fresh count");
    assert_eq!(snap.demoted.len(), expected_demoted, "stale count");
    assert_eq!(snap.members.len() + snap.demoted.len(), total, "all files accounted for");
    assert!(
        elapsed < Duration::from_secs(10),
        "1000-file evaluation should finish in a few seconds, took {elapsed:?}"
    );
}
