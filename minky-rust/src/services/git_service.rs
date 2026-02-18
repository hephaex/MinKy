use anyhow::Result;
use chrono::{DateTime, Utc};
use std::process::Command;

use crate::models::{
    CommitRequest, CreateBranchRequest, FileStatus, GitBranch, GitCommit, GitDiff, GitDiffFile,
    GitDiffStats, GitFileChange, GitLogEntry, GitRepository, GitStatus, PullRequest, PushRequest,
};

/// Git service for repository operations
pub struct GitService {
    repo_path: String,
}

impl GitService {
    pub fn new(repo_path: &str) -> Self {
        Self {
            repo_path: repo_path.to_string(),
        }
    }

    /// Execute git command
    fn git(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!(
                "Git command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Get repository info
    pub fn get_repository_info(&self) -> Result<GitRepository> {
        let current_branch = self.git(&["branch", "--show-current"])?;
        let remote_url = self.git(&["remote", "get-url", "origin"]).ok();

        let status_output = self.git(&["status", "--porcelain"])?;
        let uncommitted_changes = status_output.lines().count() as i32;
        let is_clean = uncommitted_changes == 0;

        let last_commit = self.get_last_commit().ok();

        Ok(GitRepository {
            path: self.repo_path.clone(),
            remote_url,
            current_branch,
            is_clean,
            last_commit,
            uncommitted_changes,
        })
    }

    /// Get last commit
    pub fn get_last_commit(&self) -> Result<GitCommit> {
        let output = self.git(&[
            "log",
            "-1",
            "--format=%H%n%h%n%s%n%an%n%ae%n%aI",
        ])?;

        let lines: Vec<&str> = output.lines().collect();
        if lines.len() < 6 {
            return Err(anyhow::anyhow!("Invalid git log output"));
        }

        let date = DateTime::parse_from_rfc3339(lines[5])
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Ok(GitCommit {
            hash: lines[0].to_string(),
            short_hash: lines[1].to_string(),
            message: lines[2].to_string(),
            author: lines[3].to_string(),
            author_email: lines[4].to_string(),
            date,
        })
    }

    /// Get git status
    pub fn get_status(&self) -> Result<GitStatus> {
        let branch = self.git(&["branch", "--show-current"])?;

        // Get ahead/behind count
        let (ahead, behind) = self
            .git(&["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .ok()
            .and_then(|s| {
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() == 2 {
                    Some((
                        parts[0].parse().unwrap_or(0),
                        parts[1].parse().unwrap_or(0),
                    ))
                } else {
                    None
                }
            })
            .unwrap_or((0, 0));

        let status_output = self.git(&["status", "--porcelain"])?;

        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_status = line.chars().next().unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].to_string();

            // Staged changes
            if index_status != ' ' && index_status != '?' {
                staged.push(GitFileChange {
                    path: path.clone(),
                    status: parse_status(index_status),
                });
            }

            // Unstaged changes
            if worktree_status != ' ' && worktree_status != '?' {
                unstaged.push(GitFileChange {
                    path: path.clone(),
                    status: parse_status(worktree_status),
                });
            }

            // Untracked
            if index_status == '?' {
                untracked.push(path);
            }
        }

        Ok(GitStatus {
            branch,
            ahead,
            behind,
            staged,
            unstaged,
            untracked,
        })
    }

    /// Get commit log
    pub fn get_log(&self, limit: i32) -> Result<Vec<GitLogEntry>> {
        let output = self.git(&[
            "log",
            &format!("-{}", limit),
            "--format=%H|%h|%s|%an|%aI",
            "--shortstat",
        ])?;

        let mut entries = Vec::new();
        let mut lines = output.lines().peekable();

        while let Some(line) = lines.next() {
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 5 {
                continue;
            }

            let date = DateTime::parse_from_rfc3339(parts[4])
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            // Parse stat line if present
            let (files_changed, insertions, deletions) =
                if let Some(stat_line) = lines.peek() {
                    if stat_line.contains("file") {
                        let stat = lines.next().unwrap();
                        parse_stat_line(stat)
                    } else {
                        (0, 0, 0)
                    }
                } else {
                    (0, 0, 0)
                };

            entries.push(GitLogEntry {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                message: parts[2].to_string(),
                author: parts[3].to_string(),
                date,
                files_changed,
                insertions,
                deletions,
            });
        }

        Ok(entries)
    }

    /// List branches
    pub fn list_branches(&self) -> Result<Vec<GitBranch>> {
        let output = self.git(&["branch", "-a", "--format=%(refname:short)|%(HEAD)|%(upstream:short)"])?;

        let mut branches = Vec::new();
        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.is_empty() {
                continue;
            }

            let name = parts[0].to_string();
            let is_current = parts.get(1).map(|s| *s == "*").unwrap_or(false);
            let is_remote = name.starts_with("origin/") || name.starts_with("remotes/");
            let tracking = parts.get(2).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });

            branches.push(GitBranch {
                name,
                is_current,
                is_remote,
                last_commit: None,
                tracking,
            });
        }

