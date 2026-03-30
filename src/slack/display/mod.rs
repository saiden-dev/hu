//! Slack output formatting

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Color, ContentArrangement, Table};
use regex::Regex;

use super::tidy;
use super::types::{
    AuthInfo, AuthResult, OutputFormat, SlackChannel, SlackMessage, SlackSearchResult, SlackUser,
    TidySummary,
};

#[cfg(test)]
mod tests;

/// Create a table with standard formatting
fn new_table(headers: Vec<&str>) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers);
    table
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Clean up Slack message text for display
/// - Converts <@U04H482TK6Z|Adam Ladachowski> to @Adam Ladachowski
/// - Converts <@U04H482TK6Z> to @username using lookup
/// - Converts <#C12345678|channel-name> to #channel-name
/// - Converts <URL|text> to text
fn clean_message_text(text: &str, user_lookup: &HashMap<String, String>) -> String {
    // Match Slack's special formatting: <...>
    let re = Regex::new(r"<([^>]+)>").unwrap();

    re.replace_all(text, |caps: &regex::Captures| {
        let content = &caps[1];

        if let Some(rest) = content.strip_prefix('@') {
            // User mention: <@U12345|Display Name> or <@U12345>
            if let Some((_, display_name)) = rest.split_once('|') {
                format!("@{}", display_name)
            } else {
                // No display name, look up user ID
                user_lookup
                    .get(rest)
                    .map(|name| format!("@{}", name))
                    .unwrap_or_else(|| format!("@{}", rest))
            }
        } else if let Some(rest) = content.strip_prefix('#') {
            // Channel mention: <#C12345|channel-name>
            if let Some((_, channel_name)) = rest.split_once('|') {
                format!("#{}", channel_name)
            } else {
                format!("#{}", rest)
            }
        } else if let Some(rest) = content.strip_prefix('!') {
            // Special mention: <!here>, <!channel>, <!everyone>
            format!("@{}", rest)
        } else if content.contains('|') {
            // URL with display text: <https://example.com|Example>
            let (_, display) = content.split_once('|').unwrap();
            display.to_string()
        } else {
            // Plain URL or other
            content.to_string()
        }
    })
    .to_string()
}

/// Format channel name for display
/// Converts mpdm-user1--user2--user3-1 to @user1, @user2, @user3
/// Converts user IDs like U04H482TK6Z to @username using lookup
fn format_channel_name(name: &str, user_lookup: &HashMap<String, String>) -> String {
    if name.starts_with("mpdm-") {
        // Multi-person DM: mpdm-user1--user2--user3-1
        let without_prefix = name.strip_prefix("mpdm-").unwrap_or(name);
        // Remove trailing -1, -2, etc.
        let without_suffix = without_prefix
            .rsplit_once('-')
            .map(|(rest, _)| rest)
            .unwrap_or(without_prefix);
        // Split on -- and format as @mentions
        let users: Vec<String> = without_suffix
            .split("--")
            .map(|u| format!("@{}", u))
            .collect();
        users.join(", ")
    } else if name.starts_with('U')
        && name.len() == 11
        && name.chars().all(|c| c.is_ascii_alphanumeric())
    {
        // User ID (DM): resolve to @username
        user_lookup
            .get(name)
            .map(|n| format!("@{}", n))
            .unwrap_or_else(|| "DM".to_string())
    } else {
        format!("#{}", name)
    }
}

/// Format Unix timestamp to readable date
fn format_timestamp(ts: &str) -> String {
    // Slack timestamps are like "1234567890.123456"
    ts.split('.')
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
        .map_or_else(
            || ts.to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M").to_string(),
        )
}

