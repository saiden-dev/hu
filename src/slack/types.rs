//! Slack data types and structures

use serde::{Deserialize, Serialize};

/// Slack channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackChannel {
    /// Channel ID (e.g., "C12345678")
    pub id: String,
    /// Channel name (without #)
    pub name: String,
    /// Whether this is a private channel
    pub is_private: bool,
    /// Whether the bot is a member of this channel
    pub is_member: bool,
    /// Channel topic
    pub topic: Option<String>,
    /// Channel purpose
    pub purpose: Option<String>,
    /// Number of members
    pub num_members: Option<u32>,
    /// Creation timestamp
    pub created: i64,
}

/// Slack message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    /// Message type (usually "message")
    #[serde(rename = "type")]
    pub msg_type: String,
    /// User ID who sent the message
    pub user: Option<String>,
    /// Message text
    pub text: String,
    /// Timestamp (unique ID for the message)
    pub ts: String,
    /// Thread timestamp (if this is a reply)
    pub thread_ts: Option<String>,
    /// Number of replies in thread
    pub reply_count: Option<u32>,
    /// User display name (enriched after fetch)
    #[serde(skip_deserializing)]
    pub username: Option<String>,
}

/// Slack user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackUser {
    /// User ID
    pub id: String,
    /// Team ID
    pub team_id: Option<String>,
    /// Username (handle without @)
    pub name: String,
    /// Display name
    pub real_name: Option<String>,
    /// Whether this is a bot
    pub is_bot: bool,
    /// Whether this user is deleted
    pub deleted: bool,
    /// User's timezone
    pub tz: Option<String>,
}

/// Search result match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSearchMatch {
    /// Channel where the message was posted
    pub channel: SlackSearchChannel,
    /// User ID who posted
    pub user: Option<String>,
    /// Username who posted
    pub username: Option<String>,
    /// Message text
    pub text: String,
    /// Timestamp
    pub ts: String,
    /// Permalink to the message
    pub permalink: Option<String>,
}

/// Channel info in search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSearchChannel {
    /// Channel ID
    pub id: String,
    /// Channel name
    pub name: String,
}

/// Search results container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSearchResult {
    /// Total matches found
    pub total: u32,
    /// Matches returned
    pub matches: Vec<SlackSearchMatch>,
}

/// Output format for Slack commands
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format for scripting
    Json,
}

/// Authenticated user info returned from auth.test
#[derive(Debug, Clone)]
pub struct AuthInfo {
    /// User ID (e.g., "U04H482TK6Z")
    pub user_id: String,
    /// Username
    pub user: String,
    /// Team/workspace ID
    pub team_id: String,
    /// Team/workspace name
    pub team: String,
}

/// Result of an auth operation
#[derive(Debug, Clone)]
pub enum AuthResult {
    /// Bot token was saved
    BotTokenSaved { team_name: String },
    /// User token was saved
    UserTokenSaved,
    /// OAuth flow completed
    OAuthCompleted { team_name: Option<String> },
}

