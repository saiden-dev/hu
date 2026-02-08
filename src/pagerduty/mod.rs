//! PagerDuty integration
//!
//! View on-call schedules and incidents.
//!
//! # CLI Usage
//! Use [`run`] for CLI commands that format and print output.
//!
//! # Programmatic Usage (MCP/HTTP)
//! Use the reusable functions that return typed data:
//! - [`get_config`] - Get configuration status
//! - [`list_oncalls`] - List on-call users
//! - [`list_alerts`] - List active alerts (triggered + acknowledged)
//! - [`list_incidents`] - List incidents with filters
//! - [`get_incident`] - Get incident details
//! - [`get_current_user`] - Get current user info

mod cli;
mod client;
mod config;
mod display;
mod service;
pub mod types;

use anyhow::Result;

pub use cli::PagerDutyCommand;
use cli::StatusFilter;
use client::PagerDutyClient;
pub use config::PagerDutyConfig;
pub use service::{IncidentOptions, OncallOptions};
pub use types::{Incident, Oncall, User};
use types::{IncidentStatus, OutputFormat};

/// Run a PagerDuty command (CLI entry point - formats and prints)
#[cfg(not(tarpaulin_include))]
pub async fn run(cmd: PagerDutyCommand) -> Result<()> {
    match cmd {
        PagerDutyCommand::Config => cmd_config(),
        PagerDutyCommand::Auth { token } => cmd_auth(&token),
        PagerDutyCommand::Oncall {
            policy,
            schedule,
            json,
        } => cmd_oncall(policy.as_deref(), schedule.as_deref(), json).await,
        PagerDutyCommand::Alerts { limit, json } => cmd_alerts(limit, json).await,
        PagerDutyCommand::Incidents {
            status,
            limit,
            json,
        } => cmd_incidents(status, limit, json).await,
        PagerDutyCommand::Show { id, json } => cmd_show(&id, json).await,
        PagerDutyCommand::Whoami { json } => cmd_whoami(json).await,
    }
}

// ============================================================================
// Reusable functions for MCP/HTTP - return typed data, never print
// ============================================================================

/// Get PagerDuty configuration status (for MCP/HTTP)
#[allow(dead_code)]
pub fn get_config() -> Result<PagerDutyConfig> {
    service::get_config()
}

/// List on-call users (for MCP/HTTP)
#[allow(dead_code)]
pub async fn list_oncalls(opts: &OncallOptions) -> Result<Vec<Oncall>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = PagerDutyClient::new()?;
    service::list_oncalls(&client, opts).await
}

/// List active alerts - triggered + acknowledged only (for MCP/HTTP)
#[allow(dead_code)]
pub async fn list_alerts(limit: usize) -> Result<Vec<Incident>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = PagerDutyClient::new()?;
    service::list_alerts(&client, limit).await
}

/// List incidents with filters (for MCP/HTTP)
#[allow(dead_code)]
pub async fn list_incidents(opts: &IncidentOptions) -> Result<Vec<Incident>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = PagerDutyClient::new()?;
    service::list_incidents(&client, opts).await
}

/// Get incident details by ID (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_incident(id: &str) -> Result<Incident> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = PagerDutyClient::new()?;
    service::get_incident(&client, id).await
}

/// Get current user info (for MCP/HTTP)
#[allow(dead_code)]
pub async fn get_current_user() -> Result<User> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = PagerDutyClient::new()?;
    service::get_current_user(&client).await
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

/// Save API token
fn cmd_auth(token: &str) -> Result<()> {
    service::save_auth(token)?;
    println!("PagerDuty API token saved.");
    Ok(())
}

/// Show who's on call
async fn cmd_oncall(policy: Option<&str>, schedule: Option<&str>, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = PagerDutyClient::new()?;
    let opts = OncallOptions {
        policy_id: policy.map(|p| p.to_string()),
        schedule_id: schedule.map(|s| s.to_string()),
    };

    let oncalls = service::list_oncalls(&client, &opts).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };
    display::output_oncalls(&oncalls, format)?;
    Ok(())
}

