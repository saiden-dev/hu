use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum GhCommand {
    /// Authenticate with GitHub using a Personal Access Token
    Login(LoginArgs),
    /// List open pull requests authored by you
    Prs,
    /// Extract test failures from CI
    Failures(FailuresArgs),
    /// Analyze CI failures and output investigation context
    Fix(FixArgs),
    /// List workflow runs
    Runs(RunsArgs),
    /// Commit and push all changes (quick sync)
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Path to git repository (default: current directory)
    pub path: Option<PathBuf>,
    /// Skip git commit
    #[arg(long)]
    pub no_commit: bool,
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

#[derive(Debug, Args)]
pub struct FixArgs {
    /// PR number
    #[arg(long)]
    pub pr: Option<u64>,
    /// Workflow run ID
    #[arg(long)]
    pub run: Option<u64>,
    /// Branch name
    #[arg(long, short)]
    pub branch: Option<String>,
    /// Repository in owner/repo format
    #[arg(long, short)]
    pub repo: Option<String>,
    /// Output as JSON
    #[arg(long, short)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct RunsArgs {
    /// Ticket key to find runs for (e.g. BFR-1234)
    pub ticket: Option<String>,
    /// Filter by status: queued, in_progress, completed, success, failure
    #[arg(long, short)]
    pub status: Option<String>,
    /// Filter by branch name
    #[arg(long, short)]
    pub branch: Option<String>,
    /// Repository in owner/repo format
    #[arg(long, short)]
    pub repo: Option<String>,
    /// Max results (default: 20)
    #[arg(long, short = 'n', default_value = "20")]
    pub limit: usize,
    /// Output as JSON
    #[arg(long, short)]
    pub json: bool,
}
