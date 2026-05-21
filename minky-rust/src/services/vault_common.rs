//! Shared path-safety helpers for vault ingestion and the vault watcher.
//!
//! Both the manual-ingest route (`routes/vault.rs`) and the future
//! `VaultWatcherService` must apply the same path-traversal guards and
//! markdown-file collection logic.  Keeping a single canonical implementation
//! here prevents the two code paths from diverging.

use std::path::{Path, PathBuf};

/// Maximum file size that the vault pipeline will process (10 MiB).
///
/// Files larger than this constant are silently skipped during collection /
/// ingestion to prevent excessive memory pressure.
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;

/// Hard cap on the number of `.md` files collected in a single scan.
///
/// Shared by the manual-ingest route and the vault watcher initial scan so
/// both code paths apply the same safety limit.
pub const MAX_FILES_HARD_CAP: usize = 500;

// ── Path validation ───────────────────────────────────────────────────────────

/// Validate that `path_str` is an acceptable vault path.
///
/// Rules enforced:
/// 1. The path must be absolute.
/// 2. The path must not contain any `..` (parent-directory) components —
///    this prevents path-traversal attacks.
/// 3. The path must exist on the filesystem.
///
/// Returns the resolved [`PathBuf`] on success, or a human-readable error
/// message on failure.
///
/// # Examples
///
/// ```
/// use minky::services::vault_common::validate_path;
///
/// assert!(validate_path("/tmp").is_ok());
/// assert!(validate_path("relative/path").is_err());
/// assert!(validate_path("/tmp/../etc").is_err());
/// ```
pub fn validate_path(path_str: &str) -> Result<PathBuf, String> {
    let path = Path::new(path_str);

    if !path.is_absolute() {
        return Err("path must be an absolute path".to_string());
    }

    // Reject any path that contains a `..` component (path-traversal guard).
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err("path must not contain '..' components".to_string());
        }
    }

    if !path.exists() {
        return Err(format!("path does not exist: {path_str}"));
    }

    Ok(path.to_path_buf())
}

// ── Markdown file predicates ──────────────────────────────────────────────────

