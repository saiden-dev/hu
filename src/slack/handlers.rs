use anyhow::Result;

use super::auth;
use super::client::SlackClient;
use super::config::{self, load_config};
use super::display;
use super::service;
use super::tidy;
use super::types::OutputFormat;
use super::SlackCommands;

/// Run a Slack command (CLI entry point - formats and prints)
#[cfg(not(tarpaulin_include))]
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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let channels = service::list_channels(&client).await?;
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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let info = service::get_channel_info(&client, channel).await?;
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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let (sent_channel, ts) = service::send_message(&client, channel, text).await?;

    println!("Message sent to {} (ts: {})", sent_channel, ts);
    Ok(())
}

/// Get message history
async fn cmd_history(channel: &str, limit: usize, json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let messages = service::get_history(&client, channel, limit).await?;
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
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let results = service::search_messages(&client, query, count).await?;
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    // Build user lookup for resolving DM user IDs to names
    let user_lookup = service::build_user_lookup(&client).await?;

    display::output_search_results(&results, format, &user_lookup)?;
    Ok(())
}

/// List users
async fn cmd_users(json: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let users = service::list_users(&client).await?;
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
    let config = service::get_config()?;

    display::output_config_status(
        config.is_configured,
        config.oauth.has_user_token(),
        config.oauth.team_name.as_deref(),
        &config.default_channel,
    );

    if let Some(path) = config::config_path() {
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
    let config = service::get_config()?;
    service::ensure_user_token(&config)?;

    let client = SlackClient::new()?;
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
