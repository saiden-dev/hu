use anyhow::{bail, Context, Result};
use std::future::Future;

use super::auth;
use super::types::{Board, Issue, IssueUpdate, Sprint, Transition, User};

/// Trait for Jira API operations (enables mocking in tests)
pub trait JiraApi: Send + Sync {
    /// Get current authenticated user
    fn get_current_user(&self) -> impl Future<Output = Result<User>> + Send;

    /// Get all boards
    fn get_boards(&self) -> impl Future<Output = Result<Vec<Board>>> + Send;

    /// Get active sprint for a board
    fn get_active_sprint(
        &self,
        board_id: u64,
    ) -> impl Future<Output = Result<Option<Sprint>>> + Send;

    /// Get issues in a sprint
    fn get_sprint_issues(&self, sprint_id: u64) -> impl Future<Output = Result<Vec<Issue>>> + Send;

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

    /// Build API URL for Jira Agile API v1
    fn agile_url(&self, path: &str) -> String {
        format!(
            "https://api.atlassian.com/ex/jira/{}/rest/agile/1.0{}",
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

    async fn get_boards(&self) -> Result<Vec<Board>> {
        let url = self.agile_url("/board");
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get boards")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get boards: {}", error_text);
        }

        let json: serde_json::Value = response.json().await?;
        Ok(parse_boards(&json))
    }

    async fn get_active_sprint(&self, board_id: u64) -> Result<Option<Sprint>> {
        let url = self.agile_url(&format!("/board/{}/sprint?state=active", board_id));
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get active sprint")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get active sprint: {}", error_text);
        }

        let json: serde_json::Value = response.json().await?;
        Ok(parse_sprints(&json).into_iter().next())
    }

    async fn get_sprint_issues(&self, sprint_id: u64) -> Result<Vec<Issue>> {
        let url = self.agile_url(&format!("/sprint/{}/issue", sprint_id));
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to get sprint issues")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            bail!("Failed to get sprint issues: {}", error_text);
        }

        let json: serde_json::Value = response.json().await?;
        Ok(parse_issues(&json))
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
        let url = self.api_url("/search");
        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[("jql", jql)])
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

/// Parse boards from JSON (pure function, testable)
pub fn parse_boards(json: &serde_json::Value) -> Vec<Board> {
    json["values"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|b| {
            Some(Board {
                id: b["id"].as_u64()?,
                name: b["name"].as_str()?.to_string(),
                board_type: b["type"].as_str()?.to_string(),
            })
        })
        .collect()
}

