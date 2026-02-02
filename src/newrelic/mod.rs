//! New Relic integration
//!
//! Query incidents and run NRQL queries via NerdGraph.

mod client;
mod config;
mod display;
pub mod types;

use anyhow::Result;
use clap::Subcommand;

use client::NewRelicClient;
use types::OutputFormat;

/// New Relic subcommands
#[derive(Debug, Subcommand)]
pub enum NewRelicCommand {
    /// Show configuration status
    Config,

    /// Set API key and account ID
    Auth {
        /// API key (NRAK-...)
        key: String,

        /// Account ID
        #[arg(short, long)]
        account: i64,
    },

    /// List recent issues
    Issues {
        /// Maximum number of issues
        #[arg(short, long, default_value = "25")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List recent incidents
    Incidents {
        /// Maximum number of incidents
        #[arg(short, long, default_value = "25")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Run NRQL query
    Query {
        /// NRQL query string
        nrql: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Run a New Relic command
pub async fn run(cmd: NewRelicCommand) -> Result<()> {
    match cmd {
        NewRelicCommand::Config => cmd_config(),
        NewRelicCommand::Auth { key, account } => cmd_auth(&key, account),
        NewRelicCommand::Issues { limit, json } => cmd_issues(limit, json).await,
        NewRelicCommand::Incidents { limit, json } => cmd_incidents(limit, json).await,
        NewRelicCommand::Query { nrql, json } => cmd_query(&nrql, json).await,
    }
}

/// Check if client is configured
fn check_configured(client: &NewRelicClient) -> Result<()> {
    if !client.config().is_configured() {
        anyhow::bail!(
            "New Relic not configured. Run: hu newrelic auth <key> --account <id>\n\
             Or set NEW_RELIC_API_KEY and NEW_RELIC_ACCOUNT_ID environment variables."
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

/// Set auth
fn cmd_auth(key: &str, account_id: i64) -> Result<()> {
    config::save_config(key, account_id)?;
    println!("New Relic API key saved for account: {}", account_id);
    Ok(())
}

/// List issues
async fn cmd_issues(limit: usize, json: bool) -> Result<()> {
    let client = NewRelicClient::new()?;
    check_configured(&client)?;

    let issues = client.list_issues(limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_issues(&issues, format)?;
    Ok(())
}

/// List incidents
async fn cmd_incidents(limit: usize, json: bool) -> Result<()> {
    let client = NewRelicClient::new()?;
    check_configured(&client)?;

    let incidents = client.list_incidents(limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_incidents(&incidents, format)?;
    Ok(())
}

/// Run NRQL query
async fn cmd_query(nrql: &str, json: bool) -> Result<()> {
    let client = NewRelicClient::new()?;
    check_configured(&client)?;

    let results = client.run_nrql(nrql).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_nrql(&results, format)?;
    Ok(())
}
