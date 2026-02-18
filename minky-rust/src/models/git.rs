use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Git repository info
#[derive(Debug, Serialize)]
pub struct GitRepository {
    pub path: String,
    pub remote_url: Option<String>,
    pub current_branch: String,
    pub is_clean: bool,
    pub last_commit: Option<GitCommit>,
    pub uncommitted_changes: i32,
}

/// Git commit info
#[derive(Debug, Serialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub date: DateTime<Utc>,
}

/// Git log entry
#[derive(Debug, Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub date: DateTime<Utc>,
    pub files_changed: i32,
    pub insertions: i32,
    pub deletions: i32,
}

/// Git status
#[derive(Debug, Serialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: i32,
    pub behind: i32,
    pub staged: Vec<GitFileChange>,
    pub unstaged: Vec<GitFileChange>,
    pub untracked: Vec<String>,
}

/// Git file change
#[derive(Debug, Serialize)]
pub struct GitFileChange {
    pub path: String,
    pub status: FileStatus,
}

/// File status in git
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
}

/// Commit request
#[derive(Debug, Deserialize)]
pub struct CommitRequest {
    pub message: String,
    pub files: Option<Vec<String>>,
    pub amend: Option<bool>,
}

/// Push request
#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub force: Option<bool>,
}

/// Pull request
#[derive(Debug, Deserialize)]
pub struct PullRequest {
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub rebase: Option<bool>,
}

/// Branch info
#[derive(Debug, Serialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
    pub last_commit: Option<GitCommit>,
    pub tracking: Option<String>,
}

/// Create branch request
#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub from: Option<String>,
    pub checkout: Option<bool>,
}

/// Git diff
#[derive(Debug, Serialize)]
pub struct GitDiff {
    pub files: Vec<GitDiffFile>,
    pub stats: GitDiffStats,
}

/// Git diff file
#[derive(Debug, Serialize)]
pub struct GitDiffFile {
    pub path: String,
    pub status: FileStatus,
    pub additions: i32,
    pub deletions: i32,
    pub hunks: Vec<GitDiffHunk>,
}

/// Diff hunk
#[derive(Debug, Serialize)]
pub struct GitDiffHunk {
    pub header: String,
    pub lines: Vec<GitDiffLine>,
}

/// Git diff line
#[derive(Debug, Serialize)]
pub struct GitDiffLine {
    pub line_type: GitLineType,
    pub content: String,
    pub old_line_number: Option<i32>,
    pub new_line_number: Option<i32>,
}

/// Git line type in diff
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GitLineType {
    Context,
    Addition,
    Deletion,
}

/// Git diff stats
#[derive(Debug, Serialize)]
pub struct GitDiffStats {
    pub files_changed: i32,
    pub insertions: i32,
    pub deletions: i32,
}