/// Return `true` when `path` is a plain (non-symlink) `.md` file.
///
/// Uses [`std::fs::symlink_metadata`] so that symbolic links are never
/// silently followed — this is the same defence-in-depth strategy applied
/// throughout the collection helpers.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use minky::services::vault_common::is_safe_md_path;
///
/// // A real markdown file:
/// // assert!(is_safe_md_path(Path::new("/some/vault/note.md")));
/// // A symlink or a non-.md file returns false.
/// assert!(!is_safe_md_path(Path::new("/tmp/image.png")));
/// ```
pub fn is_safe_md_path(path: &Path) -> bool {
    let meta = match path.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    if meta.file_type().is_symlink() {
        return false;
    }
    if !meta.is_file() {
        return false;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

// ── File collection ───────────────────────────────────────────────────────────

/// Collect `.md` files reachable from `dir`, up to `max_files`.
///
/// Behaviour:
/// - When `dir` itself is a `.md` file its path is returned directly (after
///   verifying it passes [`is_safe_md_path`]).
/// - When `recursive` is `false` only the immediate children of `dir` are
///   considered; sub-directories are skipped.
/// - Symbolic links are **never** followed at any level.
/// - Results within each directory level are returned in lexicographic order.
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use minky::services::vault_common::collect_md_files;
///
/// let files = collect_md_files(Path::new("/my/vault"), true, 100);
/// ```
pub fn collect_md_files(dir: &Path, recursive: bool, max_files: usize) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Use symlink_metadata so we never follow symlinks.
    let root_meta = match dir.symlink_metadata() {
        Ok(m) => m,
        Err(_) => return files,
    };
    if root_meta.file_type().is_symlink() {
        return files;
    }
    if root_meta.is_file() {
        if is_safe_md_path(dir) {
            files.push(dir.to_path_buf());
        }
        return files;
    }

    collect_md_recursive(dir, recursive, max_files, &mut files);
    files
}

fn collect_md_recursive(
    dir: &Path,
    recursive: bool,
    limit: usize,
    out: &mut Vec<PathBuf>,
) {
    if out.len() >= limit {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut sorted: Vec<_> = entries.flatten().collect();
    sorted.sort_by_key(|e| e.path());

    for entry in sorted {
        if out.len() >= limit {
            break;
        }
        let path = entry.path();

        // H5: skip hidden entries (files or directories whose names start
        // with '.', e.g. `.obsidian/`, `.git/`, `.DS_Store`).
        let entry_name = entry.file_name();
        let name_str = entry_name.to_str().unwrap_or("");
        if name_str.starts_with('.') {
            continue;
        }

        // Use symlink_metadata to avoid following symlinks (path-traversal guard).
        let meta = match path.symlink_metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() && recursive {
            collect_md_recursive(&path, recursive, limit, out);
        } else if meta.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "md" {
                out.push(path);
            }
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_path ─────────────────────────────────────────────────────────

    #[test]
    fn validate_path_rejects_relative() {
        assert!(validate_path("relative/path").is_err());
    }

    #[test]
    fn validate_path_rejects_traversal() {
        assert!(validate_path("/valid/root/../../../etc/passwd").is_err());
    }

    #[test]
    fn validate_path_rejects_traversal_simple() {
        assert!(validate_path("/tmp/../etc/shadow").is_err());
    }

    #[test]
    fn validate_path_accepts_existing_absolute() {
        assert!(validate_path("/tmp").is_ok());
    }

    #[test]
    fn validate_path_rejects_nonexistent() {
        assert!(validate_path("/this/path/definitely/does/not/exist/9f3b2a").is_err());
    }

    // ── is_safe_md_path ───────────────────────────────────────────────────────

    #[test]
    fn is_safe_md_path_rejects_nonexistent() {
        assert!(!is_safe_md_path(Path::new("/no/such/file.md")));
    }

    #[test]
    fn is_safe_md_path_rejects_non_md_extension() {
        // /tmp itself is a directory, not a .md file.
        assert!(!is_safe_md_path(Path::new("/tmp")));
    }

    // ── collect_md_files ──────────────────────────────────────────────────────

    fn make_temp_vault() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).expect("write temp file");
        p
    }

    #[test]
    fn collect_single_file_path() {
        let dir = make_temp_vault();
        let f = write_file(dir.path(), "note.md", "# Hello");
        let files = collect_md_files(&f, false, 10);
        assert_eq!(files, vec![f]);
    }

    #[test]
    fn collect_ignores_non_md_files() {
        let dir = make_temp_vault();
        write_file(dir.path(), "note.md", "# Hello");
        write_file(dir.path(), "image.png", "binary");
        write_file(dir.path(), "readme.txt", "text");
        let files = collect_md_files(dir.path(), false, 100);
        assert_eq!(files.len(), 1);
        assert!(files[0].to_str().unwrap().ends_with("note.md"));
    }

    #[test]
    fn collect_respects_limit() {
        let dir = make_temp_vault();
        for i in 0..10 {
            write_file(dir.path(), &format!("note{i}.md"), "body");
        }
        let files = collect_md_files(dir.path(), false, 3);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn collect_non_recursive_skips_subdir() {
        let dir = make_temp_vault();
        write_file(dir.path(), "root.md", "root");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        write_file(&sub, "child.md", "child");
        let files = collect_md_files(dir.path(), false, 100);
        assert_eq!(files.len(), 1, "should only find root-level file");
    }

    #[test]
    fn collect_recursive_includes_subdir() {
        let dir = make_temp_vault();
        write_file(dir.path(), "root.md", "root");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        write_file(&sub, "child.md", "child");
        let files = collect_md_files(dir.path(), true, 100);
        assert_eq!(files.len(), 2, "should find both root and sub-dir files");
    }

    #[test]
    fn collect_single_non_md_file_returns_empty() {
        let dir = make_temp_vault();
        let f = write_file(dir.path(), "data.json", "{}");
        let files = collect_md_files(&f, false, 10);
        assert!(files.is_empty());
    }

    // ── H5: dotfile / hidden-directory skipping ────────────────────────────────

    #[test]
    fn collect_skips_hidden_files() {
        let dir = make_temp_vault();
        write_file(dir.path(), "visible.md", "# Visible");
        write_file(dir.path(), ".hidden.md", "# Hidden");
        let files = collect_md_files(dir.path(), false, 100);
        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap().to_str().unwrap() == "visible.md");
    }

    #[test]
    fn collect_skips_hidden_directories_recursively() {
        let dir = make_temp_vault();
        write_file(dir.path(), "root.md", "# Root");
        // .obsidian/ is a hidden directory Obsidian uses — must be skipped.
        let obsidian_dir = dir.path().join(".obsidian");
        std::fs::create_dir(&obsidian_dir).unwrap();
        write_file(&obsidian_dir, "config.md", "# Config");
        let files = collect_md_files(dir.path(), true, 100);
        assert_eq!(files.len(), 1, "files inside .obsidian/ must be skipped");
        assert!(files[0].file_name().unwrap().to_str().unwrap() == "root.md");
    }

    #[test]
    fn collect_skips_git_directory() {
        let dir = make_temp_vault();
        write_file(dir.path(), "note.md", "# Note");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        write_file(&git_dir, "HEAD.md", "ref: refs/heads/main");
        let files = collect_md_files(dir.path(), true, 100);
        assert_eq!(files.len(), 1, ".git/ entries must be skipped");
    }
}
