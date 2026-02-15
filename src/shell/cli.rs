use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum ShellCommand {
    /// List directory contents with icons
    Ls(LsArgs),
    /// Show disk filesystem usage
    Df(DfArgs),
}

#[derive(Debug, Args)]
pub struct LsArgs {
    /// Directory to list (default: current directory)
    pub path: Option<PathBuf>,

    /// Show hidden files (starting with .)
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Use long listing format
    #[arg(short = 'l', long)]
    pub long: bool,

    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DfArgs {
    /// Output as JSON
    #[arg(short, long)]
    pub json: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: ShellCommand,
    }

    #[test]
    fn parse_ls_default() {
        let cli = TestCli::try_parse_from(["test", "ls"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert!(args.path.is_none());
                assert!(!args.all);
                assert!(!args.long);
                assert!(!args.json);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_ls_with_path() {
        let cli = TestCli::try_parse_from(["test", "ls", "/tmp"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert_eq!(args.path, Some(PathBuf::from("/tmp")));
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_ls_all() {
        let cli = TestCli::try_parse_from(["test", "ls", "-a"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert!(args.all);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_ls_long() {
        let cli = TestCli::try_parse_from(["test", "ls", "-l"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert!(args.long);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_ls_combined() {
        let cli = TestCli::try_parse_from(["test", "ls", "-la", "/home"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert!(args.all);
                assert!(args.long);
                assert_eq!(args.path, Some(PathBuf::from("/home")));
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_ls_json() {
        let cli = TestCli::try_parse_from(["test", "ls", "--json"]).unwrap();
        match cli.cmd {
            ShellCommand::Ls(args) => {
                assert!(args.json);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn parse_df_default() {
        let cli = TestCli::try_parse_from(["test", "df"]).unwrap();
        match cli.cmd {
            ShellCommand::Df(args) => {
                assert!(!args.json);
            }
            _ => panic!("Expected Df command"),
        }
    }

    #[test]
    fn parse_df_json() {
        let cli = TestCli::try_parse_from(["test", "df", "--json"]).unwrap();
        match cli.cmd {
            ShellCommand::Df(args) => {
                assert!(args.json);
            }
            _ => panic!("Expected Df command"),
        }
    }

    #[test]
    fn parse_df_json_short() {
        let cli = TestCli::try_parse_from(["test", "df", "-j"]).unwrap();
        match cli.cmd {
            ShellCommand::Df(args) => {
                assert!(args.json);
            }
            _ => panic!("Expected Df command"),
        }
    }
}
