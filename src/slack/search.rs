//! Slack message search
//!
//! Search messages across channels.

use anyhow::Result;
use serde::Deserialize;

use super::client::SlackClient;
use super::types::{SlackSearchChannel, SlackSearchMatch, SlackSearchResult};

/// Response from search.messages API
#[derive(Deserialize)]
struct SearchResponse {
    messages: MessagesContainer,
}

/// Container for search matches
#[derive(Deserialize)]
struct MessagesContainer {
    total: u32,
    matches: Vec<MatchResponse>,
}

/// Raw match data from API
#[derive(Deserialize)]
struct MatchResponse {
    channel: ChannelResponse,
    user: Option<String>,
    username: Option<String>,
    text: String,
    ts: String,
    permalink: Option<String>,
}

/// Channel info in search response
#[derive(Deserialize)]
struct ChannelResponse {
    id: String,
    name: String,
}

impl From<MatchResponse> for SlackSearchMatch {
    fn from(r: MatchResponse) -> Self {
        Self {
            channel: SlackSearchChannel {
                id: r.channel.id,
                name: r.channel.name,
            },
            user: r.user,
            username: r.username,
            text: r.text,
            ts: r.ts,
            permalink: r.permalink,
        }
    }
}

/// Search messages across the workspace (requires user token)
pub async fn search_messages(
    client: &SlackClient,
    query: &str,
    count: usize,
) -> Result<SlackSearchResult> {
    let count_str = count.to_string();
    let response: SearchResponse = client
        .get_with_user_token(
            "search.messages",
            &[
                ("query", query),
                ("count", &count_str),
                ("sort", "timestamp"),
                ("sort_dir", "desc"),
            ],
        )
        .await?;

    Ok(SlackSearchResult {
        total: response.messages.total,
        matches: response
            .messages
            .matches
            .into_iter()
            .map(SlackSearchMatch::from)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_response_to_slack_search_match_full() {
        let response = MatchResponse {
            channel: ChannelResponse {
                id: "C12345".to_string(),
                name: "general".to_string(),
            },
            user: Some("U12345".to_string()),
            username: Some("alice".to_string()),
            text: "Hello world".to_string(),
            ts: "1704067200.123456".to_string(),
            permalink: Some("https://slack.com/archives/C12345/p1704067200123456".to_string()),
        };

        let match_result = SlackSearchMatch::from(response);
        assert_eq!(match_result.channel.id, "C12345");
        assert_eq!(match_result.channel.name, "general");
        assert_eq!(match_result.user, Some("U12345".to_string()));
        assert_eq!(match_result.username, Some("alice".to_string()));
        assert_eq!(match_result.text, "Hello world");
        assert_eq!(match_result.ts, "1704067200.123456");
        assert!(match_result.permalink.is_some());
    }

    #[test]
    fn test_match_response_to_slack_search_match_minimal() {
        let response = MatchResponse {
            channel: ChannelResponse {
                id: "C12345".to_string(),
                name: "general".to_string(),
            },
            user: None,
            username: None,
            text: "Message".to_string(),
            ts: "1704067200.123456".to_string(),
            permalink: None,
        };

        let match_result = SlackSearchMatch::from(response);
        assert_eq!(match_result.channel.id, "C12345");
        assert!(match_result.user.is_none());
        assert!(match_result.username.is_none());
        assert!(match_result.permalink.is_none());
    }

    #[test]
    fn test_search_response_deserialize() {
        let json = r#"{
            "messages": {
                "total": 42,
                "matches": [
                    {
                        "channel": {"id": "C12345", "name": "general"},
                        "user": "U12345",
                        "username": "alice",
                        "text": "Hello",
                        "ts": "1704067200.123456",
                        "permalink": "https://slack.com/..."
                    }
                ]
            }
        }"#;

        let response: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.messages.total, 42);
        assert_eq!(response.messages.matches.len(), 1);
        assert_eq!(response.messages.matches[0].text, "Hello");
    }

    #[test]
    fn test_search_response_empty_matches() {
        let json = r#"{
            "messages": {
                "total": 0,
                "matches": []
            }
        }"#;

        let response: SearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.messages.total, 0);
        assert!(response.messages.matches.is_empty());
    }

    #[test]
    fn test_messages_container_deserialize() {
        let json = r#"{"total": 100, "matches": []}"#;
        let container: MessagesContainer = serde_json::from_str(json).unwrap();
        assert_eq!(container.total, 100);
        assert!(container.matches.is_empty());
    }

    #[test]
    fn test_channel_response_deserialize() {
        let json = r#"{"id": "C12345", "name": "test-channel"}"#;
        let channel: ChannelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(channel.id, "C12345");
        assert_eq!(channel.name, "test-channel");
    }
}