/// Summary of a tidy operation
#[derive(Debug, Clone)]
pub struct TidySummary {
    /// Number of channels marked as read
    pub marked_read: usize,
    /// Number of channels with mentions (skipped)
    pub has_mentions: usize,
    /// Number of channels already read (skipped)
    pub already_read: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert!(matches!(format, OutputFormat::Table));
    }

    #[test]
    fn test_output_format_clone() {
        let format = OutputFormat::Json;
        let cloned = format.clone();
        assert!(matches!(cloned, OutputFormat::Json));
    }

    #[test]
    fn test_output_format_debug() {
        let format = OutputFormat::Table;
        let debug = format!("{:?}", format);
        assert_eq!(debug, "Table");
    }

    #[test]
    fn test_slack_channel_debug() {
        let channel = SlackChannel {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: false,
            is_member: true,
            topic: Some("Test topic".to_string()),
            purpose: None,
            num_members: Some(100),
            created: 1704067200,
        };
        let debug = format!("{:?}", channel);
        assert!(debug.contains("SlackChannel"));
        assert!(debug.contains("general"));
    }

    #[test]
    fn test_slack_channel_clone() {
        let channel = SlackChannel {
            id: "C12345".to_string(),
            name: "general".to_string(),
            is_private: false,
            is_member: true,
            topic: None,
            purpose: None,
            num_members: None,
            created: 1704067200,
        };
        let cloned = channel.clone();
        assert_eq!(cloned.id, channel.id);
        assert_eq!(cloned.name, channel.name);
    }

    #[test]
    fn test_slack_message_debug() {
        let msg = SlackMessage {
            msg_type: "message".to_string(),
            user: Some("U12345".to_string()),
            text: "Hello world".to_string(),
            ts: "1704067200.123456".to_string(),
            thread_ts: None,
            reply_count: Some(5),
            username: None,
        };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("SlackMessage"));
    }

    #[test]
    fn test_slack_user_debug() {
        let user = SlackUser {
            id: "U12345".to_string(),
            team_id: Some("T12345".to_string()),
            name: "alice".to_string(),
            real_name: Some("Alice Smith".to_string()),
            is_bot: false,
            deleted: false,
            tz: Some("America/New_York".to_string()),
        };
        let debug = format!("{:?}", user);
        assert!(debug.contains("SlackUser"));
    }

    #[test]
    fn test_slack_search_result_debug() {
        let result = SlackSearchResult {
            total: 42,
            matches: vec![],
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("SlackSearchResult"));
        assert!(debug.contains("42"));
    }

    #[test]
    fn test_slack_search_match_debug() {
        let m = SlackSearchMatch {
            channel: SlackSearchChannel {
                id: "C12345".to_string(),
                name: "general".to_string(),
            },
            user: Some("U12345".to_string()),
            username: Some("alice".to_string()),
            text: "Hello".to_string(),
            ts: "1704067200.123456".to_string(),
            permalink: Some("https://slack.com/...".to_string()),
        };
        let debug = format!("{:?}", m);
        assert!(debug.contains("SlackSearchMatch"));
    }

    #[test]
    fn test_slack_search_channel_clone() {
        let channel = SlackSearchChannel {
            id: "C12345".to_string(),
            name: "general".to_string(),
        };
        let cloned = channel.clone();
        assert_eq!(cloned.id, channel.id);
    }

    #[test]
    fn test_auth_info_debug() {
        let info = AuthInfo {
            user_id: "U12345".to_string(),
            user: "alice".to_string(),
            team_id: "T12345".to_string(),
            team: "Acme Corp".to_string(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("AuthInfo"));
        assert!(debug.contains("alice"));
    }

    #[test]
    fn test_auth_info_clone() {
        let info = AuthInfo {
            user_id: "U12345".to_string(),
            user: "alice".to_string(),
            team_id: "T12345".to_string(),
            team: "Acme Corp".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(cloned.user_id, "U12345");
        assert_eq!(cloned.team, "Acme Corp");
    }

    #[test]
    fn test_auth_result_debug() {
        let result = AuthResult::BotTokenSaved {
            team_name: "Acme".to_string(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("BotTokenSaved"));

        let result = AuthResult::UserTokenSaved;
        let debug = format!("{:?}", result);
        assert!(debug.contains("UserTokenSaved"));

        let result = AuthResult::OAuthCompleted {
            team_name: Some("Team".to_string()),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("OAuthCompleted"));
    }

    #[test]
    fn test_auth_result_clone() {
        let result = AuthResult::BotTokenSaved {
            team_name: "Acme".to_string(),
        };
        let cloned = result.clone();
        assert!(matches!(cloned, AuthResult::BotTokenSaved { .. }));
    }

    #[test]
    fn test_tidy_summary_debug() {
        let summary = TidySummary {
            marked_read: 5,
            has_mentions: 2,
            already_read: 10,
        };
        let debug = format!("{:?}", summary);
        assert!(debug.contains("TidySummary"));
    }

    #[test]
    fn test_tidy_summary_clone() {
        let summary = TidySummary {
            marked_read: 5,
            has_mentions: 2,
            already_read: 10,
        };
        let cloned = summary.clone();
        assert_eq!(cloned.marked_read, 5);
        assert_eq!(cloned.has_mentions, 2);
        assert_eq!(cloned.already_read, 10);
    }
}
