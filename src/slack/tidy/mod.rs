//! Slack tidy operations
//!
//! Mark channels as read if no direct mentions in unread messages.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use super::client::SlackClient;

#[cfg(test)]
mod tests;

/// User info for mention detection
pub struct UserInfo {
    pub user_id: String,
    pub name: String,
    pub full_name: String,
}

/// Channel with unread info
struct ChannelUnreadInfo {
    last_read: String,
    has_unreads: bool,
}

/// Response from conversations.list with membership info
#[derive(Deserialize)]
struct ConversationsListResponse {
    channels: Vec<ChannelListItem>,
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Deserialize)]
struct ChannelListItem {
    id: String,
    name: Option<String>,
    user: Option<String>, // For DMs, contains the other user's ID
    is_member: Option<bool>,
    is_im: Option<bool>,
}

#[derive(Deserialize)]
struct ResponseMetadata {
    next_cursor: Option<String>,
}

/// Response from conversations.info
#[derive(Deserialize)]
struct ConversationsInfoResponse {
    channel: ChannelInfoItem,
}

#[derive(Deserialize)]
struct ChannelInfoItem {
    last_read: Option<String>,
    latest: Option<LatestMessage>,
}

#[derive(Deserialize)]
struct LatestMessage {
    ts: String,
}

/// Response from conversations.history
#[derive(Deserialize)]
struct HistoryResponse {
    messages: Vec<HistoryMessage>,
}

#[derive(Deserialize)]
struct HistoryMessage {
    ts: String,
    text: Option<String>,
}

/// Request body for conversations.mark
#[derive(Serialize)]
struct MarkRequest {
    channel: String,
    ts: String,
}

/// Empty response from conversations.mark
#[derive(Deserialize)]
struct MarkResponse {}

/// Result of tidy operation for a single channel
#[derive(Debug)]
pub struct TidyResult {
    pub channel_name: String,
    pub action: TidyAction,
}

#[derive(Debug)]
pub enum TidyAction {
    Skipped,            // No unreads
    MarkedRead,         // Marked as read (no mentions)
    HasMention(String), // Has mention, not marked
}

/// Run tidy operation on all channels
pub async fn tidy_channels(
    client: &SlackClient,
    user_info: &UserInfo,
    dry_run: bool,
) -> Result<Vec<TidyResult>> {
    let mut results = Vec::new();

    // Get channels user is member of
    let channels = list_member_channels(client).await?;
    println!("Found {} channels you're a member of", channels.len());

    for channel in channels {
        let display_name = get_display_name(&channel);

        // Rate limit
        sleep(Duration::from_millis(500)).await;

        // Get channel info with last_read
        let info = get_channel_unread_info(client, &channel.id).await?;

        if !info.has_unreads {
            results.push(TidyResult {
                channel_name: display_name,
                action: TidyAction::Skipped,
            });
            continue;
        }

        // Get unread messages
        sleep(Duration::from_millis(500)).await;
        let messages = get_messages_since(client, &channel.id, &info.last_read).await?;

        // Check for mentions
        if let Some(mention) = find_mention(&messages, user_info) {
            results.push(TidyResult {
                channel_name: display_name,
                action: TidyAction::HasMention(mention),
            });
            continue;
        }

        // No mentions - mark as read
        if !dry_run {
            if let Some(latest_ts) = messages.first().map(|m| m.ts.as_str()) {
                sleep(Duration::from_millis(500)).await;
                mark_channel_read(client, &channel.id, latest_ts).await?;
            }
        }

        results.push(TidyResult {
            channel_name: display_name,
            action: TidyAction::MarkedRead,
        });
    }

    Ok(results)
}

/// Get display name for a channel/DM
fn get_display_name(channel: &ChannelListItem) -> String {
    if let Some(ref name) = channel.name {
        name.clone()
    } else if let Some(ref user_id) = channel.user {
        // DM - show user ID (ideally we'd look up the name, but this works for now)
        format!("DM:{}", user_id)
    } else {
        channel.id.clone()
    }
}

/// List channels where user is a member
async fn list_member_channels(client: &SlackClient) -> Result<Vec<ChannelListItem>> {
    let mut all_channels = Vec::new();
    let mut cursor: Option<String> = None;
    let mut first = true;

    loop {
        if !first {
            sleep(Duration::from_millis(500)).await;
        }
        first = false;

        let mut params = vec![
            ("types", "public_channel,private_channel,mpim,im"),
            ("exclude_archived", "true"),
            ("limit", "200"),
        ];

        let cursor_str;
        if let Some(ref c) = cursor {
            cursor_str = c.clone();
            params.push(("cursor", &cursor_str));
        }

        let response: ConversationsListResponse = client
            .get_with_user_token("conversations.list", &params)
            .await?;

        for ch in response.channels {
            // DMs (is_im) don't have is_member field - user is implicitly a member
            let is_member = ch.is_im.unwrap_or(false) || ch.is_member.unwrap_or(false);
            if is_member {
                all_channels.push(ch);
            }
        }

        match response.response_metadata.and_then(|m| m.next_cursor) {
            Some(c) if !c.is_empty() => cursor = Some(c),
            _ => break,
        }
    }

    Ok(all_channels)
}

/// Get channel info to determine if there are unreads
async fn get_channel_unread_info(
    client: &SlackClient,
    channel_id: &str,
) -> Result<ChannelUnreadInfo> {
    let response: ConversationsInfoResponse = client
        .get_with_user_token("conversations.info", &[("channel", channel_id)])
        .await?;

    let last_read = response.channel.last_read.unwrap_or_default();
    let latest_ts = response.channel.latest.map(|l| l.ts).unwrap_or_default();

    // Has unreads if latest message ts > last_read ts
    let has_unreads = !last_read.is_empty() && !latest_ts.is_empty() && latest_ts > last_read;

    Ok(ChannelUnreadInfo {
        last_read,
        has_unreads,
    })
}

/// Get messages since last_read timestamp
async fn get_messages_since(
    client: &SlackClient,
    channel_id: &str,
    oldest: &str,
) -> Result<Vec<HistoryMessage>> {
    let response: HistoryResponse = client
        .get_with_user_token(
            "conversations.history",
            &[
                ("channel", channel_id),
                ("oldest", oldest),
                ("limit", "100"),
            ],
        )
        .await?;

    Ok(response.messages)
}

/// Check if any message contains a mention of the user
fn find_mention(messages: &[HistoryMessage], user_info: &UserInfo) -> Option<String> {
    let user_mention = format!("<@{}>", user_info.user_id);
    let name_lower = user_info.name.to_lowercase();
    let full_name_lower = user_info.full_name.to_lowercase();

    for msg in messages {
        if let Some(ref text) = msg.text {
            // Check direct mention
            if text.contains(&user_mention) {
                return Some(format!("@mention: {}", truncate(text, 50)));
            }

            // Check name (case-insensitive)
            let text_lower = text.to_lowercase();
            if text_lower.contains(&name_lower) {
                return Some(format!("name '{}': {}", user_info.name, truncate(text, 50)));
            }

            // Check full name (case-insensitive)
            if text_lower.contains(&full_name_lower) {
                return Some(format!("full name: {}", truncate(text, 50)));
            }
        }
    }

    None
}

/// Mark a channel as read at the given timestamp
async fn mark_channel_read(client: &SlackClient, channel_id: &str, ts: &str) -> Result<()> {
    let body = MarkRequest {
        channel: channel_id.to_string(),
        ts: ts.to_string(),
    };

    let _: MarkResponse = client
        .post_with_user_token("conversations.mark", &body)
        .await?;
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}
