use anyhow::Result;

use super::client::{JiraApi, JiraClient};
use super::types::Issue;

/// Run the jira search command
pub async fn run(query: &str) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_search(&client, query).await?;
    print!("{}", output);
    Ok(())
}

/// Process search command (business logic, testable)
pub async fn process_search(client: &impl JiraApi, query: &str) -> Result<String> {
    let issues = client.search_issues(query).await?;
    Ok(format_search_results(&issues, query))
}

/// Format search results
fn format_search_results(issues: &[Issue], query: &str) -> String {
    let mut output = String::new();

    if issues.is_empty() {
        output.push_str(&format!("No issues found for: {}\n", query));
        return output;
    }

    output.push_str(&format!(
        "Found {} issue{} for: {}\n\n",
        issues.len(),
        if issues.len() == 1 { "" } else { "s" },
        query
    ));

    // Calculate column widths
    let key_width = issues.iter().map(|i| i.key.len()).max().unwrap_or(0).max(4);
    let status_width = issues
        .iter()
        .map(|i| i.status.len())
        .max()
        .unwrap_or(0)
        .max(6);

    for issue in issues {
        let assignee = issue.assignee.as_deref().unwrap_or("-");
        let status_color = match issue.status.as_str() {
            "Done" => "\x1b[32m",        // green
            "In Progress" => "\x1b[33m", // yellow
            _ => "\x1b[34m",             // blue
        };

        output.push_str(&format!(
            "{:<key_w$}  {}{:<status_w$}\x1b[0m  {}\n",
            issue.key,
            status_color,
            issue.status,
            truncate(&issue.summary, 50),
            key_w = key_width,
            status_w = status_width,
        ));
        output.push_str(&format!(
            "{:<key_w$}  \x1b[90m{} | {}\x1b[0m\n",
            "",
            issue.issue_type,
            assignee,
            key_w = key_width,
        ));
    }

    output
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{Board, IssueUpdate, Sprint, Transition, User};
    use super::*;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn truncate_very_short_max() {
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn truncate_zero_max() {
        assert_eq!(truncate("hello", 0), "...");
    }

    #[test]
    fn format_search_results_empty() {
        let issues: Vec<Issue> = vec![];
        let output = format_search_results(&issues, "project = TEST");
        assert!(output.contains("No issues found"));
        assert!(output.contains("project = TEST"));
    }

    #[test]
    fn format_search_results_single() {
        let issues = vec![Issue {
            key: "TEST-1".to_string(),
            summary: "Test issue".to_string(),
            status: "Open".to_string(),
            issue_type: "Bug".to_string(),
            assignee: Some("Alice".to_string()),
            description: None,
            updated: "2024-01-01T00:00:00Z".to_string(),
        }];
        let output = format_search_results(&issues, "jql");
        assert!(output.contains("Found 1 issue for"));
        assert!(output.contains("TEST-1"));
        assert!(output.contains("Test issue"));
        assert!(output.contains("Bug"));
        assert!(output.contains("Alice"));
    }

    #[test]
    fn format_search_results_multiple() {
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "First".to_string(),
                status: "Done".to_string(),
                issue_type: "Task".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Second".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Story".to_string(),
                assignee: Some("Bob".to_string()),
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_search_results(&issues, "q");
        assert!(output.contains("Found 2 issues"));
        assert!(output.contains("A-1"));
        assert!(output.contains("A-2"));
        assert!(output.contains("-")); // unassigned
        assert!(output.contains("Bob"));
    }

    #[test]
    fn format_search_results_truncates_long_summary() {
        let issues = vec![Issue {
            key: "X-1".to_string(),
            summary: "This is a very long summary that should be truncated to fit on screen"
                .to_string(),
            status: "Open".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            description: None,
            updated: "U".to_string(),
        }];
        let output = format_search_results(&issues, "q");
        assert!(output.contains("..."));
    }

    #[test]
    fn format_search_results_colors_status() {
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "Done".to_string(),
                status: "Done".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "In Progress".to_string(),
                status: "In Progress".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "A-3".to_string(),
                summary: "Other".to_string(),
                status: "Other".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_search_results(&issues, "q");
        assert!(output.contains("\x1b[32m")); // green for Done
        assert!(output.contains("\x1b[33m")); // yellow for In Progress
        assert!(output.contains("\x1b[34m")); // blue for other
    }

    // Mock client for testing process_search
    struct MockJiraClient {
        issues: Vec<Issue>,
    }

    impl JiraApi for MockJiraClient {
        async fn get_current_user(&self) -> Result<User> {
            unimplemented!()
        }

        async fn get_boards(&self) -> Result<Vec<Board>> {
            unimplemented!()
        }

        async fn get_active_sprint(&self, _board_id: u64) -> Result<Option<Sprint>> {
            unimplemented!()
        }

        async fn get_sprint_issues(&self, _sprint_id: u64) -> Result<Vec<Issue>> {
            unimplemented!()
        }

        async fn get_issue(&self, _key: &str) -> Result<Issue> {
            unimplemented!()
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<Issue>> {
            Ok(self.issues.clone())
        }

        async fn update_issue(&self, _key: &str, _update: &IssueUpdate) -> Result<()> {
            unimplemented!()
        }

        async fn get_transitions(&self, _key: &str) -> Result<Vec<Transition>> {
            unimplemented!()
        }

        async fn transition_issue(&self, _key: &str, _transition_id: &str) -> Result<()> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn process_search_returns_formatted_results() {
        let client = MockJiraClient {
            issues: vec![Issue {
                key: "TEST-123".to_string(),
                summary: "Test issue".to_string(),
                status: "Open".to_string(),
                issue_type: "Bug".to_string(),
                assignee: Some("Tester".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            }],
        };

        let output = process_search(&client, "project = TEST").await.unwrap();
        assert!(output.contains("TEST-123"));
        assert!(output.contains("Test issue"));
    }

    #[tokio::test]
    async fn process_search_empty_results() {
        let client = MockJiraClient { issues: vec![] };

        let output = process_search(&client, "nonexistent").await.unwrap();
        assert!(output.contains("No issues found"));
    }
}
