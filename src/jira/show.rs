use anyhow::Result;

use super::client::{JiraApi, JiraClient};
use super::types::Issue;

/// Run the jira show command
pub async fn run(key: &str) -> Result<()> {
    let client = JiraClient::new().await?;
    let output = process_show(&client, key).await?;
    print!("{}", output);
    Ok(())
}

/// Process show command (business logic, testable)
pub async fn process_show(client: &impl JiraApi, key: &str) -> Result<String> {
    let issue = client.get_issue(key).await?;
    Ok(format_issue(&issue))
}

/// Format issue for display
fn format_issue(issue: &Issue) -> String {
    let mut output = String::new();

    // Header
    output.push_str(&format!("\x1b[1m{}\x1b[0m {}\n", issue.key, issue.summary));
    output.push('\n');

    // Metadata
    output.push_str(&format!("Type:     {}\n", issue.issue_type));
    output.push_str(&format!("Status:   {}\n", format_status(&issue.status)));
    output.push_str(&format!(
        "Assignee: {}\n",
        issue.assignee.as_deref().unwrap_or("Unassigned")
    ));
    output.push_str(&format!("Updated:  {}\n", format_date(&issue.updated)));

    // Description
    if let Some(desc) = &issue.description {
        output.push('\n');
        output.push_str("Description:\n");
        output.push_str(&format_description(desc));
    }

    output
}

/// Format status with color
fn format_status(status: &str) -> String {
    let color = match status {
        "Done" => "\x1b[32m",        // green
        "In Progress" => "\x1b[33m", // yellow
        "To Do" => "\x1b[34m",       // blue
        "In Review" => "\x1b[35m",   // magenta
        _ => "\x1b[36m",             // cyan
    };
    format!("{}{}\x1b[0m", color, status)
}

/// Format date for display
fn format_date(date: &str) -> String {
    // Parse ISO date and format nicely
    // Input: "2024-01-15T10:30:00.000+0000"
    if let Some((date_part, time_part)) = date.split_once('T') {
        if let Some((time, _)) = time_part.split_once('.') {
            return format!("{} {}", date_part, time);
        }
        return format!(
            "{} {}",
            date_part,
            time_part.split('+').next().unwrap_or(time_part)
        );
    }
    date.to_string()
}

/// Format description with indentation
fn format_description(desc: &str) -> String {
    desc.lines().map(|line| format!("  {}\n", line)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_issue_shows_key_and_summary() {
        let issue = Issue {
            key: "PROJ-123".to_string(),
            summary: "Fix the bug".to_string(),
            status: "In Progress".to_string(),
            issue_type: "Bug".to_string(),
            assignee: Some("John".to_string()),
            description: None,
            updated: "2024-01-15T10:30:00.000+0000".to_string(),
        };
        let output = format_issue(&issue);
        assert!(output.contains("PROJ-123"));
        assert!(output.contains("Fix the bug"));
        assert!(output.contains("Bug"));
        assert!(output.contains("In Progress"));
        assert!(output.contains("John"));
    }

    #[test]
    fn format_issue_shows_unassigned() {
        let issue = Issue {
            key: "X-1".to_string(),
            summary: "S".to_string(),
            status: "Open".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            description: None,
            updated: "2024-01-01T00:00:00Z".to_string(),
        };
        let output = format_issue(&issue);
        assert!(output.contains("Unassigned"));
    }

    #[test]
    fn format_issue_shows_description() {
        let issue = Issue {
            key: "X-1".to_string(),
            summary: "S".to_string(),
            status: "Open".to_string(),
            issue_type: "Task".to_string(),
            assignee: None,
            description: Some("This is the description.\nWith multiple lines.".to_string()),
            updated: "2024-01-01T00:00:00Z".to_string(),
        };
        let output = format_issue(&issue);
        assert!(output.contains("Description:"));
        assert!(output.contains("This is the description."));
        assert!(output.contains("With multiple lines."));
    }

    #[test]
    fn format_status_colors_done() {
        let output = format_status("Done");
        assert!(output.contains("\x1b[32m")); // green
        assert!(output.contains("Done"));
    }

    #[test]
    fn format_status_colors_in_progress() {
        let output = format_status("In Progress");
        assert!(output.contains("\x1b[33m")); // yellow
    }

    #[test]
    fn format_status_colors_to_do() {
        let output = format_status("To Do");
        assert!(output.contains("\x1b[34m")); // blue
    }

    #[test]
    fn format_status_colors_in_review() {
        let output = format_status("In Review");
        assert!(output.contains("\x1b[35m")); // magenta
    }

    #[test]
    fn format_status_colors_other() {
        let output = format_status("Unknown Status");
        assert!(output.contains("\x1b[36m")); // cyan
    }

    #[test]
    fn format_date_parses_full_iso() {
        let date = "2024-01-15T10:30:00.000+0000";
        let output = format_date(date);
        assert_eq!(output, "2024-01-15 10:30:00");
    }

    #[test]
    fn format_date_parses_iso_with_z() {
        let date = "2024-01-15T10:30:00Z";
        let output = format_date(date);
        assert_eq!(output, "2024-01-15 10:30:00Z");
    }

    #[test]
    fn format_date_handles_simple() {
        let date = "2024-01-15";
        let output = format_date(date);
        assert_eq!(output, "2024-01-15");
    }

    #[test]
    fn format_description_indents_lines() {
        let desc = "Line 1\nLine 2\nLine 3";
        let output = format_description(desc);
        assert!(output.contains("  Line 1\n"));
        assert!(output.contains("  Line 2\n"));
        assert!(output.contains("  Line 3\n"));
    }

    #[test]
    fn format_description_handles_empty() {
        let output = format_description("");
        // Empty string produces empty output (no lines to format)
        assert_eq!(output, "");
    }

    #[test]
    fn format_description_handles_single_line() {
        let output = format_description("Only one line");
        assert_eq!(output, "  Only one line\n");
    }

    use super::super::types::{Board, IssueUpdate, Sprint, Transition, User};

    // Mock client for testing process_show
    struct MockJiraClient {
        issue: Issue,
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
            Ok(self.issue.clone())
        }

        async fn search_issues(&self, _jql: &str) -> Result<Vec<Issue>> {
            unimplemented!()
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
    async fn process_show_returns_formatted_issue() {
        let client = MockJiraClient {
            issue: Issue {
                key: "TEST-999".to_string(),
                summary: "Test issue".to_string(),
                status: "Done".to_string(),
                issue_type: "Story".to_string(),
                assignee: Some("Tester".to_string()),
                description: Some("Test description".to_string()),
                updated: "2024-01-01T00:00:00Z".to_string(),
            },
        };

        let output = process_show(&client, "TEST-999").await.unwrap();
        assert!(output.contains("TEST-999"));
        assert!(output.contains("Test issue"));
        assert!(output.contains("Done"));
        assert!(output.contains("Story"));
        assert!(output.contains("Tester"));
        assert!(output.contains("Test description"));
    }
}
