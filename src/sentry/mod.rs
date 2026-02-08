//! Sentry integration
//!
//! List and view issues from Sentry.
//!
//! # CLI Usage
//! Use [`run`] for CLI commands that format and print output.
//!
//! # Programmatic Usage (MCP/HTTP)
//! Use the reusable functions that return typed data:
//! - [`get_config`] - Get configuration status
//! - [`list_issues`] - List issues with filters
//! - [`get_issue`] - Get issue details
//! - [`list_events`] - List events for an issue

mod client;
mod config;
mod display;
mod service;
pub mod types;

use anyhow::Result;
use clap::Subcommand;

use client::SentryClient;
pub use config::SentryConfig;
pub use service::{EventOptions, IssueOptions};
use types::OutputFormat;
pub use types::{Event, Issue};

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

/// Run a Sentry command (CLI entry point - formats and prints)
#[cfg(not(tarpaulin_include))]
pub async fn run(cmd: SentryCommand) -> Result<()> {
    match cmd {
        SentryCommand::Config => cmd_config(),
        SentryCommand::Issues {
            project,
            query,
            limit,
            json,
        } => cmd_issues(project, query, limit, json).await,
        SentryCommand::Show { issue, json } => cmd_show(&issue, json).await,
        SentryCommand::Events { issue, limit, json } => cmd_events(&issue, limit, json).await,
        SentryCommand::Auth { token, org } => cmd_auth(&token, &org),
    }
}

// ============================================================================
// Reusable functions for MCP/HTTP - return typed data, never print
// ============================================================================

/// Get Sentry configuration status (for MCP/HTTP)
#[allow(dead_code)]
pub fn get_config() -> Result<SentryConfig> {
    service::get_config()
}

/// List issues with filters (for MCP/HTTP)
#[allow(dead_code)]
pub async fn list_issues(opts: &IssueOptions) -> Result<Vec<Issue>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SentryClient::new()?;
    service::list_issues(&client, opts).await
}

/// Get issue details by ID (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_issue(issue_id: &str) -> Result<Issue> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SentryClient::new()?;
    service::get_issue(&client, issue_id).await
}

/// List events for an issue (for MCP/HTTP)
#[allow(dead_code)]
pub async fn list_events(opts: &EventOptions) -> Result<Vec<Event>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SentryClient::new()?;
    service::list_events(&client, opts).await
}

// ============================================================================
// CLI command handlers - create client, call service, format and print
// ============================================================================

/// Show config status
fn cmd_config() -> Result<()> {
    let config = service::get_config()?;
    display::output_config_status(&config);
    Ok(())
}

/// Set auth token
fn cmd_auth(token: &str, org: &str) -> Result<()> {
    service::save_auth(token, org)?;
    println!("Sentry auth token saved for organization: {}", org);
    Ok(())
}

/// List issues
async fn cmd_issues(
    project: Option<String>,
    query: Option<String>,
    limit: usize,
    json: bool,
) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SentryClient::new()?;
    let opts = IssueOptions {
        project,
        query,
        limit,
    };
    let issues = service::list_issues(&client, &opts).await?;

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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SentryClient::new()?;
    let issue = service::get_issue(&client, issue_id).await?;

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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SentryClient::new()?;
    let opts = EventOptions {
        issue_id: issue_id.to_string(),
        limit,
    };
    let events = service::list_events(&client, &opts).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_events(&events, format)?;
    Ok(())
}
