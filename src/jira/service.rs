//! Jira service layer - business logic that returns data
//!
//! Functions in this module accept trait objects and return typed data.
//! They never print - that's the CLI layer's job.

use anyhow::Result;

use super::client::{JiraApi, JiraClient};
use super::types::{Issue, IssueUpdate, Transition, User};

/// Get a single issue by key
pub async fn get_issue(api: &impl JiraApi, key: &str) -> Result<Issue> {
    api.get_issue(key).await
}

/// Search issues using JQL
pub async fn search_issues(api: &impl JiraApi, jql: &str) -> Result<Vec<Issue>> {
    api.search_issues(jql).await
}

/// Get current authenticated user
pub async fn get_current_user(api: &impl JiraApi) -> Result<User> {
    api.get_current_user().await
}

/// Update issue fields
pub async fn update_issue(api: &impl JiraApi, key: &str, update: &IssueUpdate) -> Result<()> {
    api.update_issue(key, update).await
}

/// Get available transitions for an issue
pub async fn get_transitions(api: &impl JiraApi, key: &str) -> Result<Vec<Transition>> {
    api.get_transitions(key).await
}

/// Transition an issue to a new status
pub async fn transition_issue(api: &impl JiraApi, key: &str, transition_id: &str) -> Result<()> {
    api.transition_issue(key, transition_id).await
}

/// Create a new authenticated client
pub async fn create_client() -> Result<JiraClient> {
    JiraClient::new().await
}

#[cfg(test)]
mod tests {
    use super::super::types::Comment;
    use super::*;

    struct MockApi {
        issues: Vec<Issue>,
        user: Option<User>,
        transitions: Vec<Transition>,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                issues: vec![],
                user: None,
                transitions: vec![],
            }
        }

        fn with_issues(mut self, issues: Vec<Issue>) -> Self {
            self.issues = issues;
            self
        }

        fn with_user(mut self, user: User) -> Self {
            self.user = Some(user);
            self
        }

        fn with_transitions(mut self, transitions: Vec<Transition>) -> Self {
            self.transitions = transitions;
            self
        }
    }

    impl JiraApi for MockApi {
        async fn get_current_user(&self) -> Result<User> {
            self.user
                .clone()
                .ok_or_else(|| anyhow::anyhow!("No user configured"))
        }

        async fn get_issue(&self, key: &str) -> Result<Issue> {
            self.issues
                .iter()
                .find(|i| i.key == key)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Issue not found: {}", key))
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<Issue>> {
            Ok(self.issues.clone())
        }

        async fn update_issue(&self, _key: &str, _update: &IssueUpdate) -> Result<()> {
            Ok(())
        }

        async fn get_transitions(&self, _key: &str) -> Result<Vec<Transition>> {
            Ok(self.transitions.clone())
        }

        async fn transition_issue(&self, _key: &str, _transition_id: &str) -> Result<()> {
            Ok(())
        }

        async fn list_comments(&self, _key: &str) -> Result<Vec<Comment>> {
            Ok(vec![])
        }
    }

    fn make_issue(key: &str, summary: &str, status: &str) -> Issue {
        Issue {
            key: key.to_string(),
            summary: summary.to_string(),
            status: status.to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            description: None,
            updated: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn get_issue_returns_matching() {
        let api = MockApi::new().with_issues(vec![
            make_issue("PROJ-1", "First issue", "Open"),
            make_issue("PROJ-2", "Second issue", "Done"),
        ]);

        let result = get_issue(&api, "PROJ-2").await.unwrap();
        assert_eq!(result.key, "PROJ-2");
        assert_eq!(result.summary, "Second issue");
    }

    #[tokio::test]
    async fn get_issue_not_found() {
        let api = MockApi::new();
        let result = get_issue(&api, "MISSING").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_issues_returns_all() {
        let api = MockApi::new().with_issues(vec![
            make_issue("PROJ-1", "First", "Open"),
            make_issue("PROJ-2", "Second", "Done"),
        ]);

        let result = search_issues(&api, "project = PROJ").await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn get_current_user_returns_user() {
        let api = MockApi::new().with_user(User {
            account_id: "123".to_string(),
            display_name: "John Doe".to_string(),
            email_address: Some("john@test.com".to_string()),
        });

        let result = get_current_user(&api).await.unwrap();
        assert_eq!(result.display_name, "John Doe");
    }

    #[tokio::test]
    async fn get_transitions_returns_list() {
        let api = MockApi::new().with_transitions(vec![
            Transition {
                id: "1".to_string(),
                name: "Start Progress".to_string(),
            },
            Transition {
                id: "2".to_string(),
                name: "Done".to_string(),
            },
        ]);

        let result = get_transitions(&api, "PROJ-1").await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Start Progress");
    }

    #[tokio::test]
    async fn update_issue_succeeds() {
        let api = MockApi::new();
        let update = IssueUpdate {
            summary: Some("New summary".to_string()),
            ..Default::default()
        };

        let result = update_issue(&api, "PROJ-1", &update).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn transition_issue_succeeds() {
        let api = MockApi::new();
        let result = transition_issue(&api, "PROJ-1", "2").await;
        assert!(result.is_ok());
    }
}
