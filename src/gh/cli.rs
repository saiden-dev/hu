use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum GhCommand {
    /// Authenticate with GitHub using a Personal Access Token
    Login(LoginArgs),
    /// List open pull requests authored by you
    Prs,
    /// Extract test failures from CI
    Failures(FailuresArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Personal Access Token (create at https://github.com/settings/tokens)
    #[arg(long, short)]
    pub token: String,
}

#[derive(Debug, Args)]
pub struct FailuresArgs {
    /// PR number (defaults to current branch's PR)
    #[arg(long)]
    pub pr: Option<u64>,
    /// Repository in owner/repo format (defaults to current directory's repo)
    #[arg(long, short)]
    pub repo: Option<String>,
}
