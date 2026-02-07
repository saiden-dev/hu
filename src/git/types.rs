use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of git status operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    /// Files that have been modified
    pub modified: Vec<PathBuf>,
    /// Files that are staged for commit
    pub staged: Vec<PathBuf>,
    /// Untracked files
    pub untracked: Vec<PathBuf>,
    /// Deleted files
    pub deleted: Vec<PathBuf>,
}

impl GitStatus {
    /// Returns true if there are no changes
    pub fn is_clean(&self) -> bool {
        self.modified.is_empty()
            && self.staged.is_empty()
            && self.untracked.is_empty()
            && self.deleted.is_empty()
    }

    /// Total number of changed files
    pub fn file_count(&self) -> usize {
        self.modified.len() + self.staged.len() + self.untracked.len() + self.deleted.len()
    }
}

/// Result of sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Number of files committed
    pub files_committed: usize,
    /// Commit hash (short form)
    pub commit_hash: Option<String>,
    /// Whether changes were pushed
    pub pushed: bool,
    /// Branch name
    pub branch: Option<String>,
}

/// Options for sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncOptions {
    /// Skip git commit
    pub no_commit: bool,
    /// Skip git push
    pub no_push: bool,
    /// Custom commit message (if None, uses default format)
    pub message: Option<String>,
    /// Working directory (if None, uses current directory)
    pub path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_status_is_clean_when_empty() {
        let status = GitStatus {
            modified: vec![],
            staged: vec![],
            untracked: vec![],
            deleted: vec![],
        };
        assert!(status.is_clean());
    }

    #[test]
    fn git_status_is_not_clean_with_modified() {
        let status = GitStatus {
            modified: vec![PathBuf::from("file.txt")],
            staged: vec![],
            untracked: vec![],
            deleted: vec![],
        };
        assert!(!status.is_clean());
    }

    #[test]
    fn git_status_is_not_clean_with_staged() {
        let status = GitStatus {
            modified: vec![],
            staged: vec![PathBuf::from("file.txt")],
            untracked: vec![],
            deleted: vec![],
        };
        assert!(!status.is_clean());
    }

    #[test]
    fn git_status_is_not_clean_with_untracked() {
        let status = GitStatus {
            modified: vec![],
            staged: vec![],
            untracked: vec![PathBuf::from("file.txt")],
            deleted: vec![],
        };
        assert!(!status.is_clean());
    }

    #[test]
    fn git_status_is_not_clean_with_deleted() {
        let status = GitStatus {
            modified: vec![],
            staged: vec![],
            untracked: vec![],
            deleted: vec![PathBuf::from("file.txt")],
        };
        assert!(!status.is_clean());
    }

    #[test]
    fn git_status_file_count() {
        let status = GitStatus {
            modified: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            staged: vec![PathBuf::from("c.txt")],
            untracked: vec![PathBuf::from("d.txt")],
            deleted: vec![],
        };
        assert_eq!(status.file_count(), 4);
    }

    #[test]
    fn git_status_file_count_empty() {
        let status = GitStatus {
            modified: vec![],
            staged: vec![],
            untracked: vec![],
            deleted: vec![],
        };
        assert_eq!(status.file_count(), 0);
    }

    #[test]
    fn sync_options_default() {
        let opts = SyncOptions::default();
        assert!(!opts.no_commit);
        assert!(!opts.no_push);
        assert!(opts.message.is_none());
        assert!(opts.path.is_none());
    }

    #[test]
    fn sync_result_debug() {
        let result = SyncResult {
            files_committed: 5,
            commit_hash: Some("abc1234".to_string()),
            pushed: true,
            branch: Some("main".to_string()),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("abc1234"));
    }
}
