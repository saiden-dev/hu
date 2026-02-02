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
    fn test_check_configured_with_configured_client() {
        let config = config::NewRelicConfig {
            api_key: Some("NRAK-configured".to_string()),
            account_id: Some(99999),
        };
        let client = NewRelicClient::with_config(config).unwrap();
        let result = check_configured(&client);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_configured_with_unconfigured_client() {
        let config = config::NewRelicConfig {
            api_key: None,
            account_id: None,
        };
        let client = NewRelicClient::with_config(config).unwrap();
        let result = check_configured(&client);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not configured"));
    }

    #[test]
    fn test_check_configured_partial_api_key_only() {
        let config = config::NewRelicConfig {
            api_key: Some("NRAK-partial".to_string()),
            account_id: None,
        };
        let client = NewRelicClient::with_config(config).unwrap();
        let result = check_configured(&client);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_configured_partial_account_only() {
        let config = config::NewRelicConfig {
            api_key: None,
            account_id: Some(12345),
        };
        let client = NewRelicClient::with_config(config).unwrap();
        let result = check_configured(&client);
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
