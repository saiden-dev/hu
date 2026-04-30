//! Jira REST API client.
//!
//! Module layout:
//! - [`JiraApi`] — trait for mockable client operations
//! - [`JiraClient`] — concrete OAuth-backed implementation
//! - [`issues`] — `/myself`, `/issue/{key}`, `/search/jql`, PUT `/issue/{key}` + parsers
//! - [`transitions`] — `/issue/{key}/transitions` GET/POST + parser

use anyhow::{bail, Context, Result};
use std::future::Future;

use super::auth;
use super::types::{Comment, Issue, IssueUpdate, Transition, User};

mod comments;
mod issues;
mod transitions;

#[cfg(test)]
mod tests;

/// Trait for Jira API operations (enables mocking in tests).
pub trait JiraApi: Send + Sync {
    /// Get current authenticated user.
    fn get_current_user(&self) -> impl Future<Output = Result<User>> + Send;

    /// Get a single issue by key.
    fn get_issue(&self, key: &str) -> impl Future<Output = Result<Issue>> + Send;

    /// Search issues using JQL.
    fn search_issues(&self, jql: &str) -> impl Future<Output = Result<Vec<Issue>>> + Send;

    /// Update issue fields.
    fn update_issue(
        &self,
        key: &str,
        update: &IssueUpdate,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Get available transitions for an issue.
    fn get_transitions(&self, key: &str) -> impl Future<Output = Result<Vec<Transition>>> + Send;

    /// Transition an issue to a new status.
    fn transition_issue(
        &self,
        key: &str,
        transition_id: &str,
    ) -> impl Future<Output = Result<()>> + Send;

    /// List all comments on an issue, ordered as Jira returns them
    /// (oldest first).
    #[allow(dead_code)] // handler lands in chunk 3.B
    fn list_comments(&self, key: &str) -> impl Future<Output = Result<Vec<Comment>>> + Send;
}

/// Jira API client.
pub struct JiraClient {
    /// Underlying HTTP client. `pub(super)` so submodules can issue requests.
    pub(super) http: reqwest::Client,
    /// Cloud ID for the authenticated tenant.
    pub(super) cloud_id: String,
    /// OAuth access token (refreshed on `new()`).
    pub(super) access_token: String,
}

impl JiraClient {
    /// Create a new authenticated Jira client.
    pub async fn new() -> Result<Self> {
        let access_token = auth::refresh_token_if_needed().await?;
        let creds =
            auth::get_credentials().context("Not authenticated. Run `hu jira auth` first.")?;

        Ok(Self {
            http: reqwest::Client::new(),
            cloud_id: creds.cloud_id,
            access_token,
        })
    }

    /// Build API URL for Jira REST API v3.
    pub(super) fn api_url(&self, path: &str) -> String {
        format!(
            "https://api.atlassian.com/ex/jira/{}/rest/api/3{}",
            self.cloud_id, path
        )
    }

    /// List all Jira fields (to discover custom field IDs).
    pub async fn list_fields(&self) -> Result<Vec<serde_json::Value>> {
        let url = self.api_url("/field");
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Failed to list fields")?;
        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            bail!("Failed to list fields: {err}");
        }
        let json: serde_json::Value = response.json().await?;
        Ok(json.as_array().cloned().unwrap_or_default())
    }

    /// Raw search returning the full JSON response (for custom fields).
    pub async fn search_raw(
        &self,
        jql: &str,
        fields: &[&str],
        max_results: usize,
    ) -> Result<serde_json::Value> {
        let url = self.api_url("/search/jql");
        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({
                "jql": jql,
                "fields": fields,
                "maxResults": max_results,
            }))
            .send()
            .await
            .context("Failed to search")?;
        if !response.status().is_success() {
            let err = response.text().await.unwrap_or_default();
            bail!("Search failed: {err}");
        }
        Ok(response.json().await?)
    }
}

impl JiraApi for JiraClient {
    async fn get_current_user(&self) -> Result<User> {
        issues::get_current_user(self).await
    }

    async fn get_issue(&self, key: &str) -> Result<Issue> {
        issues::get_issue(self, key).await
    }

    async fn search_issues(&self, jql: &str) -> Result<Vec<Issue>> {
        issues::search_issues(self, jql).await
    }

    async fn update_issue(&self, key: &str, update: &IssueUpdate) -> Result<()> {
        issues::update_issue(self, key, update).await
    }

    async fn get_transitions(&self, key: &str) -> Result<Vec<Transition>> {
        transitions::get_transitions(self, key).await
    }

    async fn transition_issue(&self, key: &str, transition_id: &str) -> Result<()> {
        transitions::transition_issue(self, key, transition_id).await
    }

    async fn list_comments(&self, key: &str) -> Result<Vec<Comment>> {
        comments::list_comments(self, key).await
    }
}
