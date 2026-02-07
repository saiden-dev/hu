use anyhow::Result;

use crate::git::{self, SyncOptions};

use super::cli::SyncArgs;

pub fn run(args: SyncArgs) -> Result<()> {
    let options = SyncOptions {
        no_commit: args.no_commit,
        no_push: args.no_push,
        message: args.message,
        path: args.path,
    };

    let result = git::sync(&options)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if result.files_committed == 0 {
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

    #[test]
    fn sync_args_to_options() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            no_commit: true,
            no_push: true,
            message: Some("test".to_string()),
            json: false,
        };

        let options = SyncOptions {
            no_commit: args.no_commit,
            no_push: args.no_push,
            message: args.message.clone(),
            path: args.path.clone(),
        };

        assert!(options.no_commit);
        assert!(options.no_push);
        assert_eq!(options.message.unwrap(), "test");
        assert_eq!(options.path.unwrap(), PathBuf::from("/tmp"));
    }

    #[test]
    fn run_not_git_repo() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            no_commit: false,
            no_push: false,
            message: None,
            json: false,
        };
        let result = run(args);
        assert!(result.is_err());
    }

    #[test]
    fn run_json_not_repo() {
        let args = SyncArgs {
            path: Some(PathBuf::from("/tmp")),
            no_commit: false,
            no_push: false,
            message: None,
            json: true,
        };
        let result = run(args);
        assert!(result.is_err());
    }
}
