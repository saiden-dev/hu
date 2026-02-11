//! New Relic integration
//!
//! Query incidents and run NRQL queries via NerdGraph.
//!
//! # CLI Usage
//! Use [`run`] for CLI commands that format and print output.
//!
//! # Programmatic Usage (MCP/HTTP)
//! Use the reusable functions that return typed data:
//! - [`get_config`] - Get configuration status
//! - [`list_issues`] - List recent issues
//! - [`list_incidents`] - List recent incidents
//! - [`run_nrql`] - Run NRQL query

mod client;
mod config;
mod display;
mod service;
pub mod types;

use anyhow::Result;
use clap::Subcommand;

use client::NewRelicClient;
pub use config::NewRelicConfig;
use types::OutputFormat;
pub use types::{Incident, Issue};

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

/// Run a New Relic command (CLI entry point - formats and prints)
#[cfg(not(tarpaulin_include))]
pub async fn run(cmd: NewRelicCommand) -> Result<()> {
    match cmd {
        NewRelicCommand::Config => cmd_config(),
        NewRelicCommand::Auth { key, account } => cmd_auth(&key, account),
        NewRelicCommand::Issues { limit, json } => cmd_issues(limit, json).await,
        NewRelicCommand::Incidents { limit, json } => cmd_incidents(limit, json).await,
        NewRelicCommand::Query { nrql, json } => cmd_query(&nrql, json).await,
    }
}

// ============================================================================
// Reusable functions for MCP/HTTP - return typed data, never print
// ============================================================================

/// Get New Relic configuration status (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub fn get_config() -> Result<NewRelicConfig> {
    service::get_config()
}

/// List recent issues (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn list_issues(limit: usize) -> Result<Vec<Issue>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = NewRelicClient::new()?;
    service::list_issues(&client, limit).await
}

/// List recent incidents (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn list_incidents(limit: usize) -> Result<Vec<Incident>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = NewRelicClient::new()?;
    service::list_incidents(&client, limit).await
}

/// Run NRQL query (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn run_nrql(nrql: &str) -> Result<Vec<serde_json::Value>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = NewRelicClient::new()?;
    service::run_nrql(&client, nrql).await
}

// ============================================================================
// CLI command handlers - create client, call service, format and print
// ============================================================================

/// Show config status
#[cfg(not(tarpaulin_include))]
fn cmd_config() -> Result<()> {
    let config = service::get_config()?;
    display::output_config_status(&config);
    Ok(())
}

/// Set auth
#[cfg(not(tarpaulin_include))]
fn cmd_auth(key: &str, account_id: i64) -> Result<()> {
    service::save_auth(key, account_id)?;
    println!("New Relic API key saved for account: {}", account_id);
    Ok(())
}

/// List issues
#[cfg(not(tarpaulin_include))]
async fn cmd_issues(limit: usize, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = NewRelicClient::new()?;
    let issues = service::list_issues(&client, limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_issues(&issues, format)?;
    Ok(())
}

/// List incidents
#[cfg(not(tarpaulin_include))]
async fn cmd_incidents(limit: usize, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = NewRelicClient::new()?;
    let incidents = service::list_incidents(&client, limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_incidents(&incidents, format)?;
    Ok(())
}

/// Run NRQL query
#[cfg(not(tarpaulin_include))]
async fn cmd_query(nrql: &str, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = NewRelicClient::new()?;
    let results = service::run_nrql(&client, nrql).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_nrql(&results, format)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_newrelic_command_config_variant() {
        let cmd = NewRelicCommand::Config;
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Config"));
    }

    #[test]
    fn test_newrelic_command_auth_variant() {
        let cmd = NewRelicCommand::Auth {
            key: "NRAK-test".to_string(),
            account: 12345,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Auth"));
        assert!(debug.contains("NRAK-test"));
        assert!(debug.contains("12345"));
    }

    #[test]
    fn test_newrelic_command_issues_variant() {
        let cmd = NewRelicCommand::Issues {
            limit: 50,
            json: true,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Issues"));
        assert!(debug.contains("50"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn test_newrelic_command_incidents_variant() {
        let cmd = NewRelicCommand::Incidents {
            limit: 10,
            json: false,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Incidents"));
        assert!(debug.contains("10"));
        assert!(debug.contains("false"));
    }

    #[test]
    fn test_newrelic_command_query_variant() {
        let cmd = NewRelicCommand::Query {
            nrql: "SELECT count(*) FROM Transaction".to_string(),
            json: true,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Query"));
        assert!(debug.contains("SELECT"));
        assert!(debug.contains("Transaction"));
    }

    #[test]
    fn test_ensure_configured_with_configured() {
        let config = config::NewRelicConfig {
            api_key: Some("NRAK-configured".to_string()),
            account_id: Some(99999),
        };
        let result = service::ensure_configured(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_configured_with_unconfigured() {
        let config = config::NewRelicConfig {
            api_key: None,
            account_id: None,
        };
        let result = service::ensure_configured(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not configured"));
    }

    #[test]
    fn test_ensure_configured_partial_api_key_only() {
        let config = config::NewRelicConfig {
            api_key: Some("NRAK-partial".to_string()),
            account_id: None,
        };
        let result = service::ensure_configured(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_ensure_configured_partial_account_only() {
        let config = config::NewRelicConfig {
            api_key: None,
            account_id: Some(12345),
        };
        let result = service::ensure_configured(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_output_format_from_json_flag_true() {
        let json = true;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert!(matches!(format, OutputFormat::Json));
    }

    #[test]
    fn test_output_format_from_json_flag_false() {
        let json = false;
        let format = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert!(matches!(format, OutputFormat::Table));
    }
}
