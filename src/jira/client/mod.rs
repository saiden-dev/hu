use anyhow::{bail, Context, Result};
use std::future::Future;

use super::auth;
use super::types::{Issue, IssueUpdate, Transition, User};

#[cfg(test)]
mod tests;

/// Trait for Jira API operations (enables mocking in tests)
pub trait JiraApi: Send + Sync {
    /// Get current authenticated user
    fn get_current_user(&self) -> impl Future<Output = Result<User>> + Send;

    /// Get a single issue by key
    fn get_issue(&self, key: &str) -> impl Future<Output = Result<Issue>> + Send;

    /// Search issues using JQL
    fn search_issues(&self, jql: &str) -> impl Future<Output = Result<Vec<Issue>>> + Send;

    /// Update issue fields
    fn update_issue(
        &self,
        key: &str,
        update: &IssueUpdate,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Get available transitions for an issue
    fn get_transitions(&self, key: &str) -> impl Future<Output = Result<Vec<Transition>>> + Send;

    /// Transition an issue to a new status
    fn transition_issue(
        &self,
        key: &str,
        transition_id: &str,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Jira API client
pub struct JiraClient {
    client: reqwest::Client,
    cloud_id: String,
    access_token: String,
}

impl JiraClient {
    /// Create a new authenticated Jira client
    pub async fn new() -> Result<Self> {
        let access_token = auth::refresh_token_if_needed().await?;
        let creds =
            auth::get_credentials().context("Not authenticated. Run `hu jira auth` first.")?;

        Ok(Self {
            client: reqwest::Client::new(),
            cloud_id: creds.cloud_id,
            access_token,
        })
    }

    /// Build API URL for Jira REST API v3
    fn api_url(&self, path: &str) -> String {
        format!(
            "https://api.atlassian.com/ex/jira/{}/rest/api/3{}",
            self.cloud_id, path
        )
    }
}

impl JiraApi for JiraClient {
    async fn get_current_user(&self) -> Result<User> {
        let url = self.api_url("/myself");
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
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

    async fn get_issue(&self, key: &str) -> Result<Issue> {
        let url = self.api_url(&format!("/issue/{}", key));
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
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

    async fn search_issues(&self, jql: &str) -> Result<Vec<Issue>> {
        // Use the new /search/jql endpoint (the old /search was deprecated)
        let url = self.api_url("/search/jql");
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
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

    async fn update_issue(&self, key: &str, update: &IssueUpdate) -> Result<()> {
        let url = self.api_url(&format!("/issue/{}", key));

        let mut fields = serde_json::Map::new();
        if let Some(summary) = &update.summary {
            fields.insert("summary".to_string(), serde_json::json!(summary));
        }
        if let Some(description) = &update.description {
            // Jira uses ADF format for description
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

        let body = serde_json::json!({ "fields": fields });

        let response = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
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

    async fn get_transitions(&self, key: &str) -> Result<Vec<Transition>> {
        let url = self.api_url(&format!("/issue/{}/transitions", key));
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get transitions")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get transitions for {}: {}", key, error_text);
        }

        let json: serde_json::Value = response.json().await?;
        Ok(parse_transitions(&json))
    }

    async fn transition_issue(&self, key: &str, transition_id: &str) -> Result<()> {
        let url = self.api_url(&format!("/issue/{}/transitions", key));
        let body = serde_json::json!({
            "transition": { "id": transition_id }
        });

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await
            .context("Failed to transition issue")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to transition issue {}: {}", key, error_text);
        }

        Ok(())
    }
}

/// Parse user from JSON (pure function, testable)
pub fn parse_user(json: &serde_json::Value) -> Option<User> {
    Some(User {
        account_id: json["accountId"].as_str()?.to_string(),
        display_name: json["displayName"].as_str()?.to_string(),
        email_address: json["emailAddress"].as_str().map(|s| s.to_string()),
    })
}

/// Parse issues from JSON (pure function, testable)
pub fn parse_issues(json: &serde_json::Value) -> Vec<Issue> {
    json["issues"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(parse_single_issue)
        .collect()
}

/// Parse a single issue from JSON (pure function, testable)
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

/// Extract description text from ADF format
fn extract_description(fields: &serde_json::Value) -> Option<String> {
    // Jira uses Atlassian Document Format (ADF) for rich text
    // For simplicity, extract text nodes recursively
    let description = &fields["description"];
    if description.is_null() {
        return None;
    }

    // If it's a simple string, return it
    if let Some(s) = description.as_str() {
        return Some(s.to_string());
    }

    // Extract text from ADF content
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

/// Extract text from an ADF node recursively
fn extract_text_from_adf_node(node: &serde_json::Value) -> Option<String> {
    // If this node has text, return it
    if let Some(text) = node["text"].as_str() {
        return Some(text.to_string());
    }

    // Otherwise, recursively extract from content
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

/// Parse transitions from JSON (pure function, testable)
pub fn parse_transitions(json: &serde_json::Value) -> Vec<Transition> {
    json["transitions"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| {
            Some(Transition {
                id: t["id"].as_str()?.to_string(),
                name: t["name"].as_str()?.to_string(),
            })
        })
        .collect()
}

/// Build update request body (pure function, testable)
#[cfg(test)]
pub fn build_update_body(update: &IssueUpdate) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    if let Some(summary) = &update.summary {
        fields.insert("summary".to_string(), serde_json::json!(summary));
    }
    if let Some(description) = &update.description {
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
