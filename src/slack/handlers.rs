use anyhow::Result;

use super::client::SlackClient;
use super::display;
use super::service;
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

/// Authenticate with Slack via OAuth or direct token
#[cfg(not(tarpaulin_include))]
async fn cmd_auth(token: Option<&str>, user_token: Option<&str>, port: u16) -> Result<()> {
    let result = service::authenticate(token, user_token, port).await?;
    display::output_auth_result(&result);
    Ok(())
}

/// List channels
#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
async fn cmd_send(channel: &str, text: &str) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;

    let client = SlackClient::new()?;
    let (sent_channel, ts) = service::send_message(&client, channel, text).await?;

    display::output_send_confirmation(&sent_channel, &ts);
    Ok(())
}

/// Get message history
#[cfg(not(tarpaulin_include))]
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

    let channel_name = channel.trim_start_matches('#');
    display::output_messages(&messages, channel_name, format)?;
    Ok(())
}

/// Search messages
#[cfg(not(tarpaulin_include))]
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

    let user_lookup = service::build_user_lookup(&client).await?;
    display::output_search_results(&results, format, &user_lookup)?;
    Ok(())
}

/// List users
#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
fn cmd_config() -> Result<()> {
    let config = service::get_config()?;

    display::output_config_status(
        config.is_configured,
        config.oauth.has_user_token(),
        config.oauth.team_name.as_deref(),
        &config.default_channel,
    );

    if let Some(path) = service::config_path() {
        display::output_config_path(&path);
    }

    Ok(())
}

/// Show current user info from token
#[cfg(not(tarpaulin_include))]
async fn cmd_whoami() -> Result<()> {
    let config = service::get_config()?;
    let info = service::whoami(&config).await?;
    display::output_whoami(&info);
    Ok(())
}

/// Tidy channels - mark as read if no mentions
#[cfg(not(tarpaulin_include))]
async fn cmd_tidy(dry_run: bool) -> Result<()> {
    let config = service::get_config()?;
    service::ensure_user_token(&config)?;

    if dry_run {
        display::output_tidy_dry_run();
    }

    let client = SlackClient::new()?;
    let (results, summary) = service::run_tidy(&client, &config, dry_run).await?;

    display::output_tidy_results(&results);
    display::output_tidy_summary(&summary);
    Ok(())
}
