use anyhow::{Context, Result};
use chrono::Local;
use std::path::Path;
use std::process::Command;

use super::types::{GitStatus, SyncOptions, SyncResult};

/// Run a git command in a directory
fn run_git(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("Failed to run git {:?}", args))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {:?} failed: {}", args, stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Check if directory is a git repository
pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get current branch name
pub fn get_branch(path: &Path) -> Result<String> {
    let output = run_git(&["branch", "--show-current"], path)?;
    let branch = output.trim().to_string();
    if branch.is_empty() {
        anyhow::bail!("Not on a branch (detached HEAD?)");
    }
    Ok(branch)
}

/// Get git status
pub fn get_status(path: &Path) -> Result<GitStatus> {
    let output = run_git(&["status", "--porcelain"], path)?;
    parse_status_output(&output)
}

/// Parse git status --porcelain output
pub fn parse_status_output(output: &str) -> Result<GitStatus> {
    let mut status = GitStatus {
        modified: vec![],
        staged: vec![],
        untracked: vec![],
        deleted: vec![],
    };

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }

        let index = line.chars().next().unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');
        let file = line[3..].trim().to_string();
        let path = std::path::PathBuf::from(&file);

        // Handle staged changes (index column)
        match index {
            'A' | 'M' | 'R' | 'C' => status.staged.push(path.clone()),
            'D' => status.deleted.push(path.clone()),
            _ => {}
        }

        // Handle worktree changes (second column)
        match worktree {
            'M' => {
                if !status.staged.contains(&path) {
                    status.modified.push(path);
                }
            }
            'D' => {
                if !status.deleted.contains(&path) {
                    status.deleted.push(path);
                }
            }
            '?' => status.untracked.push(path),
            _ => {}
        }
    }

    Ok(status)
}

/// Generate default commit message
pub fn generate_commit_message(file_count: usize) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let files_word = if file_count == 1 { "file" } else { "files" };
    format!("Update {}, {} {}", timestamp, file_count, files_word)
}

/// Stage all changes
pub fn stage_all(path: &Path) -> Result<()> {
    run_git(&["add", "-A"], path)?;
    Ok(())
}

/// Commit staged changes
pub fn commit(path: &Path, message: &str) -> Result<String> {
    run_git(&["commit", "-m", message], path)?;
    let hash = run_git(&["rev-parse", "--short", "HEAD"], path)?;
    Ok(hash.trim().to_string())
}

/// Push to remote
pub fn push(path: &Path) -> Result<()> {
    run_git(&["push"], path)?;
    Ok(())
}

/// Check if there's a remote configured
pub fn has_remote(path: &Path) -> bool {
    run_git(&["remote"], path)
        .map(|o| !o.trim().is_empty())
        .unwrap_or(false)
}

/// Perform full sync: stage, commit, push
pub fn sync(options: &SyncOptions) -> Result<SyncResult> {
    let path = options
        .path
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    if !is_git_repo(&path) {
        anyhow::bail!("Not a git repository: {}", path.display());
    }

    let status = get_status(&path)?;
    if status.is_clean() {
        return Ok(SyncResult {
            files_committed: 0,
            commit_hash: None,
            pushed: false,
            branch: get_branch(&path).ok(),
        });
    }

    let file_count = status.file_count();
    let mut result = SyncResult {
        files_committed: file_count,
        commit_hash: None,
        pushed: false,
        branch: get_branch(&path).ok(),
    };

    if options.no_commit {
        return Ok(result);
    }

    // Stage all changes
    stage_all(&path)?;

    // Commit
    let message = options
        .message
        .clone()
        .unwrap_or_else(|| generate_commit_message(file_count));
    let hash = commit(&path, &message)?;
    result.commit_hash = Some(hash);

    // Push if requested and remote exists
    if !options.no_push && has_remote(&path) {
        push(&path)?;
        result.pushed = true;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_empty() {
        let status = parse_status_output("").unwrap();
        assert!(status.is_clean());
    }

    #[test]
    fn parse_status_modified() {
        let status = parse_status_output(" M file.txt").unwrap();
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.modified[0].to_str().unwrap(), "file.txt");
    }

    #[test]
    fn parse_status_staged() {
        let status = parse_status_output("M  file.txt").unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].to_str().unwrap(), "file.txt");
    }

    #[test]
    fn parse_status_untracked() {
        let status = parse_status_output("?? new_file.txt").unwrap();
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.untracked[0].to_str().unwrap(), "new_file.txt");
    }

    #[test]
    fn parse_status_deleted() {
        let status = parse_status_output("D  removed.txt").unwrap();
        assert_eq!(status.deleted.len(), 1);
        assert_eq!(status.deleted[0].to_str().unwrap(), "removed.txt");
    }

    #[test]
    fn parse_status_added() {
        let status = parse_status_output("A  new.txt").unwrap();
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].to_str().unwrap(), "new.txt");
    }

    #[test]
    fn parse_status_renamed() {
        let status = parse_status_output("R  old.txt -> new.txt").unwrap();
        assert_eq!(status.staged.len(), 1);
    }

    #[test]
    fn parse_status_multiple() {
        let output = " M modified.txt\nA  added.txt\n?? untracked.txt\nD  deleted.txt";
        let status = parse_status_output(output).unwrap();
        assert_eq!(status.modified.len(), 1);
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.untracked.len(), 1);
        assert_eq!(status.deleted.len(), 1);
    }

    #[test]
    fn parse_status_worktree_deleted() {
        let status = parse_status_output(" D removed.txt").unwrap();
        assert_eq!(status.deleted.len(), 1);
    }

    #[test]
    fn parse_status_both_staged_and_modified() {
        // File is staged but also has unstaged modifications
        let status = parse_status_output("MM file.txt").unwrap();
        // Staged takes precedence, modified is skipped to avoid duplicates
        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.modified.len(), 0);
    }

    #[test]
    fn parse_status_short_line() {
        let status = parse_status_output("X").unwrap();
        assert!(status.is_clean());
    }

    #[test]
    fn parse_status_copied() {
        let status = parse_status_output("C  src.txt -> dst.txt").unwrap();
        assert_eq!(status.staged.len(), 1);
    }

    #[test]
    fn generate_commit_message_single() {
        let msg = generate_commit_message(1);
        assert!(msg.contains("1 file"));
        assert!(!msg.contains("1 files"));
    }

    #[test]
    fn generate_commit_message_multiple() {
        let msg = generate_commit_message(5);
        assert!(msg.contains("5 files"));
    }

    #[test]
    fn generate_commit_message_contains_date() {
        let msg = generate_commit_message(1);
        let today = Local::now().format("%Y-%m-%d").to_string();
        assert!(msg.contains(&today));
    }

    #[test]
    fn is_git_repo_current() {
        // Current directory should be a git repo (we're in hu project)
        assert!(is_git_repo(Path::new(".")));
    }

    #[test]
    fn is_git_repo_not_repo() {
        assert!(!is_git_repo(Path::new("/tmp")));
    }

    #[test]
    fn get_branch_current() {
        let result = get_branch(Path::new("."));
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn has_remote_current() {
        // hu project should have a remote
        assert!(has_remote(Path::new(".")));
    }

    #[test]
    fn has_remote_no_repo() {
        assert!(!has_remote(Path::new("/tmp")));
    }

    #[test]
    fn sync_not_git_repo() {
        let opts = SyncOptions {
            path: Some(std::path::PathBuf::from("/tmp")),
            ..Default::default()
        };
        let result = sync(&opts);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not a git repository"));
    }
}
