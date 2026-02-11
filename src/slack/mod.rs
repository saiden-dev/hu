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
//! # CLI Usage
//! Use [`run`] for CLI commands that format and print output.
//!
//! # Programmatic Usage (MCP/HTTP)
//! Use the reusable functions that return typed data:
//! - [`get_config`] - Get configuration status
//! - [`list_channels`] - List all channels
//! - [`get_channel_info`] - Get channel details
//! - [`get_history`] - Get message history
//! - [`send_message`] - Send a message
//! - [`search_messages`] - Search messages
//! - [`list_users`] - List workspace users

mod auth;
mod channels;
mod client;
mod config;
mod display;
mod handlers;
mod messages;
mod search;
mod service;
mod tidy;
mod types;

use anyhow::Result;
use clap::Subcommand;

use client::SlackClient;
pub use config::SlackConfig;
pub use handlers::run;
pub use types::{SlackChannel, SlackMessage, SlackSearchResult, SlackUser};

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

// ============================================================================
// Reusable functions for MCP/HTTP - return typed data, never print
// ============================================================================

/// Get Slack configuration status (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub fn get_config() -> Result<SlackConfig> {
    service::get_config()
}

/// List all channels (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn list_channels() -> Result<Vec<SlackChannel>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SlackClient::new()?;
    service::list_channels(&client).await
}

/// Get channel info by name or ID (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn get_channel_info(channel: &str) -> Result<SlackChannel> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SlackClient::new()?;
    service::get_channel_info(&client, channel).await
}

/// Get message history for a channel (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn get_history(channel: &str, limit: usize) -> Result<Vec<SlackMessage>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SlackClient::new()?;
    service::get_history(&client, channel, limit).await
}

/// Send a message to a channel (for MCP/HTTP)
/// Returns (channel_id, timestamp)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn send_message(channel: &str, text: &str) -> Result<(String, String)> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SlackClient::new()?;
    service::send_message(&client, channel, text).await
}

/// Search messages (for MCP/HTTP) - requires user token
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn search_messages(query: &str, count: usize) -> Result<SlackSearchResult> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    service::ensure_user_token(&config)?;
    let client = SlackClient::new()?;
    service::search_messages(&client, query, count).await
}

/// List users in the workspace (for MCP/HTTP)
#[allow(dead_code)]
#[cfg(not(tarpaulin_include))]
pub async fn list_users() -> Result<Vec<SlackUser>> {
    let config = service::get_config()?;
    service::ensure_configured(&config)?;
    let client = SlackClient::new()?;
    service::list_users(&client).await
}

#[cfg(test)]
mod tests;
