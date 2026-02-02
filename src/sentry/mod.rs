//! Sentry integration
//!
//! List and view issues from Sentry.

mod client;
mod config;
mod display;
pub mod types;

use anyhow::Result;
use clap::Subcommand;

use client::SentryClient;
use types::OutputFormat;

/// Sentry subcommands
#[derive(Debug, Subcommand)]
pub enum SentryCommand {
    /// Show configuration status
    Config,

    /// List issues
    Issues {
        /// Filter by project
        #[arg(short, long)]
        project: Option<String>,

        /// Search query (Sentry search syntax)
        #[arg(short, long)]
        query: Option<String>,

        /// Maximum number of issues to return
        #[arg(short, long, default_value = "25")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show issue details
    Show {
        /// Issue ID or short ID
        issue: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List events for an issue
    Events {
        /// Issue ID or short ID
        issue: String,

        /// Maximum number of events to return
        #[arg(short, long, default_value = "25")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Set auth token
    Auth {
        /// Auth token
        token: String,

        /// Organization slug
        #[arg(short, long)]
        org: String,
    },
}

/// Run a Sentry command
pub async fn run(cmd: SentryCommand) -> Result<()> {
    match cmd {
        SentryCommand::Config => cmd_config(),
        SentryCommand::Issues {
            project,
            query,
            limit,
            json,
        } => cmd_issues(project.as_deref(), query.as_deref(), limit, json).await,
        SentryCommand::Show { issue, json } => cmd_show(&issue, json).await,
        SentryCommand::Events { issue, limit, json } => cmd_events(&issue, limit, json).await,
        SentryCommand::Auth { token, org } => cmd_auth(&token, &org),
    }
}

/// Check if client is configured
fn check_configured(client: &SentryClient) -> Result<()> {
    if !client.config().is_configured() {
        anyhow::bail!(
            "Sentry not configured. Run: hu sentry auth <token> --org <org>\n\
             Or set SENTRY_AUTH_TOKEN and SENTRY_ORG environment variables."
        );
    }
    Ok(())
}

/// Show config status
fn cmd_config() -> Result<()> {
    let config = config::load_config()?;
    display::output_config_status(&config);
    Ok(())
}

/// Set auth token
fn cmd_auth(token: &str, org: &str) -> Result<()> {
    config::save_auth_token(token, org)?;
    println!("Sentry auth token saved for organization: {}", org);
    Ok(())
}

/// List issues
async fn cmd_issues(
    project: Option<&str>,
    query: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<()> {
    let client = SentryClient::new()?;
    check_configured(&client)?;

    let issues = if let Some(proj) = project {
        client.list_project_issues(proj, query, limit).await?
    } else {
        client.list_issues(query, limit).await?
    };

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_issues(&issues, format)?;
    Ok(())
}

/// Show issue details
async fn cmd_show(issue_id: &str, json: bool) -> Result<()> {
    let client = SentryClient::new()?;
    check_configured(&client)?;

    let issue = client.get_issue(issue_id).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_issue_detail(&issue, format)?;
    Ok(())
}

/// List events for an issue
async fn cmd_events(issue_id: &str, limit: usize, json: bool) -> Result<()> {
    let client = SentryClient::new()?;
    check_configured(&client)?;

    let events = client.list_issue_events(issue_id, limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_events(&events, format)?;
    Ok(())
}
