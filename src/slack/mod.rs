//! Slack integration module
//!
//! Provides commands for interacting with Slack:
//! - Authenticate via OAuth browser flow
//! - List channels
//! - Get channel info
//! - Send messages
//! - View message history
//! - Search messages
//! - List users
//! - Show configuration status
//!
//! # Examples
//!
//! ```no_run
//! use hu::slack::{run, SlackCommands};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // List channels
//!     run(SlackCommands::Channels { json: false }).await?;
//!     Ok(())
//! }
//! ```

mod auth;
mod channels;
mod client;
mod config;
mod display;
mod messages;
mod search;
mod tidy;
mod types;

use anyhow::Result;
use clap::Subcommand;

pub use config::{config_path, load_config};
pub use types::OutputFormat;

use client::SlackClient;

/// Slack subcommands
#[derive(Subcommand, Debug)]
pub enum SlackCommands {
    /// Authenticate with Slack (OAuth flow or direct token)
    Auth {
        /// Bot token to save directly (skips OAuth flow)
        #[arg(short, long)]
        token: Option<String>,
        /// User token for search API (xoxp-...)
        #[arg(short, long)]
        user_token: Option<String>,
        /// Local server port for OAuth callback
        #[arg(short, long, default_value = "9877")]
        port: u16,
    },
    /// List channels in the workspace
    Channels {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// Show channel details
    Info {
        /// Channel name or ID (e.g., "#general" or "C12345678")
        channel: String,
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// Send a message to a channel
    Send {
        /// Channel name or ID
        channel: String,
        /// Message text
        message: String,
    },
    /// Show message history for a channel
    History {
        /// Channel name or ID
        channel: String,
        /// Number of messages to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// Search messages
    Search {
        /// Search query
        query: String,
        /// Maximum results to return
        #[arg(short = 'n', long, default_value = "20")]
        count: usize,
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// List users in the workspace
    Users {
        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },
    /// Show Slack configuration status
    Config,
    /// Show current user info from token
    Whoami,
    /// Mark channels as read if no direct mentions
    Tidy {
        /// Dry run - show what would be marked without marking
        #[arg(short, long)]
        dry_run: bool,
    },
}

/// Run a Slack command
pub async fn run(command: SlackCommands) -> Result<()> {
    match command {
        SlackCommands::Auth {
            token,
            user_token,
            port,
        } => cmd_auth(token.as_deref(), user_token.as_deref(), port).await,
        SlackCommands::Channels { json } => cmd_channels(json).await,
        SlackCommands::Info { channel, json } => cmd_info(&channel, json).await,
        SlackCommands::Send { channel, message } => cmd_send(&channel, &message).await,
        SlackCommands::History {
            channel,
            limit,
            json,
        } => cmd_history(&channel, limit, json).await,
        SlackCommands::Search { query, count, json } => cmd_search(&query, count, json).await,
        SlackCommands::Users { json } => cmd_users(json).await,
        SlackCommands::Config => cmd_config(),
        SlackCommands::Whoami => cmd_whoami().await,
        SlackCommands::Tidy { dry_run } => cmd_tidy(dry_run).await,
    }
}

/// Verify a Slack token by calling auth.test and return the response
async fn verify_token(token: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://slack.com/api/auth.test")
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    if result.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let error = result
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        anyhow::bail!("Token validation failed: {}", error);
    }

    Ok(result)
}

/// Authenticate with Slack via OAuth or direct token
async fn cmd_auth(token: Option<&str>, user_token: Option<&str>, port: u16) -> Result<()> {
    // If user token provided, save it
    if let Some(user_tok) = user_token {
        if !user_tok.starts_with("xoxp-") {
            anyhow::bail!("Invalid user token format. Token should start with 'xoxp-'");
        }
        verify_token(user_tok).await?;
        config::update_user_token(user_tok)?;
        println!("User token saved successfully!");
        println!("\nYou can now use `hu slack search` command.");
        return Ok(());
    }

    // If bot token provided directly, save it and verify
    if let Some(bot_token) = token {
        if !bot_token.starts_with("xoxb-") {
            anyhow::bail!("Invalid bot token format. Token should start with 'xoxb-'");
        }
        let result = verify_token(bot_token).await?;
        let team_id = result
            .get("team_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let team_name = result
            .get("team")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Unknown");
        config::update_oauth_tokens(bot_token, team_id, team_name)?;
        println!("Token saved successfully!");
        println!("Connected to: {}", team_name);
        println!("\nYou can now use `hu slack channels` and other commands.");
        return Ok(());
    }

    // Otherwise, run OAuth flow
    let result = auth::run_oauth_flow(port).await?;

    if result.success {
        println!("\nAuthentication successful!");
        if let Some(team) = result.team_name {
            println!("Connected to: {}", team);
        }
        println!("\nYou can now use `hu slack channels` and other commands.");
    } else {
        let error = result.error.unwrap_or_else(|| "Unknown error".to_string());
        anyhow::bail!("Authentication failed: {}", error);
    }

    Ok(())
}

/// List channels
async fn cmd_channels(json: bool) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let channels = channels::list_channels(&client).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_channels(&channels, format)?;
    Ok(())
}

/// Get channel info
async fn cmd_info(channel: &str, json: bool) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let channel_id = channels::resolve_channel(&client, channel).await?;
    let info = channels::get_channel_info(&client, &channel_id).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_channel_detail(&info, format)?;
    Ok(())
}

/// Send a message
async fn cmd_send(channel: &str, text: &str) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let channel_id = channels::resolve_channel(&client, channel).await?;
    let (sent_channel, ts) = messages::send_message(&client, &channel_id, text).await?;

    println!("Message sent to {} (ts: {})", sent_channel, ts);
    Ok(())
}

/// Get message history
async fn cmd_history(channel: &str, limit: usize, json: bool) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let channel_id = channels::resolve_channel(&client, channel).await?;
    let messages = messages::get_history(&client, &channel_id, limit).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    // Get channel name for display
    let channel_name = channel.trim_start_matches('#');
    display::output_messages(&messages, channel_name, format)?;
    Ok(())
}

