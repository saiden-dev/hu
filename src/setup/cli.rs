//! CLI argument types for `hu setup`.

use clap::{Args, Subcommand};

#[derive(Subcommand, Debug)]
pub enum SetupCommand {
    /// Run full setup: packages, dotfiles, ssh
    Run(RunArgs),

    /// Show what would be installed without making changes (alias of status)
    Preview,

    /// Show package and config status with ✓/✗ icons
    Status,

    /// Install/refresh packages only
    Pkgs(PkgsArgs),

    /// Clone and apply dotfiles only
    Dotfiles,

    /// Sync SSH keys from 1Password only
    Ssh,

    /// Manage the setup config file
    Config {
        #[command(subcommand)]
        cmd: Option<ConfigCommand>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Write a default setup.toml to the config dir
    Init,

    /// Print the resolved config path
    Path,
}

#[derive(Args, Debug, Default)]
pub struct RunArgs {
    /// Restrict run to a single phase
    #[arg(long, value_enum)]
    pub only: Option<RunPhase>,

    /// Print intended actions without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress interactive confirmations
    #[arg(short, long)]
    pub yes: bool,
}

#[derive(Args, Debug, Default)]
pub struct PkgsArgs {
    /// Comma-separated package names to install (default: all configured)
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Print intended actions without executing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPhase {
    Pkgs,
    Dotfiles,
    Ssh,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: SetupCommand,
    }

    #[test]
    fn parses_status() {
        let cli = TestCli::try_parse_from(["test", "status"]).unwrap();
        assert!(matches!(cli.cmd, SetupCommand::Status));
    }

    #[test]
    fn parses_run_with_dry_run() {
        let cli = TestCli::try_parse_from(["test", "run", "--dry-run"]).unwrap();
        match cli.cmd {
            SetupCommand::Run(args) => {
                assert!(args.dry_run);
                assert!(!args.yes);
                assert!(args.only.is_none());
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn parses_run_with_only_filter() {
        let cli = TestCli::try_parse_from(["test", "run", "--only", "ssh", "--yes"]).unwrap();
        match cli.cmd {
            SetupCommand::Run(args) => {
                assert_eq!(args.only, Some(RunPhase::Ssh));
                assert!(args.yes);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn parses_pkgs_with_only_csv() {
        let cli = TestCli::try_parse_from(["test", "pkgs", "--only", "gh,jq,op"]).unwrap();
        match cli.cmd {
            SetupCommand::Pkgs(args) => {
                assert_eq!(args.only, vec!["gh", "jq", "op"]);
            }
            _ => panic!("expected Pkgs"),
        }
    }

    #[test]
    fn parses_config_init() {
        let cli = TestCli::try_parse_from(["test", "config", "init"]).unwrap();
        assert!(matches!(
            cli.cmd,
            SetupCommand::Config {
                cmd: Some(ConfigCommand::Init)
            }
        ));
    }
}