        Ok(branches)
    }

    /// Create branch
    pub fn create_branch(&self, request: CreateBranchRequest) -> Result<GitBranch> {
        let base = request.from.as_deref().unwrap_or("HEAD");

        self.git(&["branch", &request.name, base])?;

        if request.checkout.unwrap_or(false) {
            self.git(&["checkout", &request.name])?;
        }

        Ok(GitBranch {
            name: request.name,
            is_current: request.checkout.unwrap_or(false),
            is_remote: false,
            last_commit: None,
            tracking: None,
        })
    }

    /// Checkout branch
    pub fn checkout(&self, branch: &str) -> Result<()> {
        self.git(&["checkout", branch])?;
        Ok(())
    }

    /// Stage files
    pub fn stage_files(&self, files: &[String]) -> Result<()> {
        let mut args = vec!["add"];
        for file in files {
            args.push(file);
        }
        self.git(&args)?;
        Ok(())
    }

    /// Commit changes
    pub fn commit(&self, request: CommitRequest) -> Result<GitCommit> {
        if let Some(files) = &request.files {
            self.stage_files(files)?;
        }

        let mut args = vec!["commit", "-m", &request.message];
        if request.amend.unwrap_or(false) {
            args.push("--amend");
        }

        self.git(&args)?;
        self.get_last_commit()
    }

    /// Push changes
    pub fn push(&self, request: PushRequest) -> Result<()> {
        let remote = request.remote.as_deref().unwrap_or("origin");
        let mut args = vec!["push", remote];

        if let Some(branch) = &request.branch {
            args.push(branch);
        }

        if request.force.unwrap_or(false) {
            args.push("--force");
        }

        self.git(&args)?;
        Ok(())
    }

    /// Pull changes
    pub fn pull(&self, request: PullRequest) -> Result<()> {
        let remote = request.remote.as_deref().unwrap_or("origin");
        let mut args = vec!["pull", remote];

        if let Some(branch) = &request.branch {
            args.push(branch);
        }

        if request.rebase.unwrap_or(false) {
            args.push("--rebase");
        }

        self.git(&args)?;
        Ok(())
    }

    /// Get diff
    pub fn get_diff(&self, staged: bool) -> Result<GitDiff> {
        let args = if staged {
            vec!["diff", "--cached", "--stat"]
        } else {
            vec!["diff", "--stat"]
        };

        let stat_output = self.git(&args)?;

        let stats = parse_diff_stats(&stat_output);

        // Get file list
        let files_args = if staged {
            vec!["diff", "--cached", "--name-status"]
        } else {
            vec!["diff", "--name-status"]
        };

        let files_output = self.git(&files_args)?;

        let files: Vec<GitDiffFile> = files_output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    Some(GitDiffFile {
                        path: parts[1].to_string(),
                        status: parse_status(parts[0].chars().next().unwrap_or('M')),
                        additions: 0,
                        deletions: 0,
                        hunks: vec![],
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(GitDiff { files, stats })
    }
}

fn parse_status(c: char) -> FileStatus {
    match c {
        'A' => FileStatus::Added,
        'M' => FileStatus::Modified,
        'D' => FileStatus::Deleted,
        'R' => FileStatus::Renamed,
        'C' => FileStatus::Copied,
        '?' => FileStatus::Untracked,
        _ => FileStatus::Modified,
    }
}

fn parse_stat_line(line: &str) -> (i32, i32, i32) {
    let mut files = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for part in line.split(',') {
        let part = part.trim();
        if part.contains("file") {
            files = part
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if part.contains("insertion") {
            insertions = part
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        } else if part.contains("deletion") {
            deletions = part
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    (files, insertions, deletions)
}

fn parse_diff_stats(output: &str) -> GitDiffStats {
    let lines: Vec<&str> = output.lines().collect();
    if let Some(last_line) = lines.last() {
        let (files, insertions, deletions) = parse_stat_line(last_line);
        GitDiffStats {
            files_changed: files,
            insertions,
            deletions,
        }
    } else {
        GitDiffStats {
            files_changed: 0,
            insertions: 0,
            deletions: 0,
        }
    }
}
