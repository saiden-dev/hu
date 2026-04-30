//! Comment-related Jira API operations.
//!
//! Endpoints: `GET /issue/{key}/comment`.

// First production caller lands in chunk 3.B (handler + CLI subcommand).
// Tests inside this module exercise the parsers; the trait wiring is
// already proven by the test build.
#![allow(dead_code)]

use anyhow::{bail, Context, Result};

use super::JiraClient;
use crate::jira::adf;
use crate::jira::types::{Comment, User};

/// List comments on an issue. Returns them in the order Jira sends them
/// (oldest first by default).
pub(super) async fn list_comments(client: &JiraClient, key: &str) -> Result<Vec<Comment>> {
    // Bump the page size to 100 — Jira's default is 50 and most issues
    // we deal with have fewer than that. Real pagination can come later.
    let url = client.api_url(&format!("/issue/{}/comment?maxResults=100", key));
    let response = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .context("Failed to list comments")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to list comments for {}: {}", key, error_text);
    }

    let json: serde_json::Value = response.json().await?;
    Ok(parse_comments(&json))
}

/// Parse the comment-list response (pure function, testable).
pub fn parse_comments(json: &serde_json::Value) -> Vec<Comment> {
    json["comments"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(parse_single_comment)
        .collect()
}

/// Parse a single comment object.
pub fn parse_single_comment(json: &serde_json::Value) -> Option<Comment> {
    let body_adf = json["body"].clone();
    let body = adf::adf_to_plain_text(&body_adf);

    Some(Comment {
        id: json["id"].as_str()?.to_string(),
        author: parse_author(&json["author"])?,
        body,
        body_adf,
        created: json["created"].as_str().unwrap_or_default().to_string(),
        updated: json["updated"].as_str().unwrap_or_default().to_string(),
    })
}

/// Parse a comment author. Comments may be authored by users without
/// emails (system accounts), so we don't require it.
fn parse_author(json: &serde_json::Value) -> Option<User> {
    Some(User {
        account_id: json["accountId"].as_str()?.to_string(),
        display_name: json["displayName"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string(),
        email_address: json["emailAddress"].as_str().map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_comments_extracts_list() {
        let json = json!({
            "startAt": 0,
            "maxResults": 50,
            "total": 2,
            "comments": [
                {
                    "id": "1",
                    "author": {"accountId": "u1", "displayName": "Alice"},
                    "body": {
                        "type": "doc",
                        "version": 1,
                        "content": [{
                            "type": "paragraph",
                            "content": [{"type": "text", "text": "first"}]
                        }]
                    },
                    "created": "2026-04-30T10:00:00.000Z",
                    "updated": "2026-04-30T10:00:00.000Z",
                },
                {
                    "id": "2",
                    "author": {"accountId": "u2", "displayName": "Bob"},
                    "body": {
                        "type": "doc",
                        "version": 1,
                        "content": [{
                            "type": "paragraph",
                            "content": [{"type": "text", "text": "second"}]
                        }]
                    },
                    "created": "2026-04-30T11:00:00.000Z",
                    "updated": "2026-04-30T11:00:00.000Z",
                }
            ]
        });
        let comments = parse_comments(&json);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].id, "1");
        assert_eq!(comments[0].author.display_name, "Alice");
        assert_eq!(comments[0].body, "first");
        assert_eq!(comments[1].body, "second");
    }

    #[test]
    fn parse_comments_handles_empty_list() {
        let json = json!({"comments": []});
        let comments = parse_comments(&json);
        assert!(comments.is_empty());
    }

    #[test]
    fn parse_comments_handles_missing_field() {
        let json = json!({});
        let comments = parse_comments(&json);
        assert!(comments.is_empty());
    }

    #[test]
    fn parse_single_comment_renders_body_to_text() {
        let json = json!({
            "id": "10",
            "author": {"accountId": "u", "displayName": "User"},
            "body": {
                "type": "doc",
                "version": 1,
                "content": [
                    {"type": "paragraph", "content": [{"type": "text", "text": "line 1"}]},
                    {"type": "paragraph", "content": [{"type": "text", "text": "line 2"}]}
                ]
            },
            "created": "2026-04-30T10:00:00.000Z",
            "updated": "2026-04-30T10:00:00.000Z"
        });
        let comment = parse_single_comment(&json).unwrap();
        assert_eq!(comment.body, "line 1\nline 2");
        assert_eq!(comment.body_adf["type"], "doc");
    }

    #[test]
    fn parse_single_comment_returns_none_without_id() {
        let json = json!({
            "author": {"accountId": "u", "displayName": "User"},
            "body": {"type": "doc", "version": 1, "content": []}
        });
        assert!(parse_single_comment(&json).is_none());
    }

    #[test]
    fn parse_single_comment_returns_none_without_author_id() {
        let json = json!({
            "id": "10",
            "author": {"displayName": "Anonymous"},
            "body": {"type": "doc", "version": 1, "content": []}
        });
        assert!(parse_single_comment(&json).is_none());
    }

    #[test]
    fn parse_single_comment_handles_missing_timestamps() {
        let json = json!({
            "id": "10",
            "author": {"accountId": "u", "displayName": "User"},
            "body": {"type": "doc", "version": 1, "content": []}
        });
        let comment = parse_single_comment(&json).unwrap();
        assert_eq!(comment.created, "");
        assert_eq!(comment.updated, "");
    }

    #[test]
    fn parse_single_comment_falls_back_for_missing_display_name() {
        let json = json!({
            "id": "10",
            "author": {"accountId": "system"},
            "body": {"type": "doc", "version": 1, "content": []}
        });
        let comment = parse_single_comment(&json).unwrap();
        assert_eq!(comment.author.display_name, "Unknown");
    }
}
