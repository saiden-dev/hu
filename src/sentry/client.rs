//! Sentry HTTP client

use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::sleep;

use super::config::{load_config, SentryConfig};
use super::types::{Event, Issue};

const SENTRY_API_URL: &str = "https://sentry.io/api/0";
const MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_SECS: u64 = 5;

/// Sentry API client
pub struct SentryClient {
    config: SentryConfig,
    http: Client,
}

impl SentryClient {
    /// Create a new Sentry client
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let http = Client::builder().user_agent("hu-cli/0.1.0").build()?;
        Ok(Self { config, http })
    }

    /// Get config reference
    pub fn config(&self) -> &SentryConfig {
        &self.config
    }

    /// Get auth token
    fn auth_token(&self) -> Result<&str> {
        self.config
            .auth_token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Sentry auth_token not configured"))
    }

    /// Get organization slug
    fn organization(&self) -> Result<&str> {
        self.config
            .organization
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Sentry organization not configured"))
    }

    /// List issues for organization
    pub async fn list_issues(&self, query: Option<&str>, limit: usize) -> Result<Vec<Issue>> {
        let org = self.organization()?;
        let url = format!("{}/organizations/{}/issues/", SENTRY_API_URL, org);

        let mut params = vec![("limit", limit.to_string())];
        if let Some(q) = query {
            params.push(("query", q.to_string()));
        }

        self.get_with_params(&url, &params).await
    }

    /// List issues for a specific project
    pub async fn list_project_issues(
        &self,
        project: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Issue>> {
        let org = self.organization()?;
        let url = format!("{}/projects/{}/{}/issues/", SENTRY_API_URL, org, project);

        let mut params = vec![("limit", limit.to_string())];
        if let Some(q) = query {
            params.push(("query", q.to_string()));
        }

        self.get_with_params(&url, &params).await
    }

    /// Get a single issue by ID
    pub async fn get_issue(&self, issue_id: &str) -> Result<Issue> {
        let org = self.organization()?;
        let url = format!(
            "{}/organizations/{}/issues/{}/",
            SENTRY_API_URL, org, issue_id
        );

        self.get(&url).await
    }

    /// List events for an issue
    pub async fn list_issue_events(&self, issue_id: &str, limit: usize) -> Result<Vec<Event>> {
        let org = self.organization()?;
        let url = format!(
            "{}/organizations/{}/issues/{}/events/",
            SENTRY_API_URL, org, issue_id
        );

        self.get_with_params(&url, &[("limit", limit.to_string())])
            .await
    }

    /// Make a GET request
    async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        let token = self.auth_token()?.to_string();

        self.execute_with_retry(|| {
            self.http
                .get(url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
        })
        .await
    }

    /// Make a GET request with parameters
    async fn get_with_params<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let token = self.auth_token()?.to_string();
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();

        self.execute_with_retry(|| {
            self.http
                .get(url)
                .header("Authorization", format!("Bearer {}", token))
                .query(&params)
                .send()
        })
        .await
    }

    /// Execute request with retry on rate limit
    async fn execute_with_retry<F, Fut, T>(&self, request_fn: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
        T: DeserializeOwned,
    {
        let mut retries = 0;

        loop {
            let response = request_fn().await?;
            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                if retries >= MAX_RETRIES {
                    return Err(anyhow::anyhow!(
                        "Rate limited after {} retries",
                        MAX_RETRIES
                    ));
                }

                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(DEFAULT_RETRY_SECS);

                eprintln!(
                    "Rate limited, waiting {} seconds... (retry {}/{})",
                    retry_after,
                    retries + 1,
                    MAX_RETRIES
                );
                sleep(Duration::from_secs(retry_after)).await;
                retries += 1;
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("HTTP {}: {}", status.as_u16(), body));
            }

            let text = response.text().await?;
            return serde_json::from_str(&text).map_err(|e| {
                anyhow::anyhow!("Parse error: {}: {}", e, &text[..text.len().min(200)])
            });
        }
    }
}
