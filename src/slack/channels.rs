//! Slack channel operations
//!
//! List channels, get channel info, and resolve channel names to IDs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;

use super::client::SlackClient;
use super::config::config_path;
use super::types::{SlackChannel, SlackUser};

/// Cache expiry time (1 hour)
const CACHE_EXPIRY_SECS: u64 = 3600;

/// Cached user lookup data
#[derive(Serialize, Deserialize)]
struct UserCache {
    /// Timestamp when cache was created
    created: u64,
    /// User ID to username mapping
    users: HashMap<String, String>,
}

/// Get path to user cache file
fn user_cache_path() -> Option<PathBuf> {
    config_path().map(|p| p.with_file_name("slack_users_cache.json"))
}

/// Response from conversations.list API
#[derive(Deserialize)]
struct ConversationsListResponse {
    channels: Vec<ChannelResponse>,
    response_metadata: Option<ResponseMetadata>,
}

/// Response from conversations.info API
#[derive(Deserialize)]
struct ConversationsInfoResponse {
    channel: ChannelResponse,
}

/// Response from users.list API
#[derive(Deserialize)]
struct UsersListResponse {
    members: Vec<UserResponse>,
}

/// Raw channel data from API
#[derive(Deserialize)]
struct ChannelResponse {
    id: String,
    name: String,
    is_private: Option<bool>,
    is_member: Option<bool>,
    topic: Option<TopicResponse>,
    purpose: Option<TopicResponse>,
    num_members: Option<u32>,
    created: Option<i64>,
}

/// Raw user data from API
#[derive(Deserialize)]
struct UserResponse {
    id: String,
    team_id: Option<String>,
    name: String,
    real_name: Option<String>,
    is_bot: Option<bool>,
    deleted: Option<bool>,
    tz: Option<String>,
}

/// Topic or purpose field
#[derive(Deserialize)]
struct TopicResponse {
    value: String,
}

/// Pagination metadata
#[derive(Deserialize)]
struct ResponseMetadata {
    next_cursor: Option<String>,
}

impl From<ChannelResponse> for SlackChannel {
    fn from(r: ChannelResponse) -> Self {
        Self {
            id: r.id,
            name: r.name,
            is_private: r.is_private.unwrap_or(false),
            is_member: r.is_member.unwrap_or(false),
            topic: r.topic.map(|t| t.value).filter(|s| !s.is_empty()),
            purpose: r.purpose.map(|p| p.value).filter(|s| !s.is_empty()),
            num_members: r.num_members,
            created: r.created.unwrap_or(0),
        }
    }
}

impl From<UserResponse> for SlackUser {
    fn from(r: UserResponse) -> Self {
        Self {
            id: r.id,
            team_id: r.team_id,
            name: r.name,
            real_name: r.real_name,
            is_bot: r.is_bot.unwrap_or(false),
            deleted: r.deleted.unwrap_or(false),
            tz: r.tz,
        }
    }
}

/// List all accessible channels
pub async fn list_channels(client: &SlackClient) -> Result<Vec<SlackChannel>> {
    let mut all_channels = Vec::new();
    let mut cursor: Option<String> = None;
    let mut first_request = true;

    loop {
        // Rate limit: delay between paginated requests (Tier 2 = ~20 req/min)
        if !first_request {
            sleep(Duration::from_millis(500)).await;
        }
        first_request = false;

        let mut params = vec![
            ("types", "public_channel"),
            ("exclude_archived", "true"),
            ("limit", "200"),
        ];

        let cursor_str;
        if let Some(ref c) = cursor {
            cursor_str = c.clone();
            params.push(("cursor", &cursor_str));
        }

        let response: ConversationsListResponse = client
            .get_with_params("conversations.list", &params)
            .await?;

        all_channels.extend(response.channels.into_iter().map(SlackChannel::from));

        // Check for more pages
        match response.response_metadata.and_then(|m| m.next_cursor) {
            Some(c) if !c.is_empty() => cursor = Some(c),
            _ => break,
        }
    }

    // Sort by name
    all_channels.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(all_channels)
}

/// Get detailed info for a specific channel
pub async fn get_channel_info(client: &SlackClient, channel_id: &str) -> Result<SlackChannel> {
    let response: ConversationsInfoResponse = client
        .get_with_params("conversations.info", &[("channel", channel_id)])
        .await?;

    Ok(SlackChannel::from(response.channel))
}

