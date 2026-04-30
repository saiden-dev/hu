//! Issue-related Jira API operations.
//!
//! Endpoints: `/myself`, `/issue/{key}`, `/search/jql`, PUT `/issue/{key}`.
//! Pure parsers live alongside their endpoints for cohesion.

use anyhow::{bail, Context, Result};

use super::JiraClient;
use crate::jira::types::{Issue, IssueUpdate, User};

/// Get current authenticated user.
pub(super) async fn get_current_user(client: &JiraClient) -> Result<User> {
    let url = client.api_url("/myself");
    let response = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .context("Failed to get current user")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to get current user: {}", error_text);
    }

    let json: serde_json::Value = response.json().await?;
    parse_user(&json).context("Failed to parse user response")
}

/// Get a single issue by key.
pub(super) async fn get_issue(client: &JiraClient, key: &str) -> Result<Issue> {
    let url = client.api_url(&format!("/issue/{}", key));
    let response = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .context("Failed to get issue")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to get issue {}: {}", key, error_text);
    }

    let json: serde_json::Value = response.json().await?;
    parse_single_issue(&json).context("Failed to parse issue")
}

/// Search issues using JQL via the modern `/search/jql` endpoint.
pub(super) async fn search_issues(client: &JiraClient, jql: &str) -> Result<Vec<Issue>> {
    let url = client.api_url("/search/jql");
    let response = client
        .http
        .post(&url)
        .bearer_auth(&client.access_token)
        .json(&serde_json::json!({
            "jql": jql,
            "fields": ["summary", "status", "issuetype", "assignee", "description", "updated"]
        }))
        .send()
        .await
        .context("Failed to search issues")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to search issues: {}", error_text);
    }

    let json: serde_json::Value = response.json().await?;
    Ok(parse_issues(&json))
}

/// Update issue fields (summary, description, assignee).
pub(super) async fn update_issue(
    client: &JiraClient,
    key: &str,
    update: &IssueUpdate,
) -> Result<()> {
    let url = client.api_url(&format!("/issue/{}", key));
    let body = build_update_body(update);

    let response = client
        .http
        .put(&url)
        .bearer_auth(&client.access_token)
        .json(&body)
        .send()
        .await
        .context("Failed to update issue")?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to update issue {}: {}", key, error_text);
    }

    Ok(())
}

/// Parse user from JSON (pure function, testable).
pub fn parse_user(json: &serde_json::Value) -> Option<User> {
    Some(User {
        account_id: json["accountId"].as_str()?.to_string(),
        display_name: json["displayName"].as_str()?.to_string(),
        email_address: json["emailAddress"].as_str().map(|s| s.to_string()),
    })
}

/// Parse issues from JSON (pure function, testable).
pub fn parse_issues(json: &serde_json::Value) -> Vec<Issue> {
    json["issues"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(parse_single_issue)
        .collect()
}

/// Parse a single issue from JSON (pure function, testable).
pub fn parse_single_issue(json: &serde_json::Value) -> Option<Issue> {
    let fields = &json["fields"];
    Some(Issue {
        key: json["key"].as_str()?.to_string(),
        summary: fields["summary"].as_str()?.to_string(),
        status: fields["status"]["name"].as_str()?.to_string(),
        issue_type: fields["issuetype"]["name"].as_str()?.to_string(),
        assignee: fields["assignee"]["displayName"]
            .as_str()
            .map(|s| s.to_string()),
        description: extract_description(fields),
        updated: fields["updated"].as_str()?.to_string(),
    })
}

/// Extract description text from ADF or string format.
pub(crate) fn extract_description(fields: &serde_json::Value) -> Option<String> {
    let description = &fields["description"];
    if description.is_null() {
        return None;
    }

    if let Some(s) = description.as_str() {
        return Some(s.to_string());
    }

    let content = description["content"].as_array()?;
    let text: Vec<String> = content
        .iter()
        .filter_map(extract_text_from_adf_node)
        .collect();

    if text.is_empty() {
        None
    } else {
        Some(text.join("\n"))
    }
}

/// Extract text from an ADF node recursively.
pub(crate) fn extract_text_from_adf_node(node: &serde_json::Value) -> Option<String> {
    if let Some(text) = node["text"].as_str() {
        return Some(text.to_string());
    }

    let content = node["content"].as_array()?;
    let texts: Vec<String> = content
        .iter()
        .filter_map(extract_text_from_adf_node)
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join(""))
    }
}

/// Build update request body (pure function, testable).
pub fn build_update_body(update: &IssueUpdate) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    if let Some(summary) = &update.summary {
        fields.insert("summary".to_string(), serde_json::json!(summary));
    }
    if let Some(description) = &update.description {
        // Jira uses ADF format for description.
        // TODO(phase-2): replace with markdown_to_adf(description).
        fields.insert(
            "description".to_string(),
            serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": description
                    }]
                }]
            }),
        );
    }
    if let Some(assignee) = &update.assignee {
        fields.insert(
            "assignee".to_string(),
            serde_json::json!({ "accountId": assignee }),
        );
    }

    serde_json::json!({ "fields": fields })
}
