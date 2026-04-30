//! Transition-related Jira API operations.
//!
//! Endpoints: `GET /issue/{key}/transitions`, `POST /issue/{key}/transitions`.

use anyhow::{bail, Context, Result};

use super::JiraClient;
use crate::jira::types::Transition;

/// Get available transitions for an issue.
pub(super) async fn get_transitions(client: &JiraClient, key: &str) -> Result<Vec<Transition>> {
    let url = client.api_url(&format!("/issue/{}/transitions", key));
    let response = client
        .http
        .get(&url)
        .bearer_auth(&client.access_token)
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

/// Transition an issue to a new status.
pub(super) async fn transition_issue(
    client: &JiraClient,
    key: &str,
    transition_id: &str,
) -> Result<()> {
    let url = client.api_url(&format!("/issue/{}/transitions", key));
    let body = serde_json::json!({
        "transition": { "id": transition_id }
    });

    let response = client
        .http
        .post(&url)
        .bearer_auth(&client.access_token)
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

/// Parse transitions from JSON (pure function, testable).
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
