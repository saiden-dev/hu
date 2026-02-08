//! Sentry service layer - business logic that returns data
//!
//! Functions in this module accept trait objects and return typed data.
//! They never print - that's the CLI layer's job.

use anyhow::{bail, Result};

use super::client::SentryApi;
use super::config::{self, SentryConfig};
use super::types::{Event, Issue};

/// Options for listing issues
#[derive(Debug, Default)]
pub struct IssueOptions {
    /// Filter by project slug
    pub project: Option<String>,
    /// Search query (Sentry search syntax)
    pub query: Option<String>,
    /// Maximum number of results
    pub limit: usize,
}

impl IssueOptions {
    /// Create with default limit
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            project: None,
            query: None,
            limit: 25,
        }
    }
}

/// Options for listing events
#[derive(Debug)]
pub struct EventOptions {
    /// Issue ID
    pub issue_id: String,
    /// Maximum number of results
    pub limit: usize,
}

impl Default for EventOptions {
    fn default() -> Self {
        Self {
            issue_id: String::new(),
            limit: 25,
        }
    }
}

/// Get current configuration
pub fn get_config() -> Result<SentryConfig> {
    config::load_config()
}

/// Save auth token and organization
pub fn save_auth(token: &str, org: &str) -> Result<()> {
    config::save_auth_token(token, org)
}

/// Check if API is configured, return error if not
pub fn ensure_configured(config: &SentryConfig) -> Result<()> {
    if !config.is_configured() {
        bail!(
            "Sentry not configured. Run: hu sentry auth <token> --org <org>\n\
             Or set SENTRY_AUTH_TOKEN and SENTRY_ORG environment variables."
        );
    }
    Ok(())
}

/// List issues with options
pub async fn list_issues(api: &impl SentryApi, opts: &IssueOptions) -> Result<Vec<Issue>> {
    if let Some(ref project) = opts.project {
        api.list_project_issues(project, opts.query.as_deref(), opts.limit)
            .await
    } else {
        api.list_issues(opts.query.as_deref(), opts.limit).await
    }
}

/// Get a single issue by ID
pub async fn get_issue(api: &impl SentryApi, issue_id: &str) -> Result<Issue> {
    api.get_issue(issue_id).await
}

