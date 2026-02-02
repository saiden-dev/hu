//! Slack tidy operations
//!
//! Mark channels as read if no direct mentions in unread messages.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

use super::client::SlackClient;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_info_creation() {
        let info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };
        assert_eq!(info.user_id, "U12345");
        assert_eq!(info.name, "Alice");
        assert_eq!(info.full_name, "Alice Smith");
    }

    #[test]
    fn test_tidy_result_debug() {
        let result = TidyResult {
            channel_name: "general".to_string(),
            action: TidyAction::MarkedRead,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("general"));
        assert!(debug.contains("MarkedRead"));
    }

    #[test]
    fn test_tidy_action_skipped_debug() {
        let action = TidyAction::Skipped;
        assert_eq!(format!("{:?}", action), "Skipped");
    }

    #[test]
    fn test_tidy_action_marked_read_debug() {
        let action = TidyAction::MarkedRead;
        assert_eq!(format!("{:?}", action), "MarkedRead");
    }

    #[test]
    fn test_tidy_action_has_mention_debug() {
        let action = TidyAction::HasMention("@alice mentioned you".to_string());
        let debug = format!("{:?}", action);
        assert!(debug.contains("HasMention"));
        assert!(debug.contains("@alice mentioned you"));
    }

    #[test]
    fn test_get_display_name_with_name() {
        let channel = ChannelListItem {
            id: "C12345".to_string(),
            name: Some("general".to_string()),
            user: None,
            is_member: Some(true),
            is_im: None,
        };
        assert_eq!(get_display_name(&channel), "general");
    }

    #[test]
    fn test_get_display_name_dm() {
        let channel = ChannelListItem {
            id: "D12345".to_string(),
            name: None,
            user: Some("U67890".to_string()),
            is_member: None,
            is_im: Some(true),
        };
        assert_eq!(get_display_name(&channel), "DM:U67890");
    }

    #[test]
    fn test_get_display_name_fallback_to_id() {
        let channel = ChannelListItem {
            id: "G12345".to_string(),
            name: None,
            user: None,
            is_member: None,
            is_im: None,
        };
        assert_eq!(get_display_name(&channel), "G12345");
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_very_short_max() {
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn test_find_mention_direct_user_mention() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: Some("Hey <@U12345> check this out".to_string()),
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_some());
        assert!(result.unwrap().contains("@mention"));
    }

    #[test]
    fn test_find_mention_name_match() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: Some("Hey Alice, how are you?".to_string()),
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_some());
        assert!(result.unwrap().contains("name 'Alice'"));
    }

    #[test]
    fn test_find_mention_full_name_match() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: Some("I talked to Alice Smith yesterday".to_string()),
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Bob".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_some());
        assert!(result.unwrap().contains("full name"));
    }

    #[test]
    fn test_find_mention_case_insensitive() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: Some("ALICE is here".to_string()),
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_mention_no_match() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: Some("Just a regular message".to_string()),
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_mention_empty_messages() {
        let messages: Vec<HistoryMessage> = vec![];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_mention_message_without_text() {
        let messages = vec![HistoryMessage {
            ts: "1704067200.123456".to_string(),
            text: None,
        }];
        let user_info = UserInfo {
            user_id: "U12345".to_string(),
            name: "Alice".to_string(),
            full_name: "Alice Smith".to_string(),
        };

        let result = find_mention(&messages, &user_info);
        assert!(result.is_none());
    }

    #[test]
    fn test_conversations_list_response_deserialize() {
        let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general", "is_member": true},
                {"id": "D67890", "user": "U99999", "is_im": true}
            ],
            "response_metadata": {"next_cursor": "abc123"}
        }"#;

        let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.channels.len(), 2);
        assert_eq!(response.channels[0].id, "C12345");
        assert_eq!(response.channels[1].user, Some("U99999".to_string()));
    }

    #[test]
    fn test_channel_list_item_deserialize() {
        let json = r#"{"id": "C12345", "name": "test", "is_member": true, "is_im": false}"#;
        let item: ChannelListItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "C12345");
        assert_eq!(item.name, Some("test".to_string()));
        assert_eq!(item.is_member, Some(true));
        assert_eq!(item.is_im, Some(false));
    }

    #[test]
    fn test_conversations_info_response_deserialize() {
        let json = r#"{
            "channel": {
                "last_read": "1704067200.000000",
                "latest": {"ts": "1704067300.000000"}
            }
        }"#;

        let response: ConversationsInfoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.channel.last_read,
            Some("1704067200.000000".to_string())
        );
        assert_eq!(response.channel.latest.unwrap().ts, "1704067300.000000");
    }

    #[test]
    fn test_history_response_deserialize() {
        let json = r#"{
            "messages": [
                {"ts": "1704067200.123456", "text": "Hello"},
                {"ts": "1704067100.123456"}
            ]
        }"#;

        let response: HistoryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.messages.len(), 2);
        assert_eq!(response.messages[0].ts, "1704067200.123456");
        assert_eq!(response.messages[0].text, Some("Hello".to_string()));
    }

    #[test]
    fn test_mark_request_serialize() {
        let request = MarkRequest {
            channel: "C12345".to_string(),
            ts: "1704067200.123456".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("C12345"));
        assert!(json.contains("1704067200.123456"));
    }

    #[test]
    fn test_mark_response_deserialize() {
        let json = r#"{}"#;
        let response: MarkResponse = serde_json::from_str(json).unwrap();
        // Just verify it deserializes without error
        let _ = response;
    }

    #[test]
    fn test_response_metadata_deserialize() {
        let json = r#"{"next_cursor": "cursor123"}"#;
        let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.next_cursor, Some("cursor123".to_string()));
    }

    #[test]
    fn test_channel_info_item_deserialize() {
        let json = r#"{"last_read": "1704067200.000000"}"#;
        let item: ChannelInfoItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.last_read, Some("1704067200.000000".to_string()));
        assert!(item.latest.is_none());
    }

    #[test]
    fn test_latest_message_deserialize() {
        let json = r#"{"ts": "1704067200.123456"}"#;
        let latest: LatestMessage = serde_json::from_str(json).unwrap();
        assert_eq!(latest.ts, "1704067200.123456");
    }
}
