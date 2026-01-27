use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum GhCommand {
    /// Authenticate with GitHub via OAuth Device Flow
    Login,
    /// List open pull requests authored by you
    Prs,
    /// List workflow runs
    Runs,
    /// Show CI failures
    Failures,
    /// Check CI status
    Ci,
}