/// List events for an issue
pub async fn list_events(api: &impl SentryApi, opts: &EventOptions) -> Result<Vec<Event>> {
    api.list_issue_events(&opts.issue_id, opts.limit).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sentry::types::{IssueMetadata, ProjectInfo};

    /// Mock Sentry API for testing
    struct MockApi {
        issues: Vec<Issue>,
        events: Vec<Event>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                issues: vec![],
                events: vec![],
            }
        }

        fn with_issues(mut self, issues: Vec<Issue>) -> Self {
            self.issues = issues;
            self
        }

        fn with_events(mut self, events: Vec<Event>) -> Self {
            self.events = events;
            self
        }
    }

    impl SentryApi for MockApi {
        async fn list_issues(&self, query: Option<&str>, limit: usize) -> Result<Vec<Issue>> {
            let filtered: Vec<Issue> = self
                .issues
                .iter()
                .filter(|i| {
                    query
                        .map(|q| i.title.contains(q) || i.short_id.contains(q))
                        .unwrap_or(true)
                })
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn list_project_issues(
            &self,
            project: &str,
            query: Option<&str>,
            limit: usize,
        ) -> Result<Vec<Issue>> {
            let filtered: Vec<Issue> = self
                .issues
                .iter()
                .filter(|i| i.project.slug == project)
                .filter(|i| {
                    query
                        .map(|q| i.title.contains(q) || i.short_id.contains(q))
                        .unwrap_or(true)
                })
                .take(limit)
                .cloned()
                .collect();
            Ok(filtered)
        }

        async fn get_issue(&self, issue_id: &str) -> Result<Issue> {
            self.issues
                .iter()
                .find(|i| i.id == issue_id || i.short_id == issue_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", issue_id))
        }

        async fn list_issue_events(&self, _issue_id: &str, limit: usize) -> Result<Vec<Event>> {
            Ok(self.events.iter().take(limit).cloned().collect())
        }
    }

    fn make_issue(id: &str, short_id: &str, title: &str, project_slug: &str) -> Issue {
        Issue {
            id: id.to_string(),
            short_id: short_id.to_string(),
            title: title.to_string(),
            culprit: "app.module".to_string(),
            level: "error".to_string(),
            status: "unresolved".to_string(),
            platform: "python".to_string(),
            project: ProjectInfo {
                id: "1".to_string(),
                name: "Test Project".to_string(),
                slug: project_slug.to_string(),
            },
            count: "100".to_string(),
            user_count: 10,
            first_seen: "2024-01-01T00:00:00Z".to_string(),
            last_seen: "2024-01-02T00:00:00Z".to_string(),
            permalink: format!("https://sentry.io/issues/{}", id),
            is_subscribed: false,
            is_bookmarked: false,
            metadata: IssueMetadata::default(),
        }
    }

    fn make_event(id: &str, title: &str) -> Event {
        Event {
            id: id.to_string(),
            title: title.to_string(),
            message: "Error occurred".to_string(),
            platform: "python".to_string(),
            date_created: Some("2024-01-01T00:00:00Z".to_string()),
            user: None,
            tags: vec![],
        }
    }

    #[tokio::test]
    async fn list_issues_returns_all() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "PROJ-1", "Error 1", "proj"),
            make_issue("2", "PROJ-2", "Error 2", "proj"),
        ]);

        let opts = IssueOptions {
            project: None,
            query: None,
            limit: 10,
        };
        let result = list_issues(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn list_issues_filters_by_project() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "PROJ-1", "Error 1", "proj-a"),
            make_issue("2", "PROJ-2", "Error 2", "proj-b"),
        ]);

        let opts = IssueOptions {
            project: Some("proj-a".to_string()),
            query: None,
            limit: 10,
        };
        let result = list_issues(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].short_id, "PROJ-1");
    }

    #[tokio::test]
    async fn list_issues_filters_by_query() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "PROJ-1", "Database error", "proj"),
            make_issue("2", "PROJ-2", "Network timeout", "proj"),
        ]);

        let opts = IssueOptions {
            project: None,
            query: Some("Database".to_string()),
            limit: 10,
        };
        let result = list_issues(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Database error");
    }

    #[tokio::test]
    async fn list_issues_respects_limit() {
        let api = MockApi::new().with_issues(vec![
            make_issue("1", "PROJ-1", "Error 1", "proj"),
            make_issue("2", "PROJ-2", "Error 2", "proj"),
            make_issue("3", "PROJ-3", "Error 3", "proj"),
        ]);

        let opts = IssueOptions {
            project: None,
            query: None,
            limit: 2,
        };
        let result = list_issues(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn get_issue_by_id() {
        let api = MockApi::new().with_issues(vec![
            make_issue("123", "PROJ-1", "Error 1", "proj"),
            make_issue("456", "PROJ-2", "Error 2", "proj"),
        ]);

        let result = get_issue(&api, "456").await.unwrap();
        assert_eq!(result.id, "456");
        assert_eq!(result.title, "Error 2");
    }

    #[tokio::test]
    async fn get_issue_by_short_id() {
        let api = MockApi::new().with_issues(vec![make_issue("123", "PROJ-42", "Error", "proj")]);

        let result = get_issue(&api, "PROJ-42").await.unwrap();
        assert_eq!(result.short_id, "PROJ-42");
    }

    #[tokio::test]
    async fn get_issue_not_found() {
        let api = MockApi::new();
        let result = get_issue(&api, "MISSING").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_events_returns_data() {
        let api = MockApi::new().with_events(vec![
            make_event("evt1", "Event 1"),
            make_event("evt2", "Event 2"),
        ]);

        let opts = EventOptions {
            issue_id: "123".to_string(),
            limit: 10,
        };
        let result = list_events(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn list_events_respects_limit() {
        let api = MockApi::new().with_events(vec![
            make_event("evt1", "Event 1"),
            make_event("evt2", "Event 2"),
            make_event("evt3", "Event 3"),
        ]);

        let opts = EventOptions {
            issue_id: "123".to_string(),
            limit: 2,
        };
        let result = list_events(&api, &opts).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn ensure_configured_fails_without_token() {
        let config = SentryConfig::default();
        let result = ensure_configured(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not configured"));
    }

    #[test]
    fn ensure_configured_fails_without_org() {
        let config = SentryConfig {
            auth_token: Some("token".to_string()),
            organization: None,
            project: None,
        };
        let result = ensure_configured(&config);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_configured_succeeds_with_token_and_org() {
        let config = SentryConfig {
            auth_token: Some("token".to_string()),
            organization: Some("my-org".to_string()),
            project: None,
        };
        let result = ensure_configured(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn issue_options_default() {
        let opts = IssueOptions::default();
        assert!(opts.project.is_none());
        assert!(opts.query.is_none());
        assert_eq!(opts.limit, 0);
    }

    #[test]
    fn issue_options_new() {
        let opts = IssueOptions::new();
        assert_eq!(opts.limit, 25);
    }

    #[test]
    fn event_options_default() {
        let opts = EventOptions::default();
        assert!(opts.issue_id.is_empty());
        assert_eq!(opts.limit, 25);
    }
}