/// Resolve a channel name (with or without #) to a channel ID
pub async fn resolve_channel(client: &SlackClient, name_or_id: &str) -> Result<String> {
    // If it already looks like an ID (channel, group, DM, or user), return it
    // C = public channel, G = private channel, D = DM, U = user (for DM)
    if name_or_id.starts_with('C')
        || name_or_id.starts_with('G')
        || name_or_id.starts_with('D')
        || name_or_id.starts_with('U')
    {
        return Ok(name_or_id.to_string());
    }

    // Strip leading # if present
    let name = name_or_id.trim_start_matches('#');

    // List channels and find by name
    let channels = list_channels(client).await?;
    channels
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Channel not found: {}", name))
}

/// List all users in the workspace
pub async fn list_users(client: &SlackClient) -> Result<Vec<SlackUser>> {
    let response: UsersListResponse = client.get("users.list").await?;

    let users: Vec<SlackUser> = response
        .members
        .into_iter()
        .map(SlackUser::from)
        .filter(|u| !u.deleted && !u.is_bot)
        .collect();

    Ok(users)
}

/// Build a lookup map from user ID to username (with caching)
pub async fn build_user_lookup(client: &SlackClient) -> Result<HashMap<String, String>> {
    // Try to load from cache first
    if let Some(cached) = load_user_cache() {
        return Ok(cached);
    }

    // Fetch from API
    let users = list_users(client).await?;
    let lookup: HashMap<String, String> = users.into_iter().map(|u| (u.id, u.name)).collect();

    // Save to cache
    save_user_cache(&lookup);

    Ok(lookup)
}

/// Load user cache if valid
fn load_user_cache() -> Option<HashMap<String, String>> {
    let path = user_cache_path()?;
    let contents = fs::read_to_string(&path).ok()?;
    let cache: UserCache = serde_json::from_str(&contents).ok()?;

    // Check if cache is expired
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if now - cache.created > CACHE_EXPIRY_SECS {
        return None;
    }

    Some(cache.users)
}

