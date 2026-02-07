use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum DocsCommand {
    /// Create a scaffold file for a topic (to be filled by Claude)
    Add(AddArgs),
    /// Fetch documentation from a URL
    Get(GetArgs),
    /// List documentation files
    List(ListArgs),
    /// Remove a documentation file
    Remove(RemoveArgs),
    /// Commit and push documentation changes
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Topic to document
    pub topic: String,
    /// Output directory (default: ~/Projects/docs)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Skip git commit
    #[arg(long)]
    pub no_commit: bool,
}

#[derive(Debug, Args)]
pub struct GetArgs {
    /// URL to fetch
    pub url: String,
    /// Output filename (derived from URL if omitted)
    pub name: Option<String>,
    /// Output directory (default: ~/Projects/docs)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    /// Skip git commit
    #[arg(long)]
    pub no_commit: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Directory to list (default: ~/Projects/docs)
    pub path: Option<PathBuf>,
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// File to remove (path or slug)
    pub file: String,
    /// Base directory for relative paths (default: ~/Projects/docs)
    #[arg(short, long)]
    pub dir: Option<PathBuf>,
    /// Skip git commit
    #[arg(long)]
    pub no_commit: bool,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Directory to sync (default: ~/Projects/docs)
    pub path: Option<PathBuf>,
    /// Skip git push
    #[arg(long)]
    pub no_push: bool,
    /// Custom commit message
    #[arg(long, short)]
    pub message: Option<String>,
    /// Output as JSON
    #[arg(long, short)]
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: DocsCommand,
    }

    #[test]
    fn parse_add() {
        let cli = TestCli::try_parse_from(["test", "add", "rust error handling"]).unwrap();
        match cli.cmd {
            DocsCommand::Add(args) => {
                assert_eq!(args.topic, "rust error handling");
                assert!(args.output.is_none());
                assert!(!args.no_commit);
            }
            _ => panic!("expected Add"),
        }
    }

    #[test]
    fn parse_add_with_options() {
        let cli =
            TestCli::try_parse_from(["test", "add", "topic", "--output", "/tmp", "--no-commit"])
                .unwrap();
        match cli.cmd {
            DocsCommand::Add(args) => {
                assert_eq!(args.output, Some(PathBuf::from("/tmp")));
                assert!(args.no_commit);
            }
            _ => panic!("expected Add"),
        }
    }

    #[test]
    fn parse_get() {
        let cli = TestCli::try_parse_from(["test", "get", "https://example.com"]).unwrap();
        match cli.cmd {
            DocsCommand::Get(args) => {
                assert_eq!(args.url, "https://example.com");
                assert!(args.name.is_none());
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn parse_get_with_name() {
        let cli =
            TestCli::try_parse_from(["test", "get", "https://example.com", "example"]).unwrap();
        match cli.cmd {
            DocsCommand::Get(args) => {
                assert_eq!(args.name, Some("example".to_string()));
            }
            _ => panic!("expected Get"),
        }
    }

    #[test]
    fn parse_list() {
        let cli = TestCli::try_parse_from(["test", "list"]).unwrap();
        match cli.cmd {
            DocsCommand::List(args) => {
                assert!(args.path.is_none());
                assert!(!args.json);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_list_with_path() {
        let cli = TestCli::try_parse_from(["test", "list", "/tmp/docs"]).unwrap();
        match cli.cmd {
            DocsCommand::List(args) => {
                assert_eq!(args.path, Some(PathBuf::from("/tmp/docs")));
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_list_json() {
        let cli = TestCli::try_parse_from(["test", "list", "--json"]).unwrap();
        match cli.cmd {
            DocsCommand::List(args) => {
                assert!(args.json);
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_remove() {
        let cli = TestCli::try_parse_from(["test", "remove", "file.md"]).unwrap();
        match cli.cmd {
            DocsCommand::Remove(args) => {
                assert_eq!(args.file, "file.md");
                assert!(!args.no_commit);
            }
            _ => panic!("expected Remove"),
        }
    }

    #[test]
    fn parse_remove_no_commit() {
        let cli = TestCli::try_parse_from(["test", "remove", "file.md", "--no-commit"]).unwrap();
        match cli.cmd {
            DocsCommand::Remove(args) => {
                assert!(args.no_commit);
            }
            _ => panic!("expected Remove"),
        }
    }

    #[test]
    fn parse_sync() {
        let cli = TestCli::try_parse_from(["test", "sync"]).unwrap();
        match cli.cmd {
            DocsCommand::Sync(args) => {
                assert!(args.path.is_none());
                assert!(!args.no_push);
            }
            _ => panic!("expected Sync"),
        }
    }

    #[test]
    fn parse_sync_with_options() {
        let cli = TestCli::try_parse_from([
            "test",
            "sync",
            "/tmp/docs",
            "--no-push",
            "-m",
            "custom message",
        ])
        .unwrap();
        match cli.cmd {
            DocsCommand::Sync(args) => {
                assert_eq!(args.path, Some(PathBuf::from("/tmp/docs")));
                assert!(args.no_push);
                assert_eq!(args.message, Some("custom message".to_string()));
            }
            _ => panic!("expected Sync"),
        }
    }

    #[test]
    fn parse_remove_with_dir() {
        let cli =
            TestCli::try_parse_from(["test", "remove", "file.md", "-d", "/custom/docs"]).unwrap();
        match cli.cmd {
            DocsCommand::Remove(args) => {
                assert_eq!(args.dir, Some(PathBuf::from("/custom/docs")));
            }
            _ => panic!("expected Remove"),
        }
    }
}