/// Search messages
async fn cmd_search(query: &str, count: usize, json: bool) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let results = search::search_messages(&client, query, count).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    // Build user lookup for resolving DM user IDs to names
    let user_lookup = channels::build_user_lookup(&client).await?;

    display::output_search_results(&results, format, &user_lookup)?;
    Ok(())
}

/// List users
async fn cmd_users(json: bool) -> Result<()> {
    let client = SlackClient::new()?;
    check_configured(&client)?;

    let users = channels::list_users(&client).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    display::output_users(&users, format)?;
    Ok(())
}

/// Show configuration status
fn cmd_config() -> Result<()> {
    let config = load_config()?;

    display::output_config_status(
        config.is_configured,
        config.oauth.has_user_token(),
        config.oauth.team_name.as_deref(),
        &config.default_channel,
    );

    if let Some(path) = config_path() {
        println!("Config:     {}", path.display());
    }

    Ok(())
}

/// Show current user info from token
async fn cmd_whoami() -> Result<()> {
    let config = load_config()?;
    let token = config
        .oauth
        .user_token
        .or(config.oauth.bot_token)
        .ok_or_else(|| anyhow::anyhow!("No token configured"))?;

    let result = verify_token(&token).await?;

    println!(
        "User ID:   {}",
        result
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "User:      {}",
        result
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "Team ID:   {}",
        result
            .get("team_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );
    println!(
        "Team:      {}",
        result
            .get("team")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    );

    Ok(())
}

/// Tidy channels - mark as read if no mentions
async fn cmd_tidy(dry_run: bool) -> Result<()> {
    let client = SlackClient::new()?;
    if !client.config().oauth.has_user_token() {
        anyhow::bail!("User token required for tidy. Run `hu slack auth --user-token <token>`");
    }

    // Get user info for mention detection
    let config = load_config()?;
    let token = config.oauth.user_token.as_deref().unwrap();
    let result = verify_token(token).await?;

    let user_info = tidy::UserInfo {
        user_id: result
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: "Adam".to_string(),
        full_name: "Adam Ladachowski".to_string(),
    };

    if dry_run {
        println!("DRY RUN - no channels will be marked as read\n");
    }

    let results = tidy::tidy_channels(&client, &user_info, dry_run).await?;

    // Print results
    let mut marked = 0;
    let mut skipped = 0;
    let mut has_mentions = 0;

    for r in &results {
        match &r.action {
            tidy::TidyAction::Skipped => skipped += 1,
            tidy::TidyAction::MarkedRead => {
                marked += 1;
                println!("Marked read: #{}", r.channel_name);
            }
            tidy::TidyAction::HasMention(mention) => {
                has_mentions += 1;
                println!("Has mention: #{} - {}", r.channel_name, mention);
            }
        }
    }

    println!("\nSummary:");
    println!("  Marked as read: {}", marked);
    println!("  Has mentions:   {}", has_mentions);
    println!("  Already read:   {}", skipped);

    Ok(())
}

/// Check if Slack is configured, bail if not
fn check_configured(client: &SlackClient) -> Result<()> {
    if !client.config().is_configured {
        anyhow::bail!("Slack is not configured. Run `hu slack auth` to authenticate.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{OAuthConfig, SlackConfig};
    use reqwest::Client;

    fn make_unconfigured_client() -> SlackClient {
        let config = SlackConfig {
            oauth: OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: None,
                user_token: None,
                team_id: None,
                team_name: None,
            },
            default_channel: String::new(),
            is_configured: false,
        };
        let http = Client::builder().build().unwrap();
        SlackClient { config, http }
    }

    fn make_configured_client() -> SlackClient {
        let config = SlackConfig {
            oauth: OAuthConfig {
                client_id: None,
                client_secret: None,
                bot_token: Some("xoxb-test-token".to_string()),
                user_token: Some("xoxp-test-token".to_string()),
                team_id: Some("T12345".to_string()),
                team_name: Some("Test Team".to_string()),
            },
            default_channel: "#general".to_string(),
            is_configured: true,
        };
        let http = Client::builder().build().unwrap();
        SlackClient { config, http }
    }

    #[test]
    fn test_check_configured_when_not_configured() {
        let client = make_unconfigured_client();
        let result = check_configured(&client);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[test]
    fn test_check_configured_when_configured() {
        let client = make_configured_client();
        let result = check_configured(&client);
        assert!(result.is_ok());
    }

    #[test]
    fn test_slack_commands_debug() {
        let cmd = SlackCommands::Channels { json: false };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Channels"));
    }

    #[test]
    fn test_slack_commands_auth_debug() {
        let cmd = SlackCommands::Auth {
            token: Some("xoxb-test".to_string()),
            user_token: None,
            port: 9877,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Auth"));
        assert!(debug.contains("9877"));
    }

    #[test]
    fn test_slack_commands_info_debug() {
        let cmd = SlackCommands::Info {
            channel: "#general".to_string(),
            json: true,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Info"));
        assert!(debug.contains("#general"));
    }

    #[test]
    fn test_slack_commands_send_debug() {
        let cmd = SlackCommands::Send {
            channel: "#test".to_string(),
            message: "Hello".to_string(),
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Send"));
        assert!(debug.contains("Hello"));
    }

    #[test]
    fn test_slack_commands_history_debug() {
        let cmd = SlackCommands::History {
            channel: "#dev".to_string(),
            limit: 50,
            json: false,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("History"));
        assert!(debug.contains("50"));
    }

    #[test]
    fn test_slack_commands_search_debug() {
        let cmd = SlackCommands::Search {
            query: "deploy".to_string(),
            count: 20,
            json: true,
        };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Search"));
        assert!(debug.contains("deploy"));
    }

    #[test]
    fn test_slack_commands_users_debug() {
        let cmd = SlackCommands::Users { json: false };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Users"));
    }

    #[test]
    fn test_slack_commands_config_debug() {
        let cmd = SlackCommands::Config;
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Config"));
    }

    #[test]
    fn test_slack_commands_whoami_debug() {
        let cmd = SlackCommands::Whoami;
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Whoami"));
    }

    #[test]
    fn test_slack_commands_tidy_debug() {
        let cmd = SlackCommands::Tidy { dry_run: true };
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Tidy"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn test_output_format_reexport() {
        // Verify OutputFormat is properly re-exported
        let format = OutputFormat::Table;
        assert!(matches!(format, OutputFormat::Table));
        let format = OutputFormat::Json;
        assert!(matches!(format, OutputFormat::Json));
    }
}
