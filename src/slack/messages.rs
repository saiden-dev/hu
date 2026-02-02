//! Slack message operations
//!
//! Send messages and retrieve message history.

use anyhow::Result;
use serde::Deserialize;

use super::client::SlackClient;
use super::types::SlackMessage;

/// Response from conversations.history API
#[derive(Deserialize)]
struct HistoryResponse {
    messages: Vec<MessageResponse>,
}

/// Response from chat.postMessage API
#[derive(Deserialize)]
struct PostMessageResponse {
    ts: String,
    channel: String,
}

/// Raw message data from API
#[derive(Deserialize)]
struct MessageResponse {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    user: Option<String>,
    text: Option<String>,
    ts: String,
    thread_ts: Option<String>,
    reply_count: Option<u32>,
}

impl From<MessageResponse> for SlackMessage {
    fn from(r: MessageResponse) -> Self {
        Self {
            msg_type: r.msg_type.unwrap_or_else(|| "message".to_string()),
            user: r.user,
            text: r.text.unwrap_or_default(),
            ts: r.ts,
            thread_ts: r.thread_ts,
            reply_count: r.reply_count,
            username: None,
        }
    }
}

/// Get message history for a channel
pub async fn get_history(
    client: &SlackClient,
    channel_id: &str,
    limit: usize,
) -> Result<Vec<SlackMessage>> {
    let limit_str = limit.to_string();
    let response: HistoryResponse = client
        .get_with_params(
            "conversations.history",
            &[("channel", channel_id), ("limit", &limit_str)],
        )
        .await?;

    let messages: Vec<SlackMessage> = response
        .messages
        .into_iter()
        .map(SlackMessage::from)
        .collect();

    Ok(messages)
}

/// Send a message to a channel
pub async fn send_message(
    client: &SlackClient,
    channel_id: &str,
    text: &str,
) -> Result<(String, String), anyhow::Error> {
    let body = serde_json::json!({
        "channel": channel_id,
        "text": text,
    });

    let response: PostMessageResponse = client.post("chat.postMessage", &body).await?;

    Ok((response.channel, response.ts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_response_to_slack_message_full() {
        let response = MessageResponse {
            msg_type: Some("message".to_string()),
            user: Some("U12345".to_string()),
            text: Some("Hello world".to_string()),
            ts: "1704067200.123456".to_string(),
            thread_ts: Some("1704067100.000000".to_string()),
            reply_count: Some(5),
        };

        let message = SlackMessage::from(response);
        assert_eq!(message.msg_type, "message");
        assert_eq!(message.user, Some("U12345".to_string()));
        assert_eq!(message.text, "Hello world");
        assert_eq!(message.ts, "1704067200.123456");
        assert_eq!(message.thread_ts, Some("1704067100.000000".to_string()));
        assert_eq!(message.reply_count, Some(5));
        assert!(message.username.is_none());
    }

    #[test]
    fn test_message_response_to_slack_message_minimal() {
        let response = MessageResponse {
            msg_type: None,
            user: None,
            text: None,
            ts: "1704067200.123456".to_string(),
            thread_ts: None,
            reply_count: None,
        };

        let message = SlackMessage::from(response);
        assert_eq!(message.msg_type, "message"); // default value
        assert!(message.user.is_none());
        assert_eq!(message.text, ""); // default empty
        assert_eq!(message.ts, "1704067200.123456");
        assert!(message.thread_ts.is_none());
        assert!(message.reply_count.is_none());
    }

    #[test]
    fn test_history_response_deserialize() {
        let json = r#"{
            "messages": [
                {"ts": "1704067200.123456", "text": "Hello", "user": "U12345"},
                {"ts": "1704067100.123456"}
            ]
        }"#;

        let response: HistoryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.messages.len(), 2);
        assert_eq!(response.messages[0].ts, "1704067200.123456");
        assert_eq!(response.messages[0].text, Some("Hello".to_string()));
    }

    #[test]
    fn test_post_message_response_deserialize() {
        let json = r#"{"ts": "1704067200.123456", "channel": "C12345"}"#;
        let response: PostMessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.ts, "1704067200.123456");
        assert_eq!(response.channel, "C12345");
    }

    #[test]
    fn test_message_response_deserialize_with_type() {
        let json = r#"{
            "type": "message",
            "user": "U12345",
            "text": "Test message",
            "ts": "1704067200.123456",
            "thread_ts": "1704067100.000000",
            "reply_count": 10
        }"#;

        let response: MessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.msg_type, Some("message".to_string()));
        assert_eq!(response.user, Some("U12345".to_string()));
        assert_eq!(response.text, Some("Test message".to_string()));
        assert_eq!(response.ts, "1704067200.123456");
        assert_eq!(response.thread_ts, Some("1704067100.000000".to_string()));
        assert_eq!(response.reply_count, Some(10));
    }

    #[test]
    fn test_message_response_deserialize_empty_messages() {
        let json = r#"{"messages": []}"#;
        let response: HistoryResponse = serde_json::from_str(json).unwrap();
        assert!(response.messages.is_empty());
    }
}