/// Save user lookup to cache
fn save_user_cache(users: &HashMap<String, String>) {
    let Some(path) = user_cache_path() else {
        return;
    };

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache = UserCache {
        created: now,
        users: users.clone(),
    };

    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = fs::write(&path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_response_to_slack_channel_full() {
        let response = ChannelResponse {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: Some(true),
            is_member: Some(true),
            topic: Some(TopicResponse {
                value: "Channel topic".to_string(),
            }),
            purpose: Some(TopicResponse {
                value: "Channel purpose".to_string(),
            }),
            num_members: Some(42),
            created: Some(1704067200),
        };

        let channel = SlackChannel::from(response);
        assert_eq!(channel.id, "C12345");
        assert_eq!(channel.name, "general");
        assert!(channel.is_private);
        assert!(channel.is_member);
        assert_eq!(channel.topic, Some("Channel topic".to_string()));
        assert_eq!(channel.purpose, Some("Channel purpose".to_string()));
        assert_eq!(channel.num_members, Some(42));
        assert_eq!(channel.created, 1704067200);
    }

    #[test]
    fn test_channel_response_to_slack_channel_minimal() {
        let response = ChannelResponse {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: None,
            is_member: None,
            topic: None,
            purpose: None,
            num_members: None,
            created: None,
        };

        let channel = SlackChannel::from(response);
        assert_eq!(channel.id, "C12345");
        assert_eq!(channel.name, "general");
        assert!(!channel.is_private);
        assert!(!channel.is_member);
        assert!(channel.topic.is_none());
        assert!(channel.purpose.is_none());
        assert!(channel.num_members.is_none());
        assert_eq!(channel.created, 0);
    }

    #[test]
    fn test_channel_response_empty_topic_filtered() {
        let response = ChannelResponse {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: None,
            is_member: None,
            topic: Some(TopicResponse {
                value: "".to_string(),
            }),
            purpose: Some(TopicResponse {
                value: "".to_string(),
            }),
            num_members: None,
            created: None,
        };

        let channel = SlackChannel::from(response);
        assert!(channel.topic.is_none());
        assert!(channel.purpose.is_none());
    }

    #[test]
    fn test_user_response_to_slack_user_full() {
        let response = UserResponse {
            id: "U12345".to_string(),
            team_id: Some("T12345".to_string()),
            name: "alice".to_string(),
            real_name: Some("Alice Smith".to_string()),
            is_bot: Some(false),
            deleted: Some(false),
            tz: Some("America/New_York".to_string()),
        };

        let user = SlackUser::from(response);
        assert_eq!(user.id, "U12345");
        assert_eq!(user.team_id, Some("T12345".to_string()));
        assert_eq!(user.name, "alice");
        assert_eq!(user.real_name, Some("Alice Smith".to_string()));
        assert!(!user.is_bot);
        assert!(!user.deleted);
        assert_eq!(user.tz, Some("America/New_York".to_string()));
    }

    #[test]
    fn test_user_response_to_slack_user_minimal() {
        let response = UserResponse {
            id: "U12345".to_string(),
            team_id: None,
            name: "alice".to_string(),
            real_name: None,
            is_bot: None,
            deleted: None,
            tz: None,
        };

        let user = SlackUser::from(response);
        assert_eq!(user.id, "U12345");
        assert!(user.team_id.is_none());
        assert_eq!(user.name, "alice");
        assert!(user.real_name.is_none());
        assert!(!user.is_bot);
        assert!(!user.deleted);
        assert!(user.tz.is_none());
    }

    #[test]
    fn test_user_response_to_slack_user_bot() {
        let response = UserResponse {
            id: "U12345".to_string(),
            team_id: None,
            name: "bot".to_string(),
            real_name: None,
            is_bot: Some(true),
            deleted: Some(true),
            tz: None,
        };

        let user = SlackUser::from(response);
        assert!(user.is_bot);
        assert!(user.deleted);
    }

    #[test]
    fn test_user_cache_serialize_deserialize() {
        let mut users = HashMap::new();
        users.insert("U12345".to_string(), "alice".to_string());
        users.insert("U67890".to_string(), "bob".to_string());

        let cache = UserCache {
            created: 1704067200,
            users,
        };

        let json = serde_json::to_string(&cache).unwrap();
        let deserialized: UserCache = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.created, 1704067200);
        assert_eq!(deserialized.users.len(), 2);
        assert_eq!(deserialized.users.get("U12345"), Some(&"alice".to_string()));
    }

    #[test]
    fn test_user_cache_path_is_some() {
        // Should return Some on systems with a home directory
        let path = user_cache_path();
        if let Some(p) = path {
            assert!(p.to_string_lossy().contains("slack_users_cache.json"));
        }
    }

    #[test]
    fn test_conversations_list_response_deserialize() {
        let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general", "is_private": false, "is_member": true}
            ],
            "response_metadata": {"next_cursor": "abc123"}
        }"#;

        let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.channels.len(), 1);
        assert_eq!(response.channels[0].id, "C12345");
        assert_eq!(
            response.response_metadata.unwrap().next_cursor,
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_conversations_list_response_no_cursor() {
        let json = r#"{
            "channels": [
                {"id": "C12345", "name": "general"}
            ]
        }"#;

        let response: ConversationsListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.channels.len(), 1);
        assert!(response.response_metadata.is_none());
    }

    #[test]
    fn test_conversations_info_response_deserialize() {
        let json = r#"{
            "channel": {
                "id": "C12345",
                "name": "general",
                "is_private": true,
                "is_member": true,
                "topic": {"value": "Discussion"},
                "purpose": {"value": "General chat"},
                "num_members": 100,
                "created": 1704067200
            }
        }"#;

        let response: ConversationsInfoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.channel.id, "C12345");
        assert_eq!(response.channel.name, "general");
    }

    #[test]
    fn test_users_list_response_deserialize() {
        let json = r#"{
            "members": [
                {"id": "U12345", "name": "alice", "real_name": "Alice"},
                {"id": "U67890", "name": "bob"}
            ]
        }"#;

        let response: UsersListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.members.len(), 2);
        assert_eq!(response.members[0].id, "U12345");
        assert_eq!(response.members[1].name, "bob");
    }

    #[test]
    fn test_topic_response_deserialize() {
        let json = r#"{"value": "Test topic"}"#;
        let topic: TopicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(topic.value, "Test topic");
    }

    #[test]
    fn test_response_metadata_deserialize() {
        let json = r#"{"next_cursor": "cursor123"}"#;
        let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(meta.next_cursor, Some("cursor123".to_string()));
    }

    #[test]
    fn test_response_metadata_empty_cursor() {
        let json = r#"{}"#;
        let meta: ResponseMetadata = serde_json::from_str(json).unwrap();
        assert!(meta.next_cursor.is_none());
    }
}
