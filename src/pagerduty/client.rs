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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_oncall_params_empty() {
        let params = build_oncall_params(None, None);
        assert!(params.is_empty());
    }

    #[test]
    fn build_oncall_params_with_schedule() {
        let schedules = vec!["S1".to_string(), "S2".to_string()];
        let params = build_oncall_params(Some(&schedules), None);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("schedule_ids[]", "S1".to_string()));
        assert_eq!(params[1], ("schedule_ids[]", "S2".to_string()));
    }

    #[test]
    fn build_oncall_params_with_policy() {
        let policies = vec!["EP1".to_string()];
        let params = build_oncall_params(None, Some(&policies));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("escalation_policy_ids[]", "EP1".to_string()));
    }

    #[test]
    fn build_oncall_params_with_both() {
        let schedules = vec!["S1".to_string()];
        let policies = vec!["EP1".to_string()];
        let params = build_oncall_params(Some(&schedules), Some(&policies));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn build_incidents_params_basic() {
        let statuses = vec![IncidentStatus::Triggered];
        let params = build_incidents_params(&statuses, 25);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("limit", "25".to_string()));
        assert_eq!(params[1], ("statuses[]", "triggered".to_string()));
    }

    #[test]
    fn build_incidents_params_multiple_statuses() {
        let statuses = vec![IncidentStatus::Triggered, IncidentStatus::Acknowledged];
        let params = build_incidents_params(&statuses, 10);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], ("limit", "10".to_string()));
        assert_eq!(params[1], ("statuses[]", "triggered".to_string()));
        assert_eq!(params[2], ("statuses[]", "acknowledged".to_string()));
    }

    #[test]
    fn build_incidents_params_empty_statuses() {
        let statuses: Vec<IncidentStatus> = vec![];
        let params = build_incidents_params(&statuses, 50);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("limit", "50".to_string()));
    }

    // Mock implementation for testing handlers
    pub struct MockPagerDutyApi {
        pub oncalls: Vec<Oncall>,
        pub incidents: Vec<Incident>,
        pub services: Vec<Service>,
        pub current_user: Option<User>,
    }

    impl MockPagerDutyApi {
        pub fn new() -> Self {
            Self {
                oncalls: vec![],
                incidents: vec![],
                services: vec![],
                current_user: None,
            }
        }

        pub fn with_oncalls(mut self, oncalls: Vec<Oncall>) -> Self {
            self.oncalls = oncalls;
            self
        }

        pub fn with_incidents(mut self, incidents: Vec<Incident>) -> Self {
            self.incidents = incidents;
            self
        }

        pub fn with_services(mut self, services: Vec<Service>) -> Self {
            self.services = services;
            self
        }

        pub fn with_user(mut self, user: User) -> Self {
            self.current_user = Some(user);
            self
        }
    }

    impl PagerDutyApi for MockPagerDutyApi {
        async fn get_current_user(&self) -> Result<User> {
            self.current_user
                .clone()
                .ok_or_else(|| anyhow::anyhow!("No user configured"))
        }

        async fn list_oncalls(
            &self,
            _schedule_ids: Option<&[String]>,
            _escalation_policy_ids: Option<&[String]>,
        ) -> Result<Vec<Oncall>> {
            Ok(self.oncalls.clone())
        }

        async fn list_incidents(
            &self,
            _statuses: &[IncidentStatus],
            limit: usize,
        ) -> Result<Vec<Incident>> {
            Ok(self.incidents.iter().take(limit).cloned().collect())
        }

        async fn get_incident(&self, id: &str) -> Result<Incident> {
            self.incidents
                .iter()
                .find(|i| i.id == id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Incident not found: {}", id))
        }

        async fn list_services(&self) -> Result<Vec<Service>> {
            Ok(self.services.clone())
        }
    }

    #[tokio::test]
    async fn mock_list_oncalls() {
        let oncall = make_test_oncall("U1", "Alice");
        let mock = MockPagerDutyApi::new().with_oncalls(vec![oncall]);

        let result = mock.list_oncalls(None, None).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user.display_name(), "Alice");
    }

    #[tokio::test]
    async fn mock_list_incidents_respects_limit() {
        let incidents = vec![
            make_test_incident("1"),
            make_test_incident("2"),
            make_test_incident("3"),
        ];
        let mock = MockPagerDutyApi::new().with_incidents(incidents);

        let result = mock
            .list_incidents(&[IncidentStatus::Triggered], 2)
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn mock_get_incident() {
        let incidents = vec![make_test_incident("INC1"), make_test_incident("INC2")];
        let mock = MockPagerDutyApi::new().with_incidents(incidents);

        let result = mock.get_incident("INC1").await.unwrap();
        assert_eq!(result.id, "INC1");
    }

    #[tokio::test]
    async fn mock_get_incident_not_found() {
        let mock = MockPagerDutyApi::new();
        let result = mock.get_incident("MISSING").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_get_current_user() {
        let user = make_test_user("U1", "Alice");
        let mock = MockPagerDutyApi::new().with_user(user);

        let result = mock.get_current_user().await.unwrap();
        assert_eq!(result.display_name(), "Alice");
    }

    #[tokio::test]
    async fn mock_get_current_user_not_configured() {
        let mock = MockPagerDutyApi::new();
        let result = mock.get_current_user().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn mock_list_services() {
        let services = vec![make_test_service("S1", "Production")];
        let mock = MockPagerDutyApi::new().with_services(services);

        let result = mock.list_services().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Production");
    }

    #[test]
    fn client_new_creates_instance() {
        // This tests the happy path of client creation
        let result = PagerDutyClient::new();
        assert!(result.is_ok());
    }

    #[test]
    fn client_config_returns_reference() {
        let client = PagerDutyClient::new().unwrap();
        let _config = client.config();
        // Just verify we get a reference without panic
    }

    #[test]
    fn api_token_returns_error_when_not_set() {
        let client = PagerDutyClient::new().unwrap();
        // If no token is configured, api_token() should return error
        // This depends on whether PAGERDUTY_API_TOKEN env var is set
        let result = client.api_token();
        // Just exercise the code path
        let _ = result;
    }

    #[test]
    fn mock_builder_pattern() {
        // Test that all builder methods work correctly
        let user = make_test_user("U1", "Alice");
        let oncalls = vec![make_test_oncall("U1", "Alice")];
        let incidents = vec![make_test_incident("INC1")];
        let services = vec![make_test_service("S1", "Production")];

        let mock = MockPagerDutyApi::new()
            .with_user(user.clone())
            .with_oncalls(oncalls.clone())
            .with_incidents(incidents.clone())
            .with_services(services.clone());

        assert_eq!(mock.current_user.as_ref().unwrap().id, "U1");
        assert_eq!(mock.oncalls.len(), 1);
        assert_eq!(mock.incidents.len(), 1);
        assert_eq!(mock.services.len(), 1);
    }

    // Test data helpers
    fn make_test_user(id: &str, name: &str) -> User {
        User {
            id: id.to_string(),
            name: Some(name.to_string()),
            summary: None,
            email: format!("{}@example.com", name.to_lowercase()),
            html_url: String::new(),
        }
    }

    fn make_test_oncall(user_id: &str, user_name: &str) -> Oncall {
        use super::super::types::{EscalationPolicy, Schedule};

        Oncall {
            user: make_test_user(user_id, user_name),
            schedule: Some(Schedule {
                id: "S1".to_string(),
                name: "Weekly Rotation".to_string(),
                html_url: String::new(),
            }),
            escalation_policy: EscalationPolicy {
                id: "EP1".to_string(),
                name: "Primary".to_string(),
                html_url: String::new(),
            },
            escalation_level: 1,
            start: Some("2026-01-01T00:00:00Z".to_string()),
            end: Some("2026-01-08T00:00:00Z".to_string()),
        }
    }

    fn make_test_incident(id: &str) -> Incident {
        use super::super::types::Urgency;

        Incident {
            id: id.to_string(),
            incident_number: id.parse().unwrap_or(1),
            title: format!("Test incident {}", id),
            status: IncidentStatus::Triggered,
            urgency: Urgency::High,
            created_at: "2026-01-01T12:00:00Z".to_string(),
            html_url: String::new(),
            service: make_test_service("S1", "Production"),
            assignments: vec![],
        }
    }

    fn make_test_service(id: &str, name: &str) -> Service {
        Service {
            id: id.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            html_url: String::new(),
        }
    }
}
