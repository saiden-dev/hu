//! Issue-creation Jira API operations.
//!
//! Endpoints:
//! - `POST /issue` — create a new issue
//! - `GET /issue/createmeta/{projectIdOrKey}/issuetypes` — list issue
//!   types available on a project. The `?projectKeys=` flavour was
//!   deprecated by Atlassian in 2024; we use the new path-style endpoint.

use anyhow::{bail, Context, Result};

use super::JiraClient;
use crate::jira::adf;
use crate::jira::types::{CreatedIssue, IssueCreate, IssueType};

/// Build the human-facing browse URL for a freshly created issue.
fn browse_url(client: &JiraClient, key: &str) -> String {
    let base = client.site_url.trim_end_matches('/');
    format!("{}/browse/{}", base, key)
}

/// `POST /issue` with the supplied fields.
pub(super) async fn create_issue(client: &JiraClient, new: &IssueCreate) -> Result<CreatedIssue> {
    let url = client.api_url("/issue");
    let body = build_create_body(new);

    let response = client
        .http
        .post(&url)
        .bearer_auth(&client.access_token)
        .json(&body)
        .send()
        .await
        .context("Failed to create issue")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        bail!("Failed to create issue ({}): {}", status, error_text);
    }

    let json: serde_json::Value = response.json().await?;
    let key = json["key"]
        .as_str()
        .context("Create response missing 'key'")?
        .to_string();
    let id = json["id"]
        .as_str()
        .context("Create response missing 'id'")?
        .to_string();
    let url = browse_url(client, &key);

    Ok(CreatedIssue { id, key, url })
}

/// Build the `POST /issue` request body. Pure function for testability.
pub fn build_create_body(new: &IssueCreate) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    fields.insert(
        "project".to_string(),
        serde_json::json!({ "key": new.project_key }),
    );
    fields.insert("summary".to_string(), serde_json::json!(new.summary));
    fields.insert(
        "issuetype".to_string(),
        serde_json::json!({ "name": new.issue_type }),
    );

    // Same Markdown-vs-ADF precedence as IssueUpdate.
    if let Some(adf_doc) = &new.description_adf {
        fields.insert("description".to_string(), adf_doc.clone());
    } else if let Some(md) = &new.description {
        fields.insert("description".to_string(), adf::markdown_to_adf(md));
    }

    if let Some(account_id) = &new.assignee {
        fields.insert(
            "assignee".to_string(),
            serde_json::json!({ "accountId": account_id }),
        );
    }

    serde_json::json!({ "fields": fields })
}

/// `GET /issue/createmeta/{projectKey}/issuetypes`. Returns at most the
/// first page (50 by default, raised here) — projects don't typically
/// have more than a handful of issue types.
pub(super) async fn get_issue_types(
    client: &JiraClient,
    project_key: &str,
) -> Result<Vec<IssueType>> {
    let url = client.api_url(&format!(
        "/issue/createmeta/{}/issuetypes?maxResults=100",
        project_key
    ));
    let response = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
        .send()
        .await
        .context("Failed to fetch issue-type metadata")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        bail!(
            "Failed to fetch issue types for project {} ({}): {}",
            project_key,
            status,
            error_text
        );
    }

    let json: serde_json::Value = response.json().await?;
    Ok(parse_issue_types(&json))
}

/// Parse the `/issuetypes` response (pure function, testable).
pub fn parse_issue_types(json: &serde_json::Value) -> Vec<IssueType> {
    json["issueTypes"]
        .as_array()
        .or_else(|| json["values"].as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(parse_single_issue_type)
        .collect()
}

fn parse_single_issue_type(json: &serde_json::Value) -> Option<IssueType> {
    Some(IssueType {
        id: json["id"].as_str()?.to_string(),
        name: json["name"].as_str()?.to_string(),
        description: json["description"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_create_body_includes_required_fields() {
        let new = IssueCreate {
            project_key: "HU".to_string(),
            summary: "S".to_string(),
            issue_type: "Task".to_string(),
            ..Default::default()
        };
        let body = build_create_body(&new);
        assert_eq!(body["fields"]["project"]["key"], "HU");
        assert_eq!(body["fields"]["summary"], "S");
        assert_eq!(body["fields"]["issuetype"]["name"], "Task");
    }

    #[test]
    fn build_create_body_renders_markdown_description() {
        let new = IssueCreate {
            project_key: "HU".to_string(),
            summary: "S".to_string(),
            issue_type: "Task".to_string(),
            description: Some("# heading".to_string()),
            ..Default::default()
        };
        let body = build_create_body(&new);
        assert_eq!(
            body["fields"]["description"]["content"][0]["type"],
            "heading"
        );
    }

    #[test]
    fn build_create_body_prefers_adf_over_markdown() {
        let raw = json!({
            "type": "doc",
            "version": 1,
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": "raw"}]
            }]
        });
        let new = IssueCreate {
            project_key: "HU".to_string(),
            summary: "S".to_string(),
            issue_type: "Task".to_string(),
            description: Some("# IGNORED markdown".to_string()),
            description_adf: Some(raw),
            ..Default::default()
        };
        let body = build_create_body(&new);
        assert_eq!(
            body["fields"]["description"]["content"][0]["content"][0]["text"],
            "raw"
        );
    }

    #[test]
    fn build_create_body_assignee() {
        let new = IssueCreate {
            project_key: "HU".to_string(),
            summary: "S".to_string(),
            issue_type: "Task".to_string(),
            assignee: Some("user-123".to_string()),
            ..Default::default()
        };
        let body = build_create_body(&new);
        assert_eq!(body["fields"]["assignee"]["accountId"], "user-123");
    }

    #[test]
    fn parse_issue_types_extracts_list() {
        let json = json!({
            "issueTypes": [
                {"id": "10001", "name": "Task", "description": "Work"},
                {"id": "10002", "name": "Bug", "description": ""},
                {"id": "10003", "name": "Story"}
            ]
        });
        let types = parse_issue_types(&json);
        assert_eq!(types.len(), 3);
        assert_eq!(types[0].name, "Task");
        assert_eq!(types[0].description.as_deref(), Some("Work"));
        // Empty description normalised to None.
        assert!(types[1].description.is_none());
        // Missing description treated the same way.
        assert!(types[2].description.is_none());
    }

    #[test]
    fn parse_issue_types_falls_back_to_values_key() {
        // Atlassian flips between `issueTypes` and `values` depending on
        // which version of the meta endpoint replies — accept both.
        let json = json!({
            "values": [{"id": "1", "name": "Task"}]
        });
        let types = parse_issue_types(&json);
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].name, "Task");
    }

    #[test]
    fn parse_issue_types_handles_empty() {
        let json = json!({"issueTypes": []});
        assert!(parse_issue_types(&json).is_empty());
    }

    #[test]
    fn parse_issue_types_handles_missing() {
        assert!(parse_issue_types(&json!({})).is_empty());
    }

    #[test]
    fn parse_single_issue_type_requires_id_and_name() {
        assert!(parse_single_issue_type(&json!({"name": "Task"})).is_none());
        assert!(parse_single_issue_type(&json!({"id": "1"})).is_none());
    }
}
