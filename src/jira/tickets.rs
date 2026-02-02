use anyhow::Result;

use super::client::{JiraApi, JiraClient};
use super::types::Issue;

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const GRAY: &str = "\x1b[90m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Run the jira tickets command (list current sprint tickets assigned to me)
pub async fn run() -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_tickets(&client).await?;
    print!("{}", output);
    Ok(())
}

/// Process tickets command (business logic, testable)
pub async fn process_tickets(client: &impl JiraApi) -> Result<String> {
    // Use JQL to find issues in active sprints assigned to current user
    let jql =
        "sprint in openSprints() AND assignee = currentUser() ORDER BY status ASC, updated DESC";
    let issues = client.search_issues(jql).await?;

    Ok(format_tickets(&issues))
}

fn get_terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120)
}

/// Format tickets as a table
fn format_tickets(issues: &[Issue]) -> String {
    let mut output = String::new();
    let term_width = get_terminal_width();

    // Header
    output.push_str(&format!(
        "{}My Sprint Tickets{} ({} issues)\n\n",
        BOLD,
        RESET,
        issues.len()
    ));

    if issues.is_empty() {
        output.push_str("No tickets assigned to you in active sprints\n");
        return output;
    }

    // Calculate column widths based on content
    let key_width = issues
        .iter()
        .map(|i| i.key.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let status_width = issues
        .iter()
        .map(|i| i.status.chars().count())
        .max()
        .unwrap_or(6)
        .max(6);
    let type_width = issues
        .iter()
        .map(|i| i.issue_type.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);

    // Layout: │ Key │ Status │ Type │ Summary │
    // Borders take: 5 separators × 3 chars = 15 chars
    let border_overhead = 15;
    let fixed_cols = key_width + status_width + type_width;
    let available_for_summary = term_width
        .saturating_sub(border_overhead + fixed_cols)
        .max(20);

    // Top border
    output.push_str(&format!(
        "┌{}┬{}┬{}┬{}┐\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    // Header row
    output.push_str(&format!(
        "│ {}{:<key_w$}{} │ {}{:<status_w$}{} │ {}{:<type_w$}{} │ {}{:<sum_w$}{} │\n",
        BOLD,
        "Key",
        RESET,
        BOLD,
        "Status",
        RESET,
        BOLD,
        "Type",
        RESET,
        BOLD,
        "Summary",
        RESET,
        key_w = key_width,
        status_w = status_width,
        type_w = type_width,
        sum_w = available_for_summary,
    ));

    // Header separator
    output.push_str(&format!(
        "├{}┼{}┼{}┼{}┤\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    // Data rows
    for issue in issues {
        let status_color = match issue.status.as_str() {
            "Done" => GREEN,
            "In Progress" | "In Review" | "CODE REVIEW" => YELLOW,
            _ => BLUE,
        };

        let summary_display = truncate(&issue.summary, available_for_summary);

        output.push_str(&format!(
            "│ {:<key_w$} │ {}{:<status_w$}{} │ {}{:<type_w$}{} │ {:<sum_w$} │\n",
            issue.key,
            status_color,
            truncate(&issue.status, status_width),
            RESET,
            GRAY,
            truncate(&issue.issue_type, type_width),
            RESET,
            summary_display,
            key_w = key_width,
            status_w = status_width,
            type_w = type_width,
            sum_w = available_for_summary,
        ));
    }

    // Bottom border
    output.push_str(&format!(
        "└{}┴{}┴{}┴{}┘\n",
        "─".repeat(key_width + 2),
        "─".repeat(status_width + 2),
        "─".repeat(type_width + 2),
        "─".repeat(available_for_summary + 2)
    ));

    output
}

/// Truncate string to max length with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{IssueUpdate, Transition, User};
    use super::*;

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_adds_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }

    #[test]
    fn truncate_unicode() {
        assert_eq!(truncate("héllo", 5), "héllo");
        assert_eq!(truncate("héllo world", 6), "héllo…");
    }

    #[test]
    fn truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn get_terminal_width_returns_reasonable_value() {
        let width = get_terminal_width();
        assert!(width >= 20);
    }

    #[test]
    fn format_tickets_empty() {
        let issues: Vec<Issue> = vec![];
        let output = format_tickets(&issues);
        assert!(output.contains("My Sprint Tickets"));
        assert!(output.contains("0 issues"));
        assert!(output.contains("No tickets assigned"));
    }

    #[test]
    fn format_tickets_with_issues() {
        let issues = vec![
            Issue {
                key: "A-1".to_string(),
                summary: "First task".to_string(),
                status: "Done".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Alice".to_string()),
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "A-2".to_string(),
                summary: "Second task".to_string(),
                status: "In Progress".to_string(),
                issue_type: "Bug".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_tickets(&issues);
        assert!(output.contains("My Sprint Tickets"));
        assert!(output.contains("2 issues"));
        assert!(output.contains("A-1"));
        assert!(output.contains("A-2"));
        assert!(output.contains("First task"));
        assert!(output.contains("Second task"));
        assert!(output.contains("Task"));
        assert!(output.contains("Bug"));
        // Box-drawing characters
        assert!(output.contains("┌"));
        assert!(output.contains("┐"));
        assert!(output.contains("└"));
        assert!(output.contains("┘"));
        assert!(output.contains("│"));
        assert!(output.contains("─"));
    }

    #[test]
    fn format_tickets_colors_status() {
        let issues = vec![
            Issue {
                key: "X-1".to_string(),
                summary: "S".to_string(),
                status: "Done".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "X-2".to_string(),
                summary: "S".to_string(),
                status: "In Progress".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
            Issue {
                key: "X-3".to_string(),
                summary: "S".to_string(),
                status: "To Do".to_string(),
                issue_type: "T".to_string(),
                assignee: None,
                description: None,
                updated: "U".to_string(),
            },
        ];
        let output = format_tickets(&issues);
        assert!(output.contains(GREEN)); // Done
        assert!(output.contains(YELLOW)); // In Progress
        assert!(output.contains(BLUE)); // To Do
    }

    #[test]
    fn format_tickets_handles_long_summary() {
        let issues = vec![Issue {
            key: "LONG-123".to_string(),
            summary: "This is a very long summary that should be truncated to fit within the terminal width appropriately".to_string(),
            status: "Open".to_string(),
            issue_type: "Story".to_string(),
            assignee: Some("A Very Long Username".to_string()),
            description: None,
            updated: "U".to_string(),
        }];
        let output = format_tickets(&issues);
        // Should contain truncation indicator
        assert!(output.contains("…"));
    }

    // Mock client for testing
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
    async fn process_tickets_returns_issues() {
        let client = MockJiraClient {
            issues: vec![Issue {
                key: "TEST-1".to_string(),
                summary: "Test issue".to_string(),
                status: "Open".to_string(),
                issue_type: "Task".to_string(),
                assignee: Some("Me".to_string()),
                description: None,
                updated: "2024-01-01".to_string(),
            }],
        };

        let output = process_tickets(&client).await.unwrap();
        assert!(output.contains("TEST-1"));
        assert!(output.contains("Test issue"));
    }

    #[tokio::test]
    async fn process_tickets_handles_empty() {
        let client = MockJiraClient { issues: vec![] };

        let output = process_tickets(&client).await.unwrap();
        assert!(output.contains("No tickets assigned"));
    }
}
