//! PagerDuty API client

use anyhow::Result;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

use super::config::{load_config, PagerDutyConfig};
use super::types::{
    CurrentUserResponse, Incident, IncidentResponse, IncidentStatus, IncidentsResponse, Oncall,
    OncallsResponse, Service, ServicesResponse, User,
};

#[cfg(test)]
mod tests;

const PAGERDUTY_API_URL: &str = "https://api.pagerduty.com";
const MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_SECS: u64 = 5;

/// PagerDuty API trait for testability
#[allow(dead_code)]
pub trait PagerDutyApi: Send + Sync {
    /// Get current user
    fn get_current_user(&self) -> impl Future<Output = Result<User>> + Send;

    /// List who's on call
    fn list_oncalls(
        &self,
        schedule_ids: Option<&[String]>,
        escalation_policy_ids: Option<&[String]>,
    ) -> impl Future<Output = Result<Vec<Oncall>>> + Send;

    /// List incidents
    fn list_incidents(
        &self,
        statuses: &[IncidentStatus],
        limit: usize,
    ) -> impl Future<Output = Result<Vec<Incident>>> + Send;

    /// Get single incident
    fn get_incident(&self, id: &str) -> impl Future<Output = Result<Incident>> + Send;

    /// List services
    fn list_services(&self) -> impl Future<Output = Result<Vec<Service>>> + Send;
}

/// PagerDuty HTTP client
pub struct PagerDutyClient {
    config: PagerDutyConfig,
    http: Client,
}

impl PagerDutyClient {
    /// Create a new client
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        let http = Client::builder().user_agent("hu-cli/0.1.0").build()?;
        Ok(Self { config, http })
    }

    /// Get config reference
    pub fn config(&self) -> &PagerDutyConfig {
        &self.config
    }

    /// Get API token
    fn api_token(&self) -> Result<&str> {
        self.config
            .api_token
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("PagerDuty API token not configured"))
    }

    /// Make authenticated GET request
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_params(path, &[]).await
    }

    /// Make authenticated GET request with query parameters
    async fn get_with_params<T: DeserializeOwned>(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let token = self.api_token()?.to_string();
        let url = format!("{}{}", PAGERDUTY_API_URL, path);
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();

        self.execute_with_retry(|| {
            self.http
                .get(&url)
                .header("Authorization", format!("Token token={}", token))
                .header("Content-Type", "application/json")
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

impl PagerDutyApi for PagerDutyClient {
    async fn get_current_user(&self) -> Result<User> {
        let resp: CurrentUserResponse = self.get("/users/me").await?;
        Ok(resp.user)
    }

    async fn list_oncalls(
        &self,
        schedule_ids: Option<&[String]>,
        escalation_policy_ids: Option<&[String]>,
    ) -> Result<Vec<Oncall>> {
        let params = build_oncall_params(schedule_ids, escalation_policy_ids);
        let resp: OncallsResponse = self.get_with_params("/oncalls", &params).await?;
        Ok(resp.oncalls)
    }

    async fn list_incidents(
        &self,
        statuses: &[IncidentStatus],
        limit: usize,
    ) -> Result<Vec<Incident>> {
        let params = build_incidents_params(statuses, limit);
        let resp: IncidentsResponse = self.get_with_params("/incidents", &params).await?;
        Ok(resp.incidents)
    }

    async fn get_incident(&self, id: &str) -> Result<Incident> {
        let path = format!("/incidents/{}", id);
        let resp: IncidentResponse = self.get(&path).await?;
        Ok(resp.incident)
    }

    async fn list_services(&self) -> Result<Vec<Service>> {
        let resp: ServicesResponse = self.get("/services").await?;
        Ok(resp.services)
    }
}

/// Build query parameters for oncalls endpoint
fn build_oncall_params(
    schedule_ids: Option<&[String]>,
    escalation_policy_ids: Option<&[String]>,
) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();

    if let Some(ids) = schedule_ids {
        for id in ids {
            params.push(("schedule_ids[]", id.clone()));
        }
    }

    if let Some(ids) = escalation_policy_ids {
        for id in ids {
            params.push(("escalation_policy_ids[]", id.clone()));
        }
    }

    params
}

/// Build query parameters for incidents endpoint
fn build_incidents_params(
    statuses: &[IncidentStatus],
    limit: usize,
) -> Vec<(&'static str, String)> {
    let mut params = vec![("limit", limit.to_string())];

    for status in statuses {
        params.push(("statuses[]", status.as_str().to_string()));
    }

    params
}
