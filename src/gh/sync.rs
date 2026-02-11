use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::git::{self, SyncOptions, SyncResult};

use super::cli::SyncArgs;

/// Default log file path
fn default_log_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".hu/gh-sync.log"))
        .unwrap_or_else(|| PathBuf::from("gh-sync.log"))
}

/// Format a log line for sync result
fn format_log_line(result: &SyncResult, repo_path: &str) -> String {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let branch = result.branch.as_deref().unwrap_or("-");
    let hash = result.commit_hash.as_deref().unwrap_or("-");
    let status = if result.pushed {
        "pushed"
    } else if result.commit_hash.is_some() {
        "committed"
    } else {
        "clean"
    };

    format!(
        "{} | {} | {} | {} | {} files | {}",
        timestamp, repo_path, branch, hash, result.files_committed, status
    )
}

/// Append a log line to the log file
fn append_log(log_path: &PathBuf, line: &str) -> Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).context("Failed to create log directory")?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .context("Failed to open log file")?;

    writeln!(file, "{}", line).context("Failed to write log")?;
    Ok(())
}

pub fn run(args: SyncArgs) -> Result<()> {
    let repo_path = args
        .path
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let options = SyncOptions {
        pull: args.pull,
        trigger: args.trigger,
        no_commit: args.no_commit,
        no_push: args.no_push,
        message: args.message,
        path: args.path,
    };

    let result = git::sync(&options)?;

    // Log if requested
    if args.log {
        let log_path = args.log_file.unwrap_or_else(default_log_path);
        let repo_display = repo_path.to_string_lossy();
        let line = format_log_line(&result, &repo_display);
        append_log(&log_path, &line)?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if result.pulled {
        println!("\x1b[32m\u{2713}\x1b[0m Pulled from origin");
    }

    // Trigger mode: empty commit
    if args.trigger {
        if let Some(hash) = &result.commit_hash {
            let branch = result.branch.as_deref().unwrap_or("unknown");
            println!("\x1b[32m\u{2713}\x1b[0m Empty commit [{}] {}", branch, hash);
        }
        if result.pushed {
            println!("\x1b[32m\u{2713}\x1b[0m Pushed to origin (CI triggered)");
        }
        return Ok(());
    }

    if result.files_committed == 0 && result.commit_hash.is_none() {
        println!("Nothing to commit, working tree clean");
        return Ok(());
    }

    if let Some(hash) = &result.commit_hash {
        let branch = result.branch.as_deref().unwrap_or("unknown");
        println!(
            "\x1b[32m\u{2713}\x1b[0m Committed {} {} [{}] {}",
            result.files_committed,
            if result.files_committed == 1 {
                "file"
            } else {
                "files"
            },
            branch,
            hash
        );
    } else if args.no_commit {
        println!(
            "\x1b[33m\u{25D0}\x1b[0m {} {} changed (--no-commit)",
            result.files_committed,
            if result.files_committed == 1 {
                "file"
            } else {
                "files"
            }
        );
    }

    if result.pushed {
        println!("\x1b[32m\u{2713}\x1b[0m Pushed to origin");
    } else if !args.no_push && result.commit_hash.is_some() {
        println!("\x1b[33m\u{25D0}\x1b[0m No remote configured, skipping push");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn sync_args_to_options() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            pull: true,
            trigger: false,
            no_commit: true,
            no_push: true,
            message: Some("test".to_string()),
            log: false,
            log_file: None,
            json: false,
        };

        let options = SyncOptions {
            pull: args.pull,
            trigger: args.trigger,
            no_commit: args.no_commit,
            no_push: args.no_push,
            message: args.message.clone(),
            path: args.path.clone(),
        };

        assert!(options.pull);
        assert!(!options.trigger);
        assert!(options.no_commit);
        assert!(options.no_push);
        assert_eq!(options.message.unwrap(), "test");
        assert_eq!(options.path.unwrap(), PathBuf::from("/tmp"));
    }

    #[test]
    fn sync_args_trigger_mode() {
        let args = SyncArgs {
            path: None,
            pull: false,
            trigger: true,
            no_commit: false,
            no_push: false,
            message: Some("Retrigger build".to_string()),
            log: false,
            log_file: None,
            json: false,
        };

        let options = SyncOptions {
            pull: args.pull,
            trigger: args.trigger,
            no_commit: args.no_commit,
            no_push: args.no_push,
            message: args.message.clone(),
            path: args.path.clone(),
        };

        assert!(options.trigger);
        assert_eq!(options.message.unwrap(), "Retrigger build");
    }

    #[test]
    fn run_not_git_repo() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            pull: false,
            trigger: false,
            no_commit: false,
            no_push: false,
            message: None,
            log: false,
            log_file: None,
            json: false,
        };
        let result = run(args);
        assert!(result.is_err());
    }

    #[test]
    fn run_json_not_repo() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            pull: false,
            trigger: false,
            no_commit: false,
            no_push: false,
            message: None,
            log: false,
            log_file: None,
            json: true,
        };
        let result = run(args);
        assert!(result.is_err());
    }

    #[test]
    fn default_log_path_in_home() {
        let path = default_log_path();
        assert!(path.to_string_lossy().contains(".hu"));
        assert!(path.to_string_lossy().contains("gh-sync.log"));
    }

    #[test]
    fn format_log_line_pushed() {
        let result = SyncResult {
            pulled: false,
            files_committed: 3,
            commit_hash: Some("abc1234".to_string()),
            pushed: true,
            branch: Some("main".to_string()),
        };
        let line = format_log_line(&result, "/path/to/repo");
        assert!(line.contains("/path/to/repo"));
        assert!(line.contains("main"));
        assert!(line.contains("abc1234"));
        assert!(line.contains("3 files"));
        assert!(line.contains("pushed"));
    }

    #[test]
    fn format_log_line_committed() {
        let result = SyncResult {
            pulled: false,
            files_committed: 1,
            commit_hash: Some("def5678".to_string()),
            pushed: false,
            branch: Some("feature".to_string()),
        };
        let line = format_log_line(&result, "/repo");
        assert!(line.contains("committed"));
        assert!(line.contains("1 files"));
    }

    #[test]
    fn format_log_line_clean() {
        let result = SyncResult {
            pulled: false,
            files_committed: 0,
            commit_hash: None,
            pushed: false,
            branch: Some("main".to_string()),
        };
        let line = format_log_line(&result, "/repo");
        assert!(line.contains("clean"));
        assert!(line.contains("0 files"));
    }

    #[test]
    fn format_log_line_no_branch() {
        let result = SyncResult {
            pulled: false,
            files_committed: 0,
            commit_hash: None,
            pushed: false,
            branch: None,
        };
        let line = format_log_line(&result, "/repo");
        assert!(line.contains(" - "));
    }

    #[test]
    fn append_log_creates_file() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("subdir/test.log");

        append_log(&log_path, "test line").unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test line"));
    }

    #[test]
    fn append_log_appends() {
        let tmp = tempdir().unwrap();
        let log_path = tmp.path().join("test.log");

        append_log(&log_path, "line 1").unwrap();
        append_log(&log_path, "line 2").unwrap();

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
    }

    #[test]
    fn sync_args_with_log() {
        let args = SyncArgs {
            path: None,
            pull: false,
            trigger: false,
            no_commit: false,
            no_push: false,
            message: None,
            log: true,
            log_file: Some(PathBuf::from("/custom/log.txt")),
            json: false,
        };
        assert!(args.log);
        assert_eq!(args.log_file, Some(PathBuf::from("/custom/log.txt")));
    }
}