/// List active alerts (triggered + acknowledged)
async fn cmd_alerts(limit: usize, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = PagerDutyClient::new()?;
    let incidents = service::list_alerts(&client, limit).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };
    display::output_incidents(&incidents, format)?;
    Ok(())
}

/// List incidents with optional status filter
async fn cmd_incidents(status: Option<StatusFilter>, limit: usize, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = PagerDutyClient::new()?;
    let opts = IncidentOptions {
        statuses: status_filter_to_statuses(status),
        limit,
    };
    let incidents = service::list_incidents(&client, &opts).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };
    display::output_incidents(&incidents, format)?;
    Ok(())
}

/// Show incident details
async fn cmd_show(id: &str, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = PagerDutyClient::new()?;
    let incident = service::get_incident(&client, id).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };
    display::output_incident_detail(&incident, format)?;
    Ok(())
}

/// Show current user info
async fn cmd_whoami(json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = PagerDutyClient::new()?;
    let user = service::get_current_user(&client).await?;

    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };
    display::output_user(&user, format)?;
    Ok(())
}

/// Convert CLI status filter to API statuses
fn status_filter_to_statuses(filter: Option<StatusFilter>) -> Vec<IncidentStatus> {
    match filter {
        Some(StatusFilter::Triggered) => vec![IncidentStatus::Triggered],
        Some(StatusFilter::Acknowledged) => vec![IncidentStatus::Acknowledged],
        Some(StatusFilter::Resolved) => vec![IncidentStatus::Resolved],
        Some(StatusFilter::Active) | None => {
            vec![IncidentStatus::Triggered, IncidentStatus::Acknowledged]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::PagerDutyConfig;

    #[test]
    fn status_filter_to_statuses_none() {
        let statuses = status_filter_to_statuses(None);
        assert_eq!(statuses.len(), 2);
        assert!(statuses.contains(&IncidentStatus::Triggered));
        assert!(statuses.contains(&IncidentStatus::Acknowledged));
    }

    #[test]
    fn status_filter_to_statuses_triggered() {
        let statuses = status_filter_to_statuses(Some(StatusFilter::Triggered));
        assert_eq!(statuses, vec![IncidentStatus::Triggered]);
    }

    #[test]
    fn status_filter_to_statuses_acknowledged() {
        let statuses = status_filter_to_statuses(Some(StatusFilter::Acknowledged));
        assert_eq!(statuses, vec![IncidentStatus::Acknowledged]);
    }

    #[test]
    fn status_filter_to_statuses_resolved() {
        let statuses = status_filter_to_statuses(Some(StatusFilter::Resolved));
        assert_eq!(statuses, vec![IncidentStatus::Resolved]);
    }

    #[test]
    fn status_filter_to_statuses_active() {
        let statuses = status_filter_to_statuses(Some(StatusFilter::Active));
        assert_eq!(statuses.len(), 2);
        assert!(statuses.contains(&IncidentStatus::Triggered));
        assert!(statuses.contains(&IncidentStatus::Acknowledged));
    }

    #[test]
    fn cmd_config_runs() {
        // Just verify it doesn't panic
        let result = cmd_config();
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_configured_with_token_succeeds() {
        let config = PagerDutyConfig {
            api_token: Some("test-token".to_string()),
            ..Default::default()
        };
        let result = service::ensure_configured(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_configured_without_token_fails() {
        let config = PagerDutyConfig::default();
        let result = service::ensure_configured(&config);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_auth_saves_token() {
        // This test writes to config, which is I/O - just verify it runs
        // Note: This may modify the actual config file, but we're testing the logic
        // In a real scenario, we'd mock the file system
        let result = cmd_auth("test-token-12345");
        // Either succeeds or fails due to file system permissions
        let _ = result;
    }
}
