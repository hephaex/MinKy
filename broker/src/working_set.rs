//! Working-set evaluation — the short-term memory boundary.
//!
//! Approved semantics (spec + plan WP2):
//! - The working set is **always-include pins** (`IDENTITY.md`, `SELF.md`,
//!   `episodes/_recent.md`) plus every item **accessed within the window**
//!   (default 30 days).
//! - The access clock is `max(broker-recorded last_accessed, file mtime)`, so
//!   direct edits outside the broker still count as access (dual-clock
//!   resolution, deep-interview R20).
//! - Demotion is **soft and lazy**: files are never deleted or moved; an item
//!   simply stops being part of the default-loaded set. Evaluation happens at
//!   call time — no daemon, no scheduler.
//! - Recall: any recorded access re-enters the item into the working set.

use crate::config::WorkingSetConfig;
use crate::storage::{Storage, StorageError};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, thiserror::Error)]
pub enum WorkingSetError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("memory root not found: {0}")]
    MemoryRootMissing(PathBuf),
    #[error("io error under memory root: {0}")]
    Io(#[from] std::io::Error),
}

/// One evaluated memory item.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkingSetItem {
    /// Path relative to the memory root, using `/` separators.
    pub rel_path: String,
    /// Pinned via `always_include` — never demoted.
    pub pinned: bool,
    /// Effective access clock: `max(broker last_accessed, file mtime)`, epoch seconds.
    pub effective_access: i64,
    /// Whether the item is inside the current working set.
    pub in_working_set: bool,
}

/// Result of a working-set evaluation pass.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkingSetSnapshot {
    /// Items currently in the working set (pins first, then by recency desc).
    pub members: Vec<WorkingSetItem>,
    /// Items known but outside the window (soft-demoted; files untouched).
    pub demoted: Vec<WorkingSetItem>,
}

pub struct WorkingSet<'a> {
    config: &'a WorkingSetConfig,
    storage: &'a mut Storage,
}

impl<'a> WorkingSet<'a> {
    pub fn new(config: &'a WorkingSetConfig, storage: &'a mut Storage) -> Self {
        Self { config, storage }
    }

    /// Evaluate the working set at time `now` (epoch seconds).
    ///
    /// Lazy demotion happens here: membership flags in storage are updated to
    /// reflect the window; no files are ever modified.
    pub fn evaluate(&mut self, now: i64) -> Result<WorkingSetSnapshot, WorkingSetError> {
        let root = &self.config.memory_root;
        if !root.is_dir() {
            return Err(WorkingSetError::MemoryRootMissing(root.clone()));
        }
        let window_secs = i64::from(self.config.window_days) * 86_400;
        let cutoff = now - window_secs;

        let mut members = Vec::new();
        let mut demoted = Vec::new();

        for rel_path in collect_markdown_files(root)? {
            let pinned = self
                .config
                .always_include
                .iter()
                .any(|p| p == &rel_path);

            let mtime = file_mtime_epoch(&root.join(&rel_path)).unwrap_or(0);
            let recorded = self
                .storage
                .get_item(&rel_path)?
                .and_then(|item| item.last_accessed)
                .unwrap_or(0);
            let effective_access = mtime.max(recorded);

            let in_window = effective_access >= cutoff;
            let in_working_set = pinned || in_window;

            self.storage
                .set_membership(&rel_path, in_working_set, pinned, now)?;

            let item = WorkingSetItem {
                rel_path,
                pinned,
                effective_access,
                in_working_set,
            };
            if in_working_set {
                members.push(item);
            } else {
                demoted.push(item);
            }
        }

        // Pins first, then most recently accessed.
        members.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.effective_access.cmp(&a.effective_access))
                .then(a.rel_path.cmp(&b.rel_path))
        });
        demoted.sort_by(|a, b| {
            b.effective_access
                .cmp(&a.effective_access)
                .then(a.rel_path.cmp(&b.rel_path))
        });

        Ok(WorkingSetSnapshot { members, demoted })
    }

    /// Record a broker-mediated access and re-enter the item into the working
    /// set (recall). Files are never touched.
    pub fn record_access(
        &mut self,
        rel_path: &str,
        now: i64,
        source: &str,
    ) -> Result<(), WorkingSetError> {
        self.storage.record_access(rel_path, now, source)?;
        let pinned = self
            .config
            .always_include
            .iter()
            .any(|p| p == rel_path);
        self.storage.set_membership(rel_path, true, pinned, now)?;
        Ok(())
    }
}

