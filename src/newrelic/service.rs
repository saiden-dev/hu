//! New Relic service layer - business logic that returns data
//!
//! Functions in this module accept trait objects and return typed data.
//! They never print - that's the CLI layer's job.

use anyhow::{bail, Result};

use super::client::NewRelicApi;
use super::config::{self, NewRelicConfig};
use super::types::{Incident, Issue};

/// Get current configuration
pub fn get_config() -> Result<NewRelicConfig> {
    config::load_config()
}

/// Save API key and account ID
pub fn save_auth(key: &str, account_id: i64) -> Result<()> {
    config::save_config(key, account_id)
}

/// Check if API is configured, return error if not
pub fn ensure_configured(config: &NewRelicConfig) -> Result<()> {
    if !config.is_configured() {
        bail!(
            "New Relic not configured. Run: hu newrelic auth <key> --account <id>\n\
             Or set NEW_RELIC_API_KEY and NEW_RELIC_ACCOUNT_ID environment variables."
        );
    }
    Ok(())
}

/// List recent issues
pub async fn list_issues(api: &impl NewRelicApi, limit: usize) -> Result<Vec<Issue>> {
    api.list_issues(limit).await
}

/// List recent incidents
pub async fn list_incidents(api: &impl NewRelicApi, limit: usize) -> Result<Vec<Incident>> {
    api.list_incidents(limit).await
}

/// Run NRQL query
pub async fn run_nrql(api: &impl NewRelicApi, nrql: &str) -> Result<Vec<serde_json::Value>> {
    api.run_nrql(nrql).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock New Relic API for testing
    struct MockApi {
        issues: Vec<Issue>,
        incidents: Vec<Incident>,
        nrql_results: Vec<serde_json::Value>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                issues: vec![],
                incidents: vec![],
                nrql_results: vec![],
            }
        }

        fn with_issues(mut self, issues: Vec<Issue>) -> Self {
            self.issues = issues;
            self
        }

        fn with_incidents(mut self, incidents: Vec<Incident>) -> Self {
            self.incidents = incidents;
            self
        }

        fn with_nrql_results(mut self, results: Vec<serde_json::Value>) -> Self {
            self.nrql_results = results;
            self
        }
    }

    impl NewRelicApi for MockApi {
        async fn list_issues(&self, limit: usize) -> Result<Vec<Issue>> {
            Ok(self.issues.iter().take(limit).cloned().collect())
        }

        async fn list_incidents(&self, limit: usize) -> Result<Vec<Incident>> {
            Ok(self.incidents.iter().take(limit).cloned().collect())
        }

        async fn run_nrql(&self, _nrql: &str) -> Result<Vec<serde_json::Value>> {
            Ok(self.nrql_results.clone())
        }
    }

    fn make_issue(id: &str, title: &str, priority: &str, state: &str) -> Issue {
        Issue {
            issue_id: id.to_string(),
            title: vec![title.to_string()],
            priority: priority.to_string(),
            state: state.to_string(),
            entity_names: vec!["test-entity".to_string()],
            created_at: Some(1704067200000),
            closed_at: None,
            activated_at: Some(1704067200000),
        }
    }

    fn make_incident(id: &str, title: &str, priority: &str, state: &str) -> Incident {
        Incident {
            incident_id: id.to_string(),
            title: title.to_string(),
            priority: priority.to_string(),
            state: state.to_string(),
            account_ids: vec![12345],
            created_at: Some(1704067200000),
            closed_at: None,
        }
    }

    #[tokio::test]
    async fn list_issues_returns_all() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "Issue 1", "CRITICAL", "ACTIVATED"),
            make_issue("2", "Issue 2", "HIGH", "CREATED"),
        ]);

        let result = list_issues(&api, 10).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn list_issues_respects_limit() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "Issue 1", "CRITICAL", "ACTIVATED"),
            make_issue("2", "Issue 2", "HIGH", "CREATED"),
            make_issue("3", "Issue 3", "MEDIUM", "CLOSED"),
        ]);

        let result = list_issues(&api, 2).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn list_incidents_returns_all() {
        let api = MockApi::new().with_incidents(vec![
            make_incident("INC1", "Incident 1", "CRITICAL", "ACTIVATED"),
            make_incident("INC2", "Incident 2", "HIGH", "CREATED"),
        ]);

        let result = list_incidents(&api, 10).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn list_incidents_respects_limit() {
        let api = MockApi::new().with_incidents(vec![
            make_incident("INC1", "Incident 1", "CRITICAL", "ACTIVATED"),
            make_incident("INC2", "Incident 2", "HIGH", "CREATED"),
            make_incident("INC3", "Incident 3", "MEDIUM", "CLOSED"),
        ]);

        let result = list_incidents(&api, 2).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn run_nrql_returns_results() {
        let api = MockApi::new().with_nrql_results(vec![
            serde_json::json!({"count": 100}),
            serde_json::json!({"count": 200}),
        ]);

        let result = run_nrql(&api, "SELECT count(*) FROM Transaction")
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["count"], 100);
    }

    #[tokio::test]
    async fn run_nrql_returns_empty() {
        let api = MockApi::new();
        let result = run_nrql(&api, "SELECT count(*) FROM Nothing")
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn ensure_configured_fails_without_key() {
        let config = NewRelicConfig::default();
        let result = ensure_configured(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[test]
    fn ensure_configured_fails_without_account() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-test".to_string()),
            account_id: None,
        };
        let result = ensure_configured(&config);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_configured_succeeds_with_both() {
        let config = NewRelicConfig {
            api_key: Some("NRAK-test".to_string()),
            account_id: Some(12345),
        };
        let result = ensure_configured(&config);
        assert!(result.is_ok());
    }
}