/// Parse sprints from JSON (pure function, testable)
pub fn parse_sprints(json: &serde_json::Value) -> Vec<Sprint> {
    json["values"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|s| {
            Some(Sprint {
                id: s["id"].as_u64()?,
                name: s["name"].as_str()?.to_string(),
                state: s["state"].as_str()?.to_string(),
                start_date: s["startDate"].as_str().map(|d| d.to_string()),
                end_date: s["endDate"].as_str().map(|d| d.to_string()),
            })
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_user_extracts_fields() {
        let json = json!({
            "accountId": "123",
            "displayName": "John Doe",
            "emailAddress": "john@example.com"
        });
        let user = parse_user(&json).unwrap();
        assert_eq!(user.account_id, "123");
        assert_eq!(user.display_name, "John Doe");
        assert_eq!(user.email_address, Some("john@example.com".to_string()));
    }

    #[test]
    fn parse_user_without_email() {
        let json = json!({
            "accountId": "456",
            "displayName": "Jane"
        });
        let user = parse_user(&json).unwrap();
        assert_eq!(user.account_id, "456");
        assert!(user.email_address.is_none());
    }

    #[test]
    fn parse_user_returns_none_for_missing_fields() {
        let json = json!({
            "displayName": "Missing ID"
        });
        let user = parse_user(&json);
        assert!(user.is_none());
    }

    #[test]
    fn parse_boards_extracts_boards() {
        let json = json!({
            "values": [
                {"id": 1, "name": "Board 1", "type": "scrum"},
                {"id": 2, "name": "Board 2", "type": "kanban"}
            ]
        });
        let boards = parse_boards(&json);
        assert_eq!(boards.len(), 2);
        assert_eq!(boards[0].id, 1);
        assert_eq!(boards[0].name, "Board 1");
        assert_eq!(boards[0].board_type, "scrum");
        assert_eq!(boards[1].id, 2);
        assert_eq!(boards[1].board_type, "kanban");
    }

    #[test]
    fn parse_boards_handles_empty_values() {
        let json = json!({"values": []});
        let boards = parse_boards(&json);
        assert!(boards.is_empty());
    }

    #[test]
    fn parse_boards_handles_missing_values() {
        let json = json!({});
        let boards = parse_boards(&json);
        assert!(boards.is_empty());
    }

    #[test]
    fn parse_boards_skips_incomplete_entries() {
        let json = json!({
            "values": [
                {"id": 1, "name": "Complete", "type": "scrum"},
                {"id": 2, "name": "Missing Type"},
                {"id": 3, "type": "kanban"}
            ]
        });
        let boards = parse_boards(&json);
        assert_eq!(boards.len(), 1);
        assert_eq!(boards[0].name, "Complete");
    }

    #[test]
    fn parse_sprints_extracts_sprints() {
        let json = json!({
            "values": [
                {
                    "id": 100,
                    "name": "Sprint 1",
                    "state": "active",
                    "startDate": "2024-01-01",
                    "endDate": "2024-01-14"
                }
            ]
        });
        let sprints = parse_sprints(&json);
        assert_eq!(sprints.len(), 1);
        assert_eq!(sprints[0].id, 100);
        assert_eq!(sprints[0].name, "Sprint 1");
        assert_eq!(sprints[0].state, "active");
        assert_eq!(sprints[0].start_date, Some("2024-01-01".to_string()));
    }

    #[test]
    fn parse_sprints_handles_missing_dates() {
        let json = json!({
            "values": [{
                "id": 200,
                "name": "Future Sprint",
                "state": "future"
            }]
        });
        let sprints = parse_sprints(&json);
        assert_eq!(sprints.len(), 1);
        assert!(sprints[0].start_date.is_none());
        assert!(sprints[0].end_date.is_none());
    }

    #[test]
    fn parse_sprints_handles_empty() {
        let json = json!({"values": []});
        let sprints = parse_sprints(&json);
        assert!(sprints.is_empty());
    }

    #[test]
    fn parse_issues_extracts_issues() {
        let json = json!({
            "issues": [{
                "key": "PROJ-123",
                "fields": {
                    "summary": "Fix bug",
                    "status": {"name": "In Progress"},
                    "issuetype": {"name": "Bug"},
                    "assignee": {"displayName": "John"},
                    "updated": "2024-01-15T10:00:00Z"
                }
            }]
        });
        let issues = parse_issues(&json);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].key, "PROJ-123");
        assert_eq!(issues[0].summary, "Fix bug");
        assert_eq!(issues[0].status, "In Progress");
        assert_eq!(issues[0].issue_type, "Bug");
        assert_eq!(issues[0].assignee, Some("John".to_string()));
    }

    #[test]
    fn parse_issues_handles_unassigned() {
        let json = json!({
            "issues": [{
                "key": "PROJ-456",
                "fields": {
                    "summary": "Task",
                    "status": {"name": "Open"},
                    "issuetype": {"name": "Task"},
                    "assignee": null,
                    "updated": "2024-01-15T12:00:00Z"
                }
            }]
        });
        let issues = parse_issues(&json);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].assignee.is_none());
    }

    #[test]
    fn parse_issues_handles_empty() {
        let json = json!({"issues": []});
        let issues = parse_issues(&json);
        assert!(issues.is_empty());
    }

    #[test]
    fn parse_single_issue_extracts_fields() {
        let json = json!({
            "key": "TEST-1",
            "fields": {
                "summary": "Test issue",
                "status": {"name": "Done"},
                "issuetype": {"name": "Story"},
                "assignee": {"displayName": "Tester"},
                "description": {
                    "type": "doc",
                    "content": [{
                        "type": "paragraph",
                        "content": [{"type": "text", "text": "Description text"}]
                    }]
                },
                "updated": "2024-01-01T00:00:00Z"
            }
        });
        let issue = parse_single_issue(&json).unwrap();
        assert_eq!(issue.key, "TEST-1");
        assert_eq!(issue.summary, "Test issue");
        assert_eq!(issue.status, "Done");
        assert_eq!(issue.issue_type, "Story");
        assert_eq!(issue.assignee, Some("Tester".to_string()));
        assert_eq!(issue.description, Some("Description text".to_string()));
    }

    #[test]
    fn parse_single_issue_returns_none_for_missing_key() {
        let json = json!({
            "fields": {
                "summary": "No key",
                "status": {"name": "Open"},
                "issuetype": {"name": "Task"},
                "updated": "2024-01-01T00:00:00Z"
            }
        });
        let issue = parse_single_issue(&json);
        assert!(issue.is_none());
    }

    #[test]
    fn parse_single_issue_handles_null_description() {
        let json = json!({
            "key": "X-1",
            "fields": {
                "summary": "S",
                "status": {"name": "Open"},
                "issuetype": {"name": "Task"},
                "description": null,
                "updated": "2024-01-01T00:00:00Z"
            }
        });
        let issue = parse_single_issue(&json).unwrap();
        assert!(issue.description.is_none());
    }

    #[test]
    fn extract_description_handles_string() {
        let fields = json!({"description": "Simple string"});
        let desc = extract_description(&fields);
        assert_eq!(desc, Some("Simple string".to_string()));
    }

    #[test]
    fn extract_description_handles_adf() {
        let fields = json!({
            "description": {
                "type": "doc",
                "content": [{
                    "type": "paragraph",
                    "content": [
                        {"type": "text", "text": "Hello "},
                        {"type": "text", "text": "world"}
                    ]
                }]
            }
        });
        let desc = extract_description(&fields);
        assert_eq!(desc, Some("Hello world".to_string()));
    }

    #[test]
    fn extract_description_handles_null() {
        let fields = json!({"description": null});
        let desc = extract_description(&fields);
        assert!(desc.is_none());
    }

    #[test]
    fn extract_description_handles_empty_content() {
        let fields = json!({
            "description": {
                "type": "doc",
                "content": []
            }
        });
        let desc = extract_description(&fields);
        assert!(desc.is_none());
    }

    #[test]
    fn extract_text_from_adf_node_gets_text() {
        let node = json!({"type": "text", "text": "Hello"});
        let text = extract_text_from_adf_node(&node);
        assert_eq!(text, Some("Hello".to_string()));
    }

    #[test]
    fn extract_text_from_adf_node_recurses() {
        let node = json!({
            "type": "paragraph",
            "content": [
                {"type": "text", "text": "A"},
                {"type": "text", "text": "B"}
            ]
        });
        let text = extract_text_from_adf_node(&node);
        assert_eq!(text, Some("AB".to_string()));
    }

    #[test]
    fn extract_text_from_adf_node_handles_no_content() {
        let node = json!({"type": "hardBreak"});
        let text = extract_text_from_adf_node(&node);
        assert!(text.is_none());
    }

    #[test]
    fn parse_transitions_extracts_transitions() {
        let json = json!({
            "transitions": [
                {"id": "11", "name": "To Do"},
                {"id": "21", "name": "In Progress"},
                {"id": "31", "name": "Done"}
            ]
        });
        let transitions = parse_transitions(&json);
        assert_eq!(transitions.len(), 3);
        assert_eq!(transitions[0].id, "11");
        assert_eq!(transitions[0].name, "To Do");
        assert_eq!(transitions[2].id, "31");
        assert_eq!(transitions[2].name, "Done");
    }

    #[test]
    fn parse_transitions_handles_empty() {
        let json = json!({"transitions": []});
        let transitions = parse_transitions(&json);
        assert!(transitions.is_empty());
    }

    #[test]
    fn parse_transitions_handles_missing() {
        let json = json!({});
        let transitions = parse_transitions(&json);
        assert!(transitions.is_empty());
    }

    #[test]
    fn build_update_body_with_summary() {
        let update = IssueUpdate {
            summary: Some("New summary".to_string()),
            description: None,
            assignee: None,
        };
        let body = build_update_body(&update);
        assert_eq!(body["fields"]["summary"], "New summary");
    }

    #[test]
    fn build_update_body_with_description() {
        let update = IssueUpdate {
            summary: None,
            description: Some("New description".to_string()),
            assignee: None,
        };
        let body = build_update_body(&update);
        assert_eq!(body["fields"]["description"]["type"], "doc");
        assert_eq!(body["fields"]["description"]["version"], 1);
    }

    #[test]
    fn build_update_body_with_assignee() {
        let update = IssueUpdate {
            summary: None,
            description: None,
            assignee: Some("user123".to_string()),
        };
        let body = build_update_body(&update);
        assert_eq!(body["fields"]["assignee"]["accountId"], "user123");
    }

    #[test]
    fn build_update_body_with_all_fields() {
        let update = IssueUpdate {
            summary: Some("Sum".to_string()),
            description: Some("Desc".to_string()),
            assignee: Some("user".to_string()),
        };
        let body = build_update_body(&update);
        assert_eq!(body["fields"]["summary"], "Sum");
        assert!(body["fields"]["description"].is_object());
        assert_eq!(body["fields"]["assignee"]["accountId"], "user");
    }

    #[test]
    fn build_update_body_empty() {
        let update = IssueUpdate::default();
        let body = build_update_body(&update);
        assert_eq!(body["fields"], json!({}));
    }
}