/// Output channels list
pub fn output_channels(channels: &[SlackChannel], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if channels.is_empty() {
                println!("No channels found.");
                return Ok(());
            }
            let mut table = new_table(vec!["Name", "Type", "Members", "Topic"]);
            for channel in channels {
                let kind = if channel.is_private {
                    "private"
                } else {
                    "public"
                };
                let members = channel.num_members.map_or("-".into(), |n| n.to_string());
                let topic = channel.topic.as_deref().unwrap_or("-");
                table.add_row(vec![
                    Cell::new(format!("#{}", channel.name)).fg(Color::Cyan),
                    Cell::new(kind),
                    Cell::new(members),
                    Cell::new(truncate(topic, 40)),
                ]);
            }
            println!("{table}");
            println!("\n{} channels", channels.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(channels)
                .context("Failed to serialize channels to JSON")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output channel detail
pub fn output_channel_detail(channel: &SlackChannel, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            let sep = "-".repeat(60);
            let kind = if channel.is_private {
                "private"
            } else {
                "public"
            };
            let member = if channel.is_member { "yes" } else { "no" };
            println!("{sep}");
            println!("#{} ({})", channel.name, channel.id);
            println!("{sep}");
            println!("Type:    {kind}");
            println!("Member:  {member}");
            if let Some(n) = channel.num_members {
                println!("Members: {n}");
            }
            if let Some(ref topic) = channel.topic {
                println!("\nTopic: {topic}");
            }
            if let Some(ref purpose) = channel.purpose {
                println!("\nPurpose: {purpose}");
            }
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(channel)
                .context("Failed to serialize channel to JSON")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output message history
pub fn output_messages(
    messages: &[SlackMessage],
    channel_name: &str,
    format: OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if messages.is_empty() {
                println!("No messages found.");
                return Ok(());
            }
            println!("Messages in #{channel_name}");
            println!("{}", "-".repeat(60));
            for msg in messages.iter().rev() {
                let time = format_timestamp(&msg.ts);
                let user = msg
                    .username
                    .as_deref()
                    .or(msg.user.as_deref())
                    .unwrap_or("unknown");
                let thread = msg
                    .reply_count
                    .map_or(String::new(), |n| format!(" [{n} replies]"));
                println!("[{time}] {user}: {}{thread}", msg.text);
            }
            println!("\n{} messages", messages.len());
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(messages)
                .context("Failed to serialize messages to JSON")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output search results
pub fn output_search_results(
    results: &SlackSearchResult,
    format: OutputFormat,
    user_lookup: &HashMap<String, String>,
) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if results.matches.is_empty() {
                println!("No messages found.");
                return Ok(());
            }
            let mut table = new_table(vec!["Channel", "User", "Time", "Message"]);
            for m in &results.matches {
                let time = format_timestamp(&m.ts);
                let user = m.username.as_deref().unwrap_or("-");
                let channel = format_channel_name(&m.channel.name, user_lookup);
                let text = clean_message_text(&m.text, user_lookup);
                table.add_row(vec![
                    Cell::new(&channel).fg(Color::Cyan),
                    Cell::new(user),
                    Cell::new(time),
                    Cell::new(truncate(&text, 50)),
                ]);
            }

            println!("{table}");
            println!(
                "\nShowing {} of {} matches",
                results.matches.len(),
                results.total
            );
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(results)
                .context("Failed to serialize search results to JSON")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output users list
pub fn output_users(users: &[SlackUser], format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Table => {
            if users.is_empty() {
                println!("No users found.");
                return Ok(());
            }
            let mut table = new_table(vec!["Username", "Name", "Timezone"]);
            for user in users {
                let name = user.real_name.as_deref().unwrap_or("-");
                let tz = user.tz.as_deref().unwrap_or("-");
                table.add_row(vec![
                    Cell::new(format!("@{}", user.name)).fg(Color::Cyan),
                    Cell::new(name),
                    Cell::new(tz),
                ]);
            }

            println!("{table}");
            println!("\n{} users", users.len());
        }
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(users).context("Failed to serialize users to JSON")?;
            println!("{json}");
        }
    }
    Ok(())
}

/// Output config status
pub fn output_config_status(
    is_configured: bool,
    has_user_token: bool,
    team_name: Option<&str>,
    default_channel: &str,
) {
    let bot = if is_configured { "Yes" } else { "No" };
    let user = if has_user_token {
        "Yes (search enabled)"
    } else {
        "No (search disabled)"
    };
    println!("Slack Configuration");
    println!("{}", "-".repeat(40));
    println!("Bot token:  {bot}");
    println!("User token: {user}");
    if let Some(name) = team_name {
        println!("Workspace:  {name}");
    }
    if !default_channel.is_empty() {
        println!("Default:    {default_channel}");
    }
}

/// Output config file path
pub fn output_config_path(path: &Path) {
    println!("Config:     {}", path.display());
}

/// Output authentication result
pub fn output_auth_result(result: &AuthResult) {
    match result {
        AuthResult::UserTokenSaved => {
            println!("User token saved successfully!");
            println!("\nYou can now use `hu slack search` command.");
        }
        AuthResult::BotTokenSaved { team_name } => {
            println!("Token saved successfully!");
            println!("Connected to: {}", team_name);
            println!("\nYou can now use `hu slack channels` and other commands.");
        }
        AuthResult::OAuthCompleted { team_name } => {
            println!("\nAuthentication successful!");
            if let Some(team) = team_name {
                println!("Connected to: {}", team);
            }
            println!("\nYou can now use `hu slack channels` and other commands.");
        }
    }
}

/// Output whoami information
pub fn output_whoami(info: &AuthInfo) {
    println!("User ID:   {}", info.user_id);
    println!("User:      {}", info.user);
    println!("Team ID:   {}", info.team_id);
    println!("Team:      {}", info.team);
}

/// Output send message confirmation
pub fn output_send_confirmation(channel: &str, ts: &str) {
    println!("Message sent to {} (ts: {})", channel, ts);
}

/// Output tidy dry run notice
pub fn output_tidy_dry_run() {
    println!("DRY RUN - no channels will be marked as read\n");
}

/// Output individual tidy results (marked/mentioned channels)
pub fn output_tidy_results(results: &[tidy::TidyResult]) {
    for r in results {
        match &r.action {
            tidy::TidyAction::Skipped => {}
            tidy::TidyAction::MarkedRead => {
                println!("Marked read: #{}", r.channel_name);
            }
            tidy::TidyAction::HasMention(mention) => {
                println!("Has mention: #{} - {}", r.channel_name, mention);
            }
        }
    }
}

/// Output tidy summary
pub fn output_tidy_summary(summary: &TidySummary) {
    println!("\nSummary:");
    println!("  Marked as read: {}", summary.marked_read);
    println!("  Has mentions:   {}", summary.has_mentions);
    println!("  Already read:   {}", summary.already_read);
}