/// Recursively collect `*.md` files under `root`, returning `/`-separated
/// relative paths. Hidden directories (e.g. `.git`) are skipped.
fn collect_markdown_files(root: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "md") {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

fn file_mtime_epoch(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let secs = mtime.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(secs).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkingSetConfig;
    use filetime::{set_file_mtime, FileTime};

    /// Build a fake memory tree with controlled mtimes.
    fn fixture(now: i64) -> (tempfile::TempDir, WorkingSetConfig, Storage) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let files: &[(&str, i64)] = &[
            ("IDENTITY.md", now - 400 * 86_400),          // pin, ancient mtime
            ("SELF.md", now - 200 * 86_400),              // pin, ancient mtime
            ("episodes/_recent.md", now - 3_600),         // pin, fresh
            ("episodes/daily/2026-07-01.md", now - 86_400), // fresh (1d)
            ("episodes/daily/2026-01-01.md", now - 120 * 86_400), // stale (120d)
            ("semantics/rust.md", now - 10 * 86_400),     // fresh (10d)
            ("bonds/user.md", now - 45 * 86_400),         // stale (45d)
        ];
        for (rel, mtime) in files {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("# {rel}")).unwrap();
            set_file_mtime(&path, FileTime::from_unix_time(*mtime, 0)).unwrap();
        }

        let config = WorkingSetConfig {
            memory_root: root.to_path_buf(),
            window_days: 30,
            always_include: vec![
                "IDENTITY.md".to_string(),
                "SELF.md".to_string(),
                "episodes/_recent.md".to_string(),
            ],
        };
        let storage = Storage::open_in_memory().unwrap();
        (dir, config, storage)
    }

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn pins_are_always_members_even_with_ancient_mtime() {
        let (_dir, config, mut storage) = fixture(NOW);
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();

        for pin in &config.always_include {
            assert!(
                snap.members.iter().any(|i| &i.rel_path == pin && i.pinned),
                "pin {pin} must be a member"
            );
        }
        // IDENTITY.md mtime is 400 days old — pinned overrides the window.
        let identity = snap.members.iter().find(|i| i.rel_path == "IDENTITY.md").unwrap();
        assert!(identity.in_working_set && identity.pinned);
    }

    #[test]
    fn window_splits_fresh_and_stale_items() {
        let (_dir, config, mut storage) = fixture(NOW);
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();

        let member_paths: Vec<_> = snap.members.iter().map(|i| i.rel_path.as_str()).collect();
        assert!(member_paths.contains(&"episodes/daily/2026-07-01.md"), "1d-old is in");
        assert!(member_paths.contains(&"semantics/rust.md"), "10d-old is in");

        let demoted_paths: Vec<_> = snap.demoted.iter().map(|i| i.rel_path.as_str()).collect();
        assert!(demoted_paths.contains(&"episodes/daily/2026-01-01.md"), "120d-old is out");
        assert!(demoted_paths.contains(&"bonds/user.md"), "45d-old is out");
    }

    #[test]
    fn lazy_demotion_flips_membership_when_window_shrinks() {
        // E2E-4: threshold-shortened automatic exit without any daemon.
        let (_dir, mut config, mut storage) = fixture(NOW);
        {
            let mut ws = WorkingSet::new(&config, &mut storage);
            let snap = ws.evaluate(NOW).unwrap();
            assert!(snap
                .members
                .iter()
                .any(|i| i.rel_path == "semantics/rust.md"));
        }
        // Shrink the window below the item's 10-day age and re-evaluate.
        config.window_days = 5;
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        assert!(
            snap.demoted.iter().any(|i| i.rel_path == "semantics/rust.md"),
            "10d-old item leaves a 5d window lazily at evaluation time"
        );
        // Storage flag reflects the demotion; the file itself still exists.
        let item = storage.get_item("semantics/rust.md").unwrap().unwrap();
        assert!(!item.in_working_set);
        assert!(config.memory_root.join("semantics/rust.md").exists(), "no deletion ever");
    }

    #[test]
    fn broker_clock_wins_over_stale_mtime() {
        let (_dir, config, mut storage) = fixture(NOW);
        // bonds/user.md has a 45d-old mtime but was read via the broker 1h ago.
        storage.record_access("bonds/user.md", NOW - 3_600, "mcp").unwrap();
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        let item = snap
            .members
            .iter()
            .find(|i| i.rel_path == "bonds/user.md")
            .expect("broker-read item re-enters despite stale mtime");
        assert_eq!(item.effective_access, NOW - 3_600, "recorded clock wins");
    }

    #[test]
    fn mtime_wins_over_stale_broker_clock() {
        // Dual-clock (R20): a direct file edit counts as access even when the
        // broker never saw a read.
        let (_dir, config, mut storage) = fixture(NOW);
        // Broker last saw this file long ago; mtime (1d) must win.
        storage
            .record_access("episodes/daily/2026-07-01.md", NOW - 90 * 86_400, "mcp")
            .unwrap();
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        let item = snap
            .members
            .iter()
            .find(|i| i.rel_path == "episodes/daily/2026-07-01.md")
            .expect("fresh mtime keeps the item in the set");
        assert_eq!(item.effective_access, NOW - 86_400, "mtime wins when newer");
    }

    #[test]
    fn recall_reenters_demoted_item() {
        let (_dir, config, mut storage) = fixture(NOW);
        {
            let mut ws = WorkingSet::new(&config, &mut storage);
            let snap = ws.evaluate(NOW).unwrap();
            assert!(snap.demoted.iter().any(|i| i.rel_path == "bonds/user.md"));
        }
        // Recall: broker-mediated retrieval records an access.
        {
            let mut ws = WorkingSet::new(&config, &mut storage);
            ws.record_access("bonds/user.md", NOW, "recall").unwrap();
        }
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        assert!(
            snap.members.iter().any(|i| i.rel_path == "bonds/user.md"),
            "recalled item re-enters the working set"
        );
    }

    #[test]
    fn missing_memory_root_is_an_error_not_a_panic() {
        let config = WorkingSetConfig {
            memory_root: PathBuf::from("/nonexistent/memory/root"),
            window_days: 30,
            always_include: vec![],
        };
        let mut storage = Storage::open_in_memory().unwrap();
        let mut ws = WorkingSet::new(&config, &mut storage);
        assert!(matches!(
            ws.evaluate(NOW),
            Err(WorkingSetError::MemoryRootMissing(_))
        ));
    }

    #[test]
    fn members_are_ordered_pins_first_then_recency() {
        let (_dir, config, mut storage) = fixture(NOW);
        let mut ws = WorkingSet::new(&config, &mut storage);
        let snap = ws.evaluate(NOW).unwrap();
        let pin_count = config.always_include.len();
        for item in snap.members.iter().take(pin_count) {
            assert!(item.pinned, "pins sort first");
        }
        let tail: Vec<_> = snap.members.iter().skip(pin_count).collect();
        for pair in tail.windows(2) {
            assert!(
                pair[0].effective_access >= pair[1].effective_access,
                "non-pinned members sorted by recency desc"
            );
        }
    }
}
