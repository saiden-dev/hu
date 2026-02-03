use anyhow::Result;

use super::client::{JiraApi, JiraClient};
use super::types::Issue;

/// Arguments for sprint command
#[derive(Debug, Clone, Default)]
pub struct SprintArgs {
    // Reserved for future options (e.g., filter by project)
    pub _placeholder: Option<()>,
}

/// Run the jira sprint command
pub async fn run(_args: SprintArgs) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_sprint(&client).await?;
    print!("{}", output);
    Ok(())
}

/// Process sprint command (business logic, testable)
pub async fn process_sprint(client: &impl JiraApi) -> Result<String> {
    // Use JQL to find all issues in active sprints
    let jql = "sprint in openSprints() ORDER BY status ASC, updated DESC";
    let issues = client.search_issues(jql).await?;

    Ok(format_sprint_output(&issues))
}

/// Format sprint output
fn format_sprint_output(issues: &[Issue]) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "\x1b[1mActive Sprint Issues\x1b[0m ({} total)\n\n",
        issues.len()
    ));

    if issues.is_empty() {
        output.push_str("No issues in active sprints\n");
        return output;
    }

    // Group by status
    let mut by_status: std::collections::HashMap<&str, Vec<&Issue>> =
        std::collections::HashMap::new();
    for issue in issues {
        by_status.entry(&issue.status).or_default().push(issue);
    }

    // Status order preference
    let status_order = ["To Do", "In Progress", "In Review", "CODE REVIEW", "Done"];

    // Output in order, then any remaining
    for status in &status_order {
        if let Some(issues) = by_status.remove(*status) {
            output.push_str(&format_status_section(status, &issues));
        }
    }

    // Remaining statuses
    let mut remaining: Vec<_> = by_status.into_iter().collect();
    remaining.sort_by_key(|(status, _)| *status);
    for (status, issues) in remaining {
        output.push_str(&format_status_section(status, &issues));
    }

    output
}

/// Format a status section
fn format_status_section(status: &str, issues: &[&Issue]) -> String {
    let mut output = String::new();
    let status_color = match status {
        "Done" => "\x1b[32m",                                      // green
        "In Progress" | "In Review" | "CODE REVIEW" => "\x1b[33m", // yellow
        _ => "\x1b[34m",                                           // blue
    };
    output.push_str(&format!(
        "{}{}\x1b[0m ({})\n",
        status_color,
        status,
        issues.len()
    ));

    for issue in issues {
        let assignee = issue.assignee.as_deref().unwrap_or("Unassigned");
        output.push_str(&format!(
            "  {} {} \x1b[90m({})\x1b[0m\n",
            issue.key, issue.summary, assignee
        ));
    }
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprint_args_debug() {
        let args = SprintArgs::default();
        let debug_str = format!("{:?}", args);
        assert!(debug_str.contains("SprintArgs"));
    }

    #[test]
    fn sprint_args_clone() {
        let args = SprintArgs::default();
        let cloned = args.clone();
        assert_eq!(cloned._placeholder, args._placeholder);
    }

    #[test]
    fn format_sprint_output_shows_header() {
        let issues = vec![];
        let output = format_sprint_output(&issues);
        assert!(output.contains("Active Sprint Issues"));
        assert!(output.contains("0 total"));
        assert!(output.contains("No issues in active sprints"));
    }

    #[test]
    fn format_sprint_output_groups_by_status() {
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "Task 1".to_string(),
                status: "To Do".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Alice".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Task 2".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Bob".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
            Issue {
                key: "A-3".to_string(),
                summary: "Task 3".to_string(),
                status: "Done".to_string(),
                issue_type: "Task".to_string(),
                assignee: None,
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
        ];
        let output = format_sprint_output(&issues);
        assert!(output.contains("A-1"));
        assert!(output.contains("Task 1"));
        assert!(output.contains("Alice"));
        assert!(output.contains("A-2"));
        assert!(output.contains("Bob"));
        assert!(output.contains("A-3"));
        assert!(output.contains("Unassigned"));
    }

    #[test]
    fn format_status_section_shows_count() {
        let issue1 = Issue {
            key: "X-1".to_string(),
            summary: "S1".to_string(),
            status: "Open".to_string(),
            issue_type: "T".to_string(),
            assignee: None,
            description: None,
            updated: "U".to_string(),
        };
        let issue2 = Issue {
            key: "X-2".to_string(),
            summary: "S2".to_string(),
            status: "Open".to_string(),
            issue_type: "T".to_string(),
            assignee: Some("User".to_string()),
            description: None,
            updated: "U".to_string(),
        };
        let issues = vec![&issue1, &issue2];
        let output = format_status_section("Open", &issues);
        assert!(output.contains("Open"));
        assert!(output.contains("(2)"));
        assert!(output.contains("X-1"));
        assert!(output.contains("X-2"));
    }

    #[test]
    fn format_status_section_color_codes() {
        let empty: Vec<&Issue> = vec![];
        let done_output = format_status_section("Done", &empty);
        assert!(done_output.contains("\x1b[32m")); // green

        let progress_output = format_status_section("In Progress", &empty);
        assert!(progress_output.contains("\x1b[33m")); // yellow

        let other_output = format_status_section("Other", &empty);
        assert!(other_output.contains("\x1b[34m")); // blue
    }

    use super::super::types::{IssueUpdate, Transition, User};

    // Mock client for testing process_sprint
    struct MockJiraClient {
        issues: Vec<Issue>,
    }

    impl JiraApi for MockJiraClient {
        async fn get_current_user(&self) -> Result<User> {
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
    async fn process_sprint_returns_issues() {
        let client = MockJiraClient {
            issues: vec![Issue {
                key: "TEST-1".to_string(),
                summary: "Test issue".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Dev".to_string()),
                description: None,
                updated: "2024-01-01".to_string(),
            }],
        };

        let output = process_sprint(&client).await.unwrap();
        assert!(output.contains("TEST-1"));
        assert!(output.contains("Test issue"));
        assert!(output.contains("In Progress"));
    }

    #[tokio::test]
    async fn process_sprint_handles_empty() {
        let client = MockJiraClient { issues: vec![] };

        let output = process_sprint(&client).await.unwrap();
        assert!(output.contains("No issues in active sprints"));
    }

    #[test]
    fn format_sprint_output_handles_unknown_status() {
        // Test that unknown statuses (not in status_order) are still displayed
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "Task with custom status".to_string(),
                status: "Custom Status".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Alice".to_string()),
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Task with another status".to_string(),
                status: "Another Custom".to_string(),
                issue_type: "Task".to_string(),
                assignee: None,
                description: None,
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
        ];
        let output = format_sprint_output(&issues);
        assert!(output.contains("Custom Status"));
        assert!(output.contains("A-1"));
        assert!(output.contains("Another Custom"));
        assert!(output.contains("A-2"));
    }
}
